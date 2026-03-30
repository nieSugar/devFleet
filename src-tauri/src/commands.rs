// 这个文件是 Tauri 的「命令层」，所有 #[tauri::command] 函数都是前端可调用的 IPC 接口
// 前端调用方式：await invoke('函数名', { 参数名: 值 })
// Tauri 会自动把 JS 参数反序列化为 Rust 类型，把 Rust 返回值序列化为 JSON 发回前端

// crate:: 前缀表示从当前 crate（项目）的其他模块导入
use crate::config;
use crate::detector;
use crate::models::{IpcResponse, NodeVersionManager, PackageManager, ProjectConfig};
use crate::project;
use std::process::Command;
use std::sync::{mpsc, OnceLock};

fn reaper_tx() -> &'static mpsc::Sender<std::process::Child> {
    static TX: OnceLock<mpsc::Sender<std::process::Child>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<std::process::Child>();
        std::thread::spawn(move || {
            for mut child in rx {
                let _ = child.wait();
            }
        });
        tx
    })
}

/// 启动子进程并通过单一 reaper 线程回收退出状态，避免 Unix 上产生僵尸进程
fn spawn_and_detach(cmd: &mut Command) -> bool {
    match cmd.spawn() {
        Ok(child) => reaper_tx().send(child).is_ok(),
        Err(_) => false,
    }
}

/// 校验脚本名称是否合法，防止命令注入攻击
fn validate_script_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        // .chars() 返回字符迭代器，.all() 检查是否所有字符都满足条件
        // 只允许字母数字和 -_:. 四种符号
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || "-_:.".contains(c))
}

// ── 项目管理命令 ──

// #[tauri::command] 宏：把普通函数变成 Tauri IPC 命令
// 它会自动处理：参数的 JSON 反序列化、返回值的 JSON 序列化、错误处理
// 函数参数名就是前端 invoke 时传的参数名（自动 camelCase → snake_case 转换）
#[tauri::command]
pub fn get_package_scripts(project_path: String) -> IpcResponse {
    if !project::is_valid_path(&project_path) {
        return IpcResponse::err("无效的项目路径");
    }
    let scripts = project::get_package_scripts(&project_path);
    let pm = detector::detect_package_manager(&project_path);
    // json!() 宏创建 serde_json::Value，支持类似 JS 的对象字面量语法
    IpcResponse::ok(serde_json::json!({
        "scripts": scripts,
        "packageManager": pm.to_string(),
    }))
}

#[tauri::command]
pub fn detect_package_manager(project_path: String) -> IpcResponse {
    let pm = detector::detect_package_manager(&project_path);
    IpcResponse::ok(serde_json::json!({ "packageManager": pm.to_string() }))
}

#[tauri::command]
pub fn load_project_config() -> IpcResponse {
    IpcResponse::ok(config::load())
}

