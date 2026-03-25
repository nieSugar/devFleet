// 这个文件是 Tauri 的「命令层」，所有 #[tauri::command] 函数都是前端可调用的 IPC 接口
// 前端调用方式：await invoke('函数名', { 参数名: 值 })
// Tauri 会自动把 JS 参数反序列化为 Rust 类型，把 Rust 返回值序列化为 JSON 发回前端

// crate:: 前缀表示从当前 crate（项目）的其他模块导入
use crate::config;
use crate::detector;
use crate::models::{AppState, IpcResponse, NodeVersionManager, PackageManager, ProjectConfig};
use crate::project;
use std::process::Command;
// State 是 Tauri 的依赖注入类型
// 在 command 函数参数里写 State<AppState>，Tauri 会自动把 lib.rs 中 manage() 注册的状态注入进来
use tauri::State;

// const 定义编译时常量，&str 是字符串切片（不拥有数据的只读引用）
const EXTERNAL_UNTRACKABLE: &str =
    "外部终端模式下无法追踪脚本进程，请在终端窗口手动停止。";

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
pub fn refresh_project_config() -> IpcResponse {
    let mut cfg = config::load_and_refresh();

    let indices: Vec<usize> = cfg
        .projects
        .iter()
        .enumerate()
        .filter(|(_, p)| p.package_manager.is_none())
        .map(|(i, _)| i)
        .collect();

    if !indices.is_empty() {
        let paths: Vec<String> = indices.iter().map(|&i| cfg.projects[i].path.clone()).collect();
        let results: Vec<String> = std::thread::scope(|s| {
            let handles: Vec<_> = paths
                .iter()
                .map(|path| s.spawn(|| detector::detect_package_manager(path).to_string()))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|_| "npm".to_string()))
                .collect()
        });
        for (idx, pm) in indices.into_iter().zip(results) {
            cfg.projects[idx].package_manager = Some(pm);
        }
    }

    config::save(&cfg);
    IpcResponse::ok(cfg)
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
    // match 解构 Option：Some(p) 取出值，None 走错误分支
    match project::add_to_config(&project_path) {
        Some(p) => IpcResponse::ok(p),
        None => IpcResponse::err("添加项目失败"),
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
    Command::new("cmd")
        .raw_arg(format!("/K {}", run_command))
        .current_dir(project_path)
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()
        .is_ok()
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
    Command::new("osascript").args(["-e", &osa]).spawn().is_ok()
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
        Command::new(cmd)
            .args(args.clone())
            .current_dir(project_path)
            .spawn()
            .is_ok()
    })
}

// ── 脚本执行命令 ──

/// 构建 PATH 注入命令，将指定版本的 Node 二进制目录插入 PATH 最前面
/// 比 `nvm use` 更好：不修改全局状态，多终端可同时使用不同版本
fn build_node_path_prefix(version: &str, manager: &NodeVersionManager) -> Option<String> {
    let dir = detector::get_node_bin_dir(version, manager)?;
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
        return IpcResponse::err("脚本名称包含非法字符，仅允许字母、数字、连字符、下划线、冒号和点");
    }

    let pm = package_manager
        .and_then(|s| s.parse::<PackageManager>().ok())
        .unwrap_or_else(|| detector::detect_package_manager(&project_path));

    let base_command = pm.run_command(&script_name);

    // 如果项目指定了 Node 版本，在实际命令前注入版本切换命令
    let run_command = match node_version.as_deref().filter(|v| !v.trim().is_empty()) {
        Some(ver) => {
            let manager = detector::detect_node_version_manager();
            match build_node_path_prefix(ver, &manager) {
                Some(prefix) => format!("{} && {}", prefix, base_command),
                None => base_command,
            }
        }
        None => base_command,
    };

    if !spawn_external_terminal(&project_path, &run_command) {
        return IpcResponse::err("启动外部终端失败，无法找到可用的终端程序");
    }

    IpcResponse::ok(serde_json::json!({
        "message": "已在外部终端启动",
        "command": run_command,
        "packageManager": pm.to_string(),
        "nodeVersion": node_version,
        "mode": "external",
        "trackable": false,
        "reason": EXTERNAL_UNTRACKABLE,
    }))
}

/// 停止正在运行的脚本进程
// State<'_, AppState> 中的 '_ 是生命周期标注，告诉编译器这个引用的有效期
// 这里用 '_ 让编译器自动推断，你不需要手动管理
#[tauri::command]
pub fn stop_script(_state: State<'_, AppState>, _project_id: String) -> IpcResponse {
    IpcResponse::err(EXTERNAL_UNTRACKABLE)
}

