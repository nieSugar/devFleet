// 这个文件是 Tauri 的「命令层」，所有 #[tauri::command] 函数都是前端可调用的 IPC 接口
// 前端调用方式：await invoke('函数名', { 参数名: 值 })
// Tauri 会自动把 JS 参数反序列化为 Rust 类型，把 Rust 返回值序列化为 JSON 发回前端

// crate:: 前缀表示从当前 crate（项目）的其他模块导入
use crate::config;
use crate::detector;
use crate::models::{AppState, IpcResponse, PackageManager, ProjectConfig};
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
    let mut cfg = config::load();
    // 加载配置后，对每个项目补充检测包管理器
    for p in &mut cfg.projects {
        if p.package_manager.is_none() {
            p.package_manager =
                Some(detector::detect_package_manager(&p.path).to_string());
        }
    }
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
    let osa = format!(
        r#"tell application "Terminal"
  activate
  do script "cd \"{}\" && {}"
end tell"#,
        project_path.replace('"', "\\\""),
        run_command,
    );
    Command::new("osascript").args(["-e", &osa]).spawn().is_ok()
}

/// Linux: 依次尝试 gnome-terminal → konsole → xterm，用第一个可用的
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn spawn_external_terminal(project_path: &str, run_command: &str) -> bool {
    let gnome_cmd = format!("cd \"{}\" && {}; exec bash", project_path, run_command);
    let konsole_cmd = format!("bash -lc \"cd '{}' && {}; exec bash\"", project_path, run_command);
    let xterm_cmd = format!("bash -lc \"cd '{}' && {}; exec bash\"", project_path, run_command);
    // Vec<(&str, Vec<&str>)>：终端程序名和对应的命令行参数列表
    let terminals: Vec<(&str, Vec<&str>)> = vec![
        ("gnome-terminal", vec!["--", "bash", "-lc", &gnome_cmd]),
        ("konsole", vec!["-e", &konsole_cmd]),
        ("xterm", vec!["-e", &xterm_cmd]),
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

#[tauri::command]
pub fn run_script(
    project_path: String,
    script_name: String,
    project_id: String,
    package_manager: Option<String>,
    node_version: Option<String>,
) -> IpcResponse {
    // let _ = 是 Rust 的惯用写法，表示"我知道有这个参数但暂时不用"
    // 不写的话编译器会警告 unused variable
    let _ = project_id;
    if !validate_script_name(&script_name) {
        return IpcResponse::err("脚本名称包含非法字符，仅允许字母、数字、连字符、下划线、冒号和点");
    }

    // Option 的链式处理：
    // .and_then() 在 Some 时执行闭包，None 时直接传递 None
    // .unwrap_or_else() 在 None 时执行闭包提供默认值
    let pm = package_manager
        .and_then(|s| s.parse::<PackageManager>().ok())
        .unwrap_or_else(|| detector::detect_package_manager(&project_path));

    let run_command = pm.run_command(&script_name);

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
pub fn stop_script(state: State<'_, AppState>, project_id: String) -> IpcResponse {
    // .lock() 获取互斥锁，返回 Result（可能失败，比如锁被"poisoned"了）
    // poisoned 指之前持有锁的线程 panic 了，锁的数据可能不一致
    let mut procs = match state.running_processes.lock() {
        Ok(p) => p,
        Err(_) => return IpcResponse::err("内部状态锁异常"),
    };

    // if let 是 match 的语法糖，只关心一个分支时比 match 更简洁
    // .remove() 从 HashMap 中取出并移除这个 key
    if let Some(mut child) = procs.remove(&project_id) {
        if cfg!(target_os = "windows") {
            // Windows 下用 taskkill 杀进程树：/t = 包含子进程，/f = 强制
            // 因为 child.kill() 在 Windows 只杀父进程，不杀子进程
            let pid = child.id();
            let _ = Command::new("taskkill")
                .args(["/pid", &pid.to_string(), "/t", "/f"])
                .spawn();
        } else {
            let _ = child.kill();
        }
        IpcResponse::ok(serde_json::json!({
            "message": "脚本已停止",
            "mode": "managed",
            "trackable": true,
        }))
    } else {
        IpcResponse::err(EXTERNAL_UNTRACKABLE)
    }
}

/// 检查脚本是否仍在运行
#[tauri::command]
pub fn check_script_status(state: State<'_, AppState>, project_id: String) -> IpcResponse {
    let mut procs = match state.running_processes.lock() {
        Ok(p) => p,
        Err(_) => return IpcResponse::err("内部状态锁异常"),
    };

    if let Some(child) = procs.get_mut(&project_id) {
        // try_wait() 非阻塞地检查进程状态：
        //   Ok(Some(_)) → 进程已退出
        //   Ok(None) → 进程还在跑
        //   Err(_) → 检查失败（比如权限问题）
        match child.try_wait() {
            Ok(Some(_)) => {
                procs.remove(&project_id);
                IpcResponse::ok(serde_json::json!({
                    "isRunning": false,
                    "mode": "managed",
                    "trackable": true,
                    "reason": "process_exited",
                }))
            }
            Ok(None) => IpcResponse::ok(serde_json::json!({
                "isRunning": true,
                "mode": "managed",
                "trackable": true,
            })),
            Err(_) => {
                procs.remove(&project_id);
                IpcResponse::ok(serde_json::json!({
                    "isRunning": false,
                    "mode": "managed",
                    "trackable": true,
                    "reason": "process_exited",
                }))
            }
        }
    } else {
        // HashMap 里没有这个 project_id，说明是外部终端启动的，无法追踪
        IpcResponse::ok(serde_json::json!({
            "isRunning": false,
            "mode": "external",
            "trackable": false,
            "reason": EXTERNAL_UNTRACKABLE,
        }))
    }
}

/// 获取脚本的输出内容
#[tauri::command]
pub fn get_script_output(state: State<'_, AppState>, project_id: String) -> IpcResponse {
    let outputs = match state.script_outputs.lock() {
        Ok(o) => o,
        Err(_) => return IpcResponse::err("内部状态锁异常"),
    };

    if let Some(buf) = outputs.get(&project_id) {
        // 双层 lock：先锁 HashMap 拿到 Arc<Mutex<String>>，再锁内层 Mutex 拿到 String
        let text = buf.lock().map(|b| b.clone()).unwrap_or_default();
        IpcResponse::ok(serde_json::json!({ "output": text }))
    } else {
        IpcResponse::ok(serde_json::json!({ "output": "" }))
    }
}

// ── 编辑器命令 ──

/// 检测系统中安装了哪些代码编辑器
#[tauri::command]
pub fn detect_editors() -> IpcResponse {
    // 返回元组 (bool, bool, bool)，Rust 的元组解构赋值
    let (vscode, cursor, webstorm) = detector::detect_editors();
    IpcResponse::ok(serde_json::json!({ "vscode": vscode, "cursor": cursor, "webstorm": webstorm }))
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

/// 通过版本管理器安装指定版本的 Node.js
#[tauri::command]
pub fn install_node_version(version: String, manager: Option<String>) -> IpcResponse {
    let mgr = if let Some(m) = manager {
        match m.as_str() {
            "nvmd" => crate::models::NodeVersionManager::Nvmd,
            "nvs" => crate::models::NodeVersionManager::Nvs,
            "nvm" => crate::models::NodeVersionManager::Nvm,
            "nvm-windows" => crate::models::NodeVersionManager::NvmWindows,
            _ => detector::detect_node_version_manager(),
        }
    } else {
        detector::detect_node_version_manager()
    };

    if mgr == crate::models::NodeVersionManager::None {
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
    let mgr = if let Some(m) = manager {
        match m.as_str() {
            "nvmd" => crate::models::NodeVersionManager::Nvmd,
            "nvs" => crate::models::NodeVersionManager::Nvs,
            "nvm" => crate::models::NodeVersionManager::Nvm,
            "nvm-windows" => crate::models::NodeVersionManager::NvmWindows,
            _ => detector::detect_node_version_manager(),
        }
    } else {
        detector::detect_node_version_manager()
    };

    if mgr == crate::models::NodeVersionManager::None {
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
    let mgr = if let Some(m) = manager {
        match m.as_str() {
            "nvmd" => crate::models::NodeVersionManager::Nvmd,
            "nvs" => crate::models::NodeVersionManager::Nvs,
            "nvm" => crate::models::NodeVersionManager::Nvm,
            "nvm-windows" => crate::models::NodeVersionManager::NvmWindows,
            _ => detector::detect_node_version_manager(),
        }
    } else {
        detector::detect_node_version_manager()
    };

    if mgr == crate::models::NodeVersionManager::None {
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
    if manager == crate::models::NodeVersionManager::None {
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
        crate::models::NodeVersionManager::Nvmd => ".nvmdrc",
        crate::models::NodeVersionManager::Nvs => ".node-version",
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