#[tauri::command]
pub async fn refresh_project_config() -> IpcResponse {
    tokio::task::spawn_blocking(|| {
        let mut cfg = config::load_and_refresh();

        let indices: Vec<usize> = cfg
            .projects
            .iter()
            .enumerate()
            .filter(|(_, p)| p.package_manager.is_none())
            .map(|(i, _)| i)
            .collect();

        if !indices.is_empty() {
            let paths: Vec<String> = indices
                .iter()
                .map(|&i| cfg.projects[i].path.clone())
                .collect();
            let results: Vec<String> = std::thread::scope(|s| {
                let handles: Vec<_> = paths
                    .iter()
                    .map(|path| s.spawn(|| detector::detect_package_manager(path).to_string()))
                    .collect();
                handles
                    .into_iter()
                    .map(|h| {
                        h.join().unwrap_or_else(|e| {
                            eprintln!(
                                "[devfleet] package manager detection thread panicked: {:?}",
                                e
                            );
                            "npm".to_string()
                        })
                    })
                    .collect()
            });
            for (idx, pm) in indices.into_iter().zip(results) {
                cfg.projects[idx].package_manager = Some(pm);
            }
        }

        config::save(&cfg);
        IpcResponse::ok(cfg)
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

#[tauri::command]
pub fn save_project_config(config: ProjectConfig) -> IpcResponse {
    if config::save(&config) {
        IpcResponse::ok_msg("配置保存成功")
    } else {
        IpcResponse::err("保存配置失败")
    }
}

#[tauri::command]
pub fn add_project_to_config(project_path: String) -> IpcResponse {
    if !project::is_valid_path(&project_path) {
        return IpcResponse::err("所选文件夹不是有效的项目目录（缺少 package.json）");
    }
    match project::add_to_config(&project_path) {
        Ok(p) => IpcResponse::ok(p),
        Err(true) => IpcResponse::err("该项目路径已存在，请勿重复添加"),
        Err(false) => IpcResponse::err("添加项目失败"),
    }
}

#[tauri::command]
pub fn remove_project_from_config(project_id: String) -> IpcResponse {
    if project::remove_from_config(&project_id) {
        IpcResponse::ok_msg("项目删除成功")
    } else {
        IpcResponse::err("删除项目失败")
    }
}

// ── 外部终端启动（平台特定实现） ──
// #[cfg(...)] 是条件编译：只在指定平台上编译这段代码，其他平台完全忽略
// 这样同一个函数名在不同平台有不同实现，调用方不用关心平台差异

/// Windows: 用 cmd /K 在新控制台窗口中运行命令
#[cfg(target_os = "windows")]
fn spawn_external_terminal(project_path: &str, run_command: &str) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x00000010;
    spawn_and_detach(
        Command::new("cmd")
            .raw_arg(format!("/K {}", run_command))
            .current_dir(project_path)
            .creation_flags(CREATE_NEW_CONSOLE),
    )
}

/// macOS: 用 AppleScript 控制 Terminal.app 打开新窗口
#[cfg(target_os = "macos")]
fn spawn_external_terminal(project_path: &str, run_command: &str) -> bool {
    let safe_path = project_path.replace('\'', "'\\''");
    let safe_cmd = run_command.replace('\\', "\\\\").replace('"', "\\\"");
    let osa = format!(
        r#"tell application "Terminal"
  activate
  do script "cd '{path}' && {cmd}"
end tell"#,
        path = safe_path,
        cmd = safe_cmd,
    );
    spawn_and_detach(Command::new("osascript").args(["-e", &osa]))
}

/// Linux: 依次尝试 gnome-terminal → konsole → xterm，用第一个可用的
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn spawn_external_terminal(project_path: &str, run_command: &str) -> bool {
    let safe_path = project_path.replace('\'', "'\\''");
    let full_cmd = format!("cd '{}' && {}; exec bash", safe_path, run_command);
    let terminals: Vec<(&str, Vec<&str>)> = vec![
        ("gnome-terminal", vec!["--", "bash", "-lc", &full_cmd]),
        ("konsole", vec!["-e", "bash", "-lc", &full_cmd]),
        ("xterm", vec!["-e", "bash", "-lc", &full_cmd]),
    ];
    // .any() 遍历尝试，只要有一个成功就返回 true
    terminals.iter().any(|(cmd, args)| {
        spawn_and_detach(
            Command::new(cmd)
                .args(args.clone())
                .current_dir(project_path),
        )
    })
}

// ── 脚本执行命令 ──

/// 构建 PATH 注入命令，将指定版本的 Node 二进制目录插入 PATH 最前面
/// 比 `nvm use` 更好：不修改全局状态，多终端可同时使用不同版本
fn build_node_path_prefix(version: &str, manager: &NodeVersionManager) -> Option<String> {
    let dir = detector::get_node_bin_dir(version, manager)?;
    format_path_prefix(&dir)
}

/// 构建 builtin 管理器 current 目录的 PATH 注入命令，
/// 用于项目未指定 Node 版本时保证 node/npm/pnpm 等全局工具可用
fn build_builtin_current_path_prefix() -> Option<String> {
    let dir = crate::node_manager::get_current_bin_path()?;
    format_path_prefix(&dir)
}

fn format_path_prefix(dir: &std::path::Path) -> Option<String> {
    let dir_str = dir.to_string_lossy();
    if cfg!(target_os = "windows") {
        Some(format!(r#"set "PATH={};%PATH%""#, dir_str))
    } else {
        Some(format!("export PATH={}:$PATH", dir_str))
    }
}

#[tauri::command]
pub fn run_script(
    project_path: String,
    script_name: String,
    _project_id: String,
    package_manager: Option<String>,
    node_version: Option<String>,
) -> IpcResponse {
    if !validate_script_name(&script_name) {
        return IpcResponse::err(
            "脚本名称包含非法字符，仅允许字母、数字、连字符、下划线、冒号和点",
        );
    }

    let pm = package_manager
        .and_then(|s| s.parse::<PackageManager>().ok())
        .unwrap_or_else(|| detector::detect_package_manager(&project_path));

    let base_command = pm.run_command(&script_name);

    // 项目指定了 Node 版本 → 注入该版本的 bin 目录
    // 未指定 → fallback 注入 builtin current 目录，确保全局安装的 pnpm/yarn 等可用
    let run_command = match node_version.as_deref().filter(|v| !v.trim().is_empty()) {
        Some(ver) => {
            let manager = detector::detect_node_version_manager();
            match build_node_path_prefix(ver, &manager) {
                Some(prefix) => format!("{} && {}", prefix, base_command),
                None => base_command,
            }
        }
        None => match build_builtin_current_path_prefix() {
            Some(prefix) => format!("{} && {}", prefix, base_command),
            None => base_command,
        },
    };

    if !spawn_external_terminal(&project_path, &run_command) {
        return IpcResponse::err("启动外部终端失败，无法找到可用的终端程序");
    }

    IpcResponse::ok(serde_json::json!({
        "message": "已在外部终端启动",
        "command": run_command,
        "packageManager": pm.to_string(),
        "nodeVersion": node_version,
    }))
}

// ── 编辑器命令 ──

/// 检测系统中安装了哪些代码编辑器（带缓存，force=true 时强制重新检测）
#[tauri::command]
pub async fn detect_editors(force: Option<bool>) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        if force != Some(true) {
            if let Some(cached) = config::load_editor_cache() {
                return IpcResponse::ok(cached);
            }
        }

        let (vscode, cursor, webstorm) = detector::detect_editors();
        let cache = crate::models::EditorCache {
            vscode,
            cursor,
            webstorm,
        };
        config::save_editor_cache(&cache);
        IpcResponse::ok(cache)
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 用指定编辑器打开项目
#[tauri::command]
pub fn open_in_editor(editor: String, project_path: String) -> IpcResponse {
    if detector::open_editor(&editor, &project_path) {
        IpcResponse::ok(serde_json::json!({ "message": "已打开编辑器" }))
    } else {
        IpcResponse::err("未找到对应编辑器或命令不可用")
    }
}

// ── Node 版本管理命令 ──

/// 获取系统的 Node 版本管理器信息（nvm/nvmd/nvs 及已安装的版本列表）
#[tauri::command]
pub async fn get_nvm_info() -> IpcResponse {
    tokio::task::spawn_blocking(|| IpcResponse::ok(detector::get_nvm_info()))
        .await
        .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 检测项目指定的 Node 版本（从 .nvmrc/.node-version 等文件读取）
#[tauri::command]
pub fn detect_project_node_version(project_path: String) -> IpcResponse {
    let version = project::get_node_version(&project_path);
    IpcResponse::ok(serde_json::json!({ "version": version }))
}

/// 获取所有远程可用的 Node.js 版本列表（自动读取镜像配置）
#[tauri::command]
pub async fn fetch_remote_node_versions() -> IpcResponse {
    tokio::task::spawn_blocking(|| {
        let mirror = config::load_node_mirror();
        match detector::fetch_remote_node_versions(mirror.as_deref()) {
            Ok(versions) => IpcResponse::ok(versions),
            Err(e) => IpcResponse::err(e),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 获取当前配置的 Node 镜像地址（空字符串表示官方源）
#[tauri::command]
pub fn get_node_mirror() -> IpcResponse {
    let mirror = config::load_node_mirror().unwrap_or_default();
    IpcResponse::ok(serde_json::json!({ "mirror": mirror }))
}

/// 设置 Node 镜像地址（传空字符串则恢复官方源）
#[tauri::command]
pub fn set_node_mirror(mirror: String) -> IpcResponse {
    let value = if mirror.trim().is_empty() {
        None
    } else {
        Some(mirror.trim())
    };
    config::save_node_mirror(value);
    IpcResponse::ok_msg("镜像地址已更新")
}

fn resolve_manager(manager: Option<String>) -> NodeVersionManager {
    if let Some(m) = manager {
        match m.as_str() {
            "builtin" => NodeVersionManager::Builtin,
            "nvmd" => NodeVersionManager::Nvmd,
            "nvs" => NodeVersionManager::Nvs,
            "nvm" => NodeVersionManager::Nvm,
            "nvm-windows" => NodeVersionManager::NvmWindows,
            _ => detector::detect_node_version_manager(),
        }
    } else {
        detector::detect_node_version_manager()
    }
}

/// resolve_manager 的变体：当检测结果为 None 时自动 fallback 到 Builtin
fn resolve_manager_or_builtin(manager: Option<String>) -> NodeVersionManager {
    let mgr = resolve_manager(manager);
    if mgr == NodeVersionManager::None {
        NodeVersionManager::Builtin
    } else {
        mgr
    }
}

/// 通过版本管理器安装指定版本的 Node.js
#[tauri::command]
pub async fn install_node_version(version: String, manager: Option<String>) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        let mgr = resolve_manager_or_builtin(manager);

        match detector::install_node_version(&version, &mgr) {
            Ok(output) => IpcResponse::ok(serde_json::json!({
                "message": format!("Node.js {} 安装成功", version),
                "output": output,
            })),
            Err(e) => IpcResponse::err(e),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 切换系统当前使用的 Node.js 版本
#[tauri::command]
pub async fn switch_node_version(version: String, manager: Option<String>) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        let mgr = resolve_manager_or_builtin(manager);

        match detector::switch_node_version(&version, &mgr) {
            Ok(output) => IpcResponse::ok(serde_json::json!({
                "message": format!("已切换到 Node.js {}", version),
                "output": output,
            })),
            Err(e) => IpcResponse::err(e),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 通过版本管理器卸载指定已安装版本的 Node.js
#[tauri::command]
pub async fn uninstall_node_version(version: String, manager: Option<String>) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        let mgr = resolve_manager_or_builtin(manager);

        match detector::uninstall_node_version(&version, &mgr) {
            Ok(output) => IpcResponse::ok(serde_json::json!({
                "message": format!("Node.js {} 已卸载", version),
                "output": output,
            })),
            Err(e) => IpcResponse::err(e),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 获取 Node 安装目录（空字符串表示默认路径）
#[tauri::command]
pub fn get_node_install_dir() -> IpcResponse {
    let dir = crate::node_manager::get_install_dir();
    let custom = config::load_node_install_dir().unwrap_or_default();
    IpcResponse::ok(serde_json::json!({
        "dir": dir.to_string_lossy(),
        "custom": custom,
    }))
}

/// 将 builtin 管理器的 current 目录添加到系统 PATH
#[tauri::command]
pub async fn setup_node_global_path() -> IpcResponse {
    tokio::task::spawn_blocking(|| {
        match crate::node_manager::add_to_system_path() {
            Ok(msg) => IpcResponse::ok(serde_json::json!({ "message": msg })),
            Err(e) => IpcResponse::err(e),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 检查 builtin Node 是否已在系统 PATH 中，
/// 同时检测系统中 node 命令是否可用（避免用户已有外部 node 时误报）
#[tauri::command]
pub fn check_node_in_path() -> IpcResponse {
    let bin_path = crate::node_manager::get_current_bin_path();
    let in_path = bin_path.as_ref().map_or(false, |p| {
        let s = p.to_string_lossy();
        crate::node_manager::is_path_configured(&s)
    });
    let node_available = if in_path {
        true
    } else {
        detector::get_current_node_version().is_some()
    };
    IpcResponse::ok(serde_json::json!({
        "inPath": in_path,
        "binPath": bin_path.map(|p| p.to_string_lossy().to_string()),
        "nodeAvailable": node_available,
    }))
}

/// 设置 Node 安装目录（传空字符串恢复默认）
#[tauri::command]
pub fn set_node_install_dir(dir: String) -> IpcResponse {
    let value = if dir.trim().is_empty() {
        None
    } else {
        let p = std::path::Path::new(dir.trim());
        if !p.exists() {
            if let Err(e) = std::fs::create_dir_all(p) {
                return IpcResponse::err(format!("创建目录失败: {}", e));
            }
        }
        if !p.is_dir() {
            return IpcResponse::err("指定路径不是有效目录");
        }
        Some(dir.trim())
    };
    config::save_node_install_dir(value);
    IpcResponse::ok_msg("安装目录已更新")
}

/// 设置项目的 Node 版本（写入对应的版本文件，如 .nvmrc）
#[tauri::command]
pub fn set_project_node_version(project_id: String, node_version: Option<String>) -> IpcResponse {
    let mut cfg = config::load();
    // .position() 返回第一个满足条件的元素的索引，类似 JS 的 findIndex()
    let proj_idx = match cfg.projects.iter().position(|p| p.id == project_id) {
        Some(i) => i,
        None => return IpcResponse::err("项目不存在"),
    };

    let mut manager = detector::detect_node_version_manager();
    if manager == NodeVersionManager::None {
        manager = NodeVersionManager::Builtin;
    }

    // .as_deref() 把 Option<String> 转为 Option<&str>
    // 这是 Rust 所有权系统的常见操作：String 是拥有所有权的，&str 是借用的
    let nv = node_version.as_deref();
    if !project::set_node_version_file(&cfg.projects[proj_idx].path, nv, &manager) {
        return IpcResponse::err("操作版本配置文件失败");
    }

    // .filter() 在 Option 上使用：满足条件保持 Some，不满足变 None
    // .cloned() 把 Option<&String> 转成 Option<String>（深拷贝）
    cfg.projects[proj_idx].node_version = node_version
        .as_ref()
        .filter(|v| !v.trim().is_empty())
        .cloned();

    let updated_proj = cfg.projects[proj_idx].clone();

    if !config::save(&cfg) {
        return IpcResponse::err("保存配置失败");
    }

    let file_name = match manager {
        NodeVersionManager::Builtin => ".node-version",
        NodeVersionManager::Nvmd => ".nvmdrc",
        NodeVersionManager::Nvs => ".node-version",
        _ => ".nvmrc",
    };

    let message = match nv {
        // 模式守卫（pattern guard）：Some(v) if 条件 → 匹配 Some 且额外满足条件
        Some(v) if !v.trim().is_empty() => {
            format!("已创建 {} 文件并设置 Node 版本为 {}", file_name, v)
        }
        _ => format!("已删除 {} 文件", file_name),
    };

    IpcResponse::ok(serde_json::json!({
        "message": message,
        "project": updated_proj,
    }))
}