/// 检查脚本是否仍在运行
#[tauri::command]
pub fn check_script_status(_state: State<'_, AppState>, _project_id: String) -> IpcResponse {
    IpcResponse::ok(serde_json::json!({
        "isRunning": false,
        "mode": "external",
        "trackable": false,
        "reason": EXTERNAL_UNTRACKABLE,
    }))
}

/// 获取脚本的输出内容
#[tauri::command]
pub fn get_script_output(_state: State<'_, AppState>, _project_id: String) -> IpcResponse {
    IpcResponse::ok(serde_json::json!({ "output": "" }))
}

// ── 编辑器命令 ──

/// 检测系统中安装了哪些代码编辑器（带缓存，force=true 时强制重新检测）
#[tauri::command]
pub fn detect_editors(force: Option<bool>) -> IpcResponse {
    if force != Some(true) {
        if let Some(cached) = config::load_editor_cache() {
            return IpcResponse::ok(cached);
        }
    }

    let (vscode, cursor, webstorm) = detector::detect_editors();
    let cache = crate::models::EditorCache { vscode, cursor, webstorm };
    config::save_editor_cache(&cache);
    IpcResponse::ok(cache)
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
pub fn get_nvm_info() -> IpcResponse {
    IpcResponse::ok(detector::get_nvm_info())
}

/// 检测项目指定的 Node 版本（从 .nvmrc/.node-version 等文件读取）
#[tauri::command]
pub fn detect_project_node_version(project_path: String) -> IpcResponse {
    let version = project::get_node_version(&project_path);
    IpcResponse::ok(serde_json::json!({ "version": version }))
}

/// 从 nodejs.org 获取所有远程可用的 Node.js 版本列表
#[tauri::command]
pub fn fetch_remote_node_versions() -> IpcResponse {
    match detector::fetch_remote_node_versions() {
        Ok(versions) => IpcResponse::ok(versions),
        Err(e) => IpcResponse::err(e),
    }
}

fn resolve_manager(manager: Option<String>) -> NodeVersionManager {
    if let Some(m) = manager {
        match m.as_str() {
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

/// 通过版本管理器安装指定版本的 Node.js
#[tauri::command]
pub fn install_node_version(version: String, manager: Option<String>) -> IpcResponse {
    let mgr = resolve_manager(manager);

    if mgr == NodeVersionManager::None {
        return IpcResponse::err("未检测到 Node 版本管理器（nvmd/nvm/nvs）");
    }

    match detector::install_node_version(&version, &mgr) {
        Ok(output) => IpcResponse::ok(serde_json::json!({
            "message": format!("Node.js {} 安装成功", version),
            "output": output,
        })),
        Err(e) => IpcResponse::err(e),
    }
}

/// 切换系统当前使用的 Node.js 版本
#[tauri::command]
pub fn switch_node_version(version: String, manager: Option<String>) -> IpcResponse {
    let mgr = resolve_manager(manager);

    if mgr == NodeVersionManager::None {
        return IpcResponse::err("未检测到 Node 版本管理器（nvmd/nvm/nvs）");
    }

    match detector::switch_node_version(&version, &mgr) {
        Ok(output) => IpcResponse::ok(serde_json::json!({
            "message": format!("已切换到 Node.js {}", version),
            "output": output,
        })),
        Err(e) => IpcResponse::err(e),
    }
}

/// 通过版本管理器卸载指定已安装版本的 Node.js
#[tauri::command]
pub fn uninstall_node_version(version: String, manager: Option<String>) -> IpcResponse {
    let mgr = resolve_manager(manager);

    if mgr == NodeVersionManager::None {
        return IpcResponse::err("未检测到 Node 版本管理器（nvmd/nvm/nvs）");
    }

    match detector::uninstall_node_version(&version, &mgr) {
        Ok(output) => IpcResponse::ok(serde_json::json!({
            "message": format!("Node.js {} 已卸载", version),
            "output": output,
        })),
        Err(e) => IpcResponse::err(e),
    }
}

/// 设置项目的 Node 版本（写入对应的版本文件，如 .nvmrc）
#[tauri::command]
pub fn set_project_node_version(
    project_id: String,
    node_version: Option<String>,
) -> IpcResponse {
    let mut cfg = config::load();
    // .position() 返回第一个满足条件的元素的索引，类似 JS 的 findIndex()
    let proj_idx = match cfg.projects.iter().position(|p| p.id == project_id) {
        Some(i) => i,
        None => return IpcResponse::err("项目不存在"),
    };

    let manager = detector::detect_node_version_manager();
    if manager == NodeVersionManager::None {
        return IpcResponse::err("未检测到 Node 版本管理器（nvmd/nvm）");
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
