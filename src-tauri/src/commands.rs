// 这个文件是 Tauri 的「命令层」，所有 #[tauri::command] 函数都是前端可调用的 IPC 接口
// 前端调用方式：await invoke('函数名', { 参数名: 值 })
// Tauri 会自动把 JS 参数反序列化为 Rust 类型，把 Rust 返回值序列化为 JSON 发回前端

// crate:: 前缀表示从当前 crate（项目）的其他模块导入
use crate::config;
use crate::detector;
use crate::editors;
use crate::models::{
    CustomEditor, EditorCache, IpcResponse, NodeVersionManager, PackageManager, ProjectConfig,
};
use crate::project;
use crate::shell_context;
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
pub fn get_default_editor() -> IpcResponse {
    match config::load_default_editor_id() {
        Ok(editor_id) => IpcResponse::ok(serde_json::json!({ "editorId": editor_id })),
        Err(error) => IpcResponse::err(error),
    }
}

#[tauri::command]
pub fn set_default_editor(editor_id: Option<String>) -> IpcResponse {
    match config::save_default_editor_id(editor_id.as_deref()) {
        Ok(editor_id) => IpcResponse::ok(serde_json::json!({ "editorId": editor_id })),
        Err(error) => IpcResponse::err(error),
    }
}

fn project_config_response(cfg: ProjectConfig) -> IpcResponse {
    let availability: std::collections::HashMap<_, _> = cfg
        .projects
        .iter()
        .map(|p| (p.id.clone(), project::is_valid_path(&p.path)))
        .collect();
    let mut pinned_project_ids = cfg
        .settings
        .as_ref()
        .map(|settings| settings.pinned_project_ids.clone())
        .unwrap_or_default();
    crate::config::normalize_pinned_project_ids(&mut pinned_project_ids, &cfg.projects);
    // 可用性和置顶列表只属于本次响应，不重复写入项目结构。
    let mut data = serde_json::json!(cfg);
    data["availability"] = serde_json::json!(availability);
    data["pinnedProjectIds"] = serde_json::json!(pinned_project_ids);
    IpcResponse::ok(data)
}

#[tauri::command]
pub async fn load_project_config() -> IpcResponse {
    tokio::task::spawn_blocking(|| match config::load_checked() {
        Ok(cfg) => project_config_response(cfg),
        Err(e) => IpcResponse::err(e),
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

#[tauri::command]
pub async fn refresh_project_config() -> IpcResponse {
    tokio::task::spawn_blocking(|| match config::load_and_refresh() {
        Ok(cfg) => project_config_response(cfg),
        Err(e) => IpcResponse::err(e),
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

#[tauri::command]
pub async fn relocate_project(project_id: String, project_path: String) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        match config::relocate_project(&project_id, &project_path) {
            Ok(project) => IpcResponse::ok(project),
            Err(e) => IpcResponse::err(e),
        }
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
pub async fn set_project_script(project_id: String, script_name: String) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        match config::update_project(&project_id, |project| {
            if !project
                .scripts
                .iter()
                .any(|script| script.name == script_name)
            {
                return Err("脚本不存在，请刷新后重新选择".into());
            }
            project.selected_script = Some(script_name);
            Ok(())
        }) {
            Ok(project) => IpcResponse::ok(project),
            Err(error) => IpcResponse::err(error),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {e}")))
}

#[tauri::command]
pub async fn set_project_note(project_id: String, note: String) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        match config::update_project(&project_id, |project| {
            project.note = (!note.trim().is_empty()).then(|| note.trim().to_string());
            Ok(())
        }) {
            Ok(project) => IpcResponse::ok(project),
            Err(error) => IpcResponse::err(error),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {e}")))
}

#[tauri::command]
pub async fn set_project_pinned(project_id: String, pinned: bool) -> IpcResponse {
    tokio::task::spawn_blocking(
        move || match config::set_project_pinned(&project_id, pinned) {
            Ok(project_ids) => IpcResponse::ok(serde_json::json!({ "projectIds": project_ids })),
            Err(error) => IpcResponse::err(error),
        },
    )
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {e}")))
}

#[tauri::command]
pub fn sync_app_language(app: tauri::AppHandle, language: String) -> IpcResponse {
    #[cfg(target_os = "macos")]
    {
        let language = language.trim();
        let language = if language.is_empty() {
            None
        } else {
            Some(language)
        };

        if let Err(error) = crate::sync_macos_app_language(&app, language) {
            return IpcResponse::err(format!("同步 macOS 菜单语言失败: {}", error));
        }
    }

    let _ = (&app, &language);

    IpcResponse::ok_msg("语言同步成功")
}

#[tauri::command]
pub fn add_project_to_config(project_path: String) -> IpcResponse {
    if !project::is_valid_path(&project_path) {
        return IpcResponse::err("所选文件夹不是有效的项目目录（缺少 package.json）");
    }
    if !project::has_valid_package_json(&project_path) {
        return IpcResponse::err("package.json 不是有效的 JSON 对象，无法添加项目");
    }
    match project::add_to_config(&project_path) {
        Ok(p) => IpcResponse::ok(p),
        Err(true) => IpcResponse::err("该项目路径已存在，请勿重复添加"),
        Err(false) => IpcResponse::err("添加项目失败"),
    }
}

#[tauri::command]
pub async fn scan_project_candidates(root_path: String) -> IpcResponse {
    let generation = project::begin_project_scan();
    tokio::task::spawn_blocking(move || {
        IpcResponse::ok(project::scan_project_candidates(&root_path, generation))
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {e}")))
}

#[tauri::command]
pub fn cancel_project_scan() -> IpcResponse {
    project::cancel_project_scan();
    IpcResponse::ok_msg("扫描已取消")
}

#[tauri::command]
pub fn remove_project_from_config(project_id: String) -> IpcResponse {
    if project::remove_from_config(&project_id) {
        IpcResponse::ok_msg("项目删除成功")
    } else {
        IpcResponse::err("删除项目失败")
    }
}

#[tauri::command]
pub fn get_shell_context_menu_state() -> IpcResponse {
    IpcResponse::ok(shell_context::get_state())
}

#[tauri::command]
pub fn set_shell_context_menu_enabled(enabled: bool) -> IpcResponse {
    match shell_context::set_enabled(enabled) {
        Ok(state) => IpcResponse::ok(state),
        Err(e) => IpcResponse::err(e),
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
            .raw_arg(format!("/D /V:OFF /K {}", run_command))
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
fn exact_node_version(value: &str) -> Option<&str> {
    let version = value.trim().strip_prefix('v').unwrap_or(value.trim());
    let parts: Vec<_> = version.split('.').collect();
    (parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())))
    .then_some(version)
}

fn verify_node_output(expected: &str, output: &str) -> Result<(), (&'static str, String)> {
    match exact_node_version(output) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err((
            "NODE_VERSION_MISMATCH",
            format!("要求 Node {expected}，实际为 {actual}"),
        )),
        None => Err(("NODE_VERSION_CHECK_FAILED", "无法识别 Node 版本输出".into())),
    }
}

fn checked_node_prefix(
    version: &str,
    manager: &NodeVersionManager,
    project_path: &str,
) -> Result<Option<String>, (&'static str, String)> {
    let dir = detector::get_node_bin_dir(version, manager);
    let executable = match dir.as_ref() {
        Some(dir) => {
            let executable = dir.join(if cfg!(windows) { "node.exe" } else { "node" });
            if !executable.is_file() {
                return Err((
                    "NODE_VERSION_MISSING",
                    format!("Node {version} 可执行文件不存在"),
                ));
            }
            executable
        }
        // nvmd 可通过 shim 接管；先验证已安装，再在项目目录探测，不能静默下载安装。
        None if *manager == NodeVersionManager::Nvmd => {
            if !detector::get_node_versions(manager)
                .iter()
                .any(|v| v.version == version)
            {
                return Err((
                    "NODE_VERSION_MISSING",
                    format!("未确认 nvmd 已安装 Node {version}"),
                ));
            }
            std::path::PathBuf::from("node")
        }
        None => return Err(("NODE_VERSION_MISSING", format!("未找到 Node {version}"))),
    };
    let mut probe = Command::new(executable);
    probe.arg("--version").current_dir(project_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        probe.creation_flags(0x08000000);
    }
    let output =
        detector::output_with_timeout(probe, 5).map_err(|e| ("NODE_VERSION_CHECK_FAILED", e))?;
    if !output.status.success() {
        return Err((
            "NODE_VERSION_CHECK_FAILED",
            "Node 版本校验命令执行失败".into(),
        ));
    }
    verify_node_output(version, &String::from_utf8_lossy(&output.stdout))?;
    Ok(dir.as_deref().and_then(format_path_prefix))
}

fn version_guard(version: &str) -> String {
    // 外部终端可能加载不同环境；执行脚本前再核对一次，失败不会执行后面的脚本。
    format!("node -e \"if(process.versions.node !== '{version}'){{console.error('DevFleet: Node version mismatch.');process.exit(1)}}\"")
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
        Some(format!(
            "export PATH='{}':\"$PATH\"",
            dir_str.replace('\'', "'\\''")
        ))
    }
}

#[tauri::command]
pub async fn run_script(
    project_path: String,
    script_name: String,
    _project_id: String,
    package_manager: Option<String>,
    node_version: Option<String>,
) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        run_script_checked(&project_path, &script_name, package_manager, node_version)
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

fn run_script_checked(
    project_path: &str,
    script_name: &str,
    package_manager: Option<String>,
    node_version: Option<String>,
) -> IpcResponse {
    if !project::is_valid_path(project_path) {
        return IpcResponse::err_code("PROJECT_UNAVAILABLE", "项目路径不可用，请重新定位后再运行");
    }
    if !validate_script_name(script_name) {
        return IpcResponse::err(
            "脚本名称包含非法字符，仅允许字母、数字、连字符、下划线、冒号和点",
        );
    }

    let pm = package_manager
        .and_then(|s| s.parse::<PackageManager>().ok())
        .unwrap_or_else(|| detector::detect_package_manager(project_path));

    let base_command = pm.run_command(script_name);

    let mut verified_version = None;
    let run_command = match node_version.as_deref().filter(|v| !v.trim().is_empty()) {
        Some(ver) => {
            let Some(version) = exact_node_version(ver) else {
                return IpcResponse::err_code(
                    "NODE_VERSION_UNRESOLVED",
                    format!("无法确认版本需求 {ver}，请选用已安装的完整版本"),
                );
            };
            let manager = detector::detect_node_version_manager();
            let prefix = match checked_node_prefix(version, &manager, project_path) {
                Ok(prefix) => prefix,
                Err((code, message)) => return IpcResponse::err_code(code, message),
            };
            verified_version = Some(version.to_string());
            let guarded = format!("{} && {}", version_guard(version), base_command);
            prefix.map_or_else(
                || guarded.clone(),
                |prefix| format!("{prefix} && {guarded}"),
            )
        }
        None => match build_builtin_current_path_prefix() {
            Some(prefix) => format!("{} && {}", prefix, base_command),
            None => base_command,
        },
    };

    if !spawn_external_terminal(project_path, &run_command) {
        return IpcResponse::err("启动外部终端失败，无法找到可用的终端程序");
    }

    IpcResponse::ok(serde_json::json!({
        "message": "已提交到外部终端",
        "command": run_command,
        "packageManager": pm.to_string(),
        "nodeVersion": verified_version,
    }))
}

// ── Node 进程管理命令 ──

#[tauri::command]
pub async fn list_node_processes() -> IpcResponse {
    tokio::task::spawn_blocking(|| {
        let cfg = config::load();
        match crate::node_processes::list_node_processes(&cfg.projects) {
            Ok(processes) => IpcResponse::ok(processes),
            Err(e) => IpcResponse::err(e),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

#[tauri::command]
pub async fn kill_node_process(
    pid: u32,
    expected_started_at: Option<String>,
    expected_command_line: Option<String>,
    expected_executable: Option<String>,
) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        match crate::node_processes::kill_node_process(
            pid,
            expected_started_at.as_deref(),
            expected_command_line.as_deref(),
            expected_executable.as_deref(),
        ) {
            Ok(()) => IpcResponse::ok_msg(format!("已结束 Node 进程 {}", pid)),
            Err(e) => IpcResponse::err(e),
        }
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;

// ── 编辑器命令 ──

#[cfg(target_os = "linux")]
async fn render_editor_views(
    app: tauri::AppHandle,
    editors: EditorCache,
    custom_editors: Vec<CustomEditor>,
) -> IpcResponse {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    if let Err(error) = app.run_on_main_thread(move || {
        let _ = sender.send(crate::editors::project_editor_views(
            &editors,
            &custom_editors,
        ));
    }) {
        return IpcResponse::err(format!("调度 Linux 图标解析失败: {}", error));
    }
    match receiver.await {
        Ok(views) => IpcResponse::ok(views),
        Err(_) => IpcResponse::err("Linux 图标解析任务未返回结果"),
    }
}

#[cfg(not(target_os = "linux"))]
async fn render_editor_views(
    _app: tauri::AppHandle,
    editors: EditorCache,
    custom_editors: Vec<CustomEditor>,
) -> IpcResponse {
    tokio::task::spawn_blocking(move || {
        IpcResponse::ok(crate::editors::project_editor_views(
            &editors,
            &custom_editors,
        ))
    })
    .await
    .unwrap_or_else(|error| IpcResponse::err(format!("图标解析失败: {}", error)))
}

/// 检测系统中安装了哪些代码编辑器（带缓存，force=true 时强制重新检测）
#[tauri::command]
pub async fn detect_editors(app: tauri::AppHandle, force: Option<bool>) -> IpcResponse {
    let editors = tokio::task::spawn_blocking(
        move || -> Result<(EditorCache, Vec<CustomEditor>), String> {
            let editors = if force == Some(true) {
                crate::icons::clear_cache();
                let editors = detector::detect_editors();
                config::save_editor_cache(&editors)
                    .map_err(|error| format!("保存编辑器缓存失败: {}", error))?;
                editors
            } else {
                match config::load_editor_cache() {
                    Ok(Some(cached)) => cached,
                    Ok(None) => {
                        let editors = detector::detect_editors();
                        config::save_editor_cache(&editors)
                            .map_err(|error| format!("保存编辑器缓存失败: {}", error))?;
                        editors
                    }
                    Err(error) => return Err(error),
                }
            };
            let custom_editors = config::load_custom_editors()?;
            Ok((editors, custom_editors))
        },
    )
    .await;

    match editors {
        Ok(Ok((editors, custom_editors))) => {
            render_editor_views(app, editors, custom_editors).await
        }
        Ok(Err(error)) => IpcResponse::err(error),
        Err(error) => IpcResponse::err(format!("内部错误: {}", error)),
    }
}

#[tauri::command]
pub fn upsert_custom_editor(request: editors::UpsertCustomEditorRequest) -> IpcResponse {
    match editors::upsert_custom_editor(request) {
        Ok(editor) => IpcResponse::ok(editor),
        Err(error) => IpcResponse::err(error),
    }
}

#[tauri::command]
pub fn remove_custom_editor(editor_id: String) -> IpcResponse {
    match editors::remove_custom_editor(&editor_id) {
        Ok(editor) => IpcResponse::ok(editor),
        Err(error) => IpcResponse::err(error),
    }
}

#[tauri::command]
pub async fn discover_editor_candidates() -> IpcResponse {
    tokio::task::spawn_blocking(|| match crate::candidates::discover_editor_candidates() {
        Ok(result) => IpcResponse::ok(result),
        Err(error) => IpcResponse::err(error),
    })
    .await
    .unwrap_or_else(|error| IpcResponse::err(format!("候选扫描失败: {}", error)))
}

#[tauri::command]
pub fn import_editor_candidate(
    request: crate::candidates::ImportEditorCandidateRequest,
) -> IpcResponse {
    match crate::candidates::import_editor_candidate(request) {
        Ok(editor) => IpcResponse::ok(editor),
        Err(error) => IpcResponse::err(error),
    }
}

/// 用指定编辑器打开项目
#[tauri::command]
pub fn open_in_editor(editor: String, project_path: String) -> IpcResponse {
    match editors::open_editor(&editor, &project_path) {
        Ok(()) => IpcResponse::ok(serde_json::json!({ "message": "已打开编辑器" })),
        Err(error) => IpcResponse::err(error),
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
    match config::save_node_mirror(value) {
        Ok(()) => IpcResponse::ok_msg("镜像地址已更新"),
        Err(e) => IpcResponse::err(format!("保存镜像地址失败: {}", e)),
    }
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
    tokio::task::spawn_blocking(|| match crate::node_manager::add_to_system_path() {
        Ok(msg) => IpcResponse::ok(serde_json::json!({ "message": msg })),
        Err(e) => IpcResponse::err(e),
    })
    .await
    .unwrap_or_else(|e| IpcResponse::err(format!("内部错误: {}", e)))
}

/// 检查 builtin Node 是否已在系统 PATH 中，
/// 同时检测系统中 node 命令是否可用（避免用户已有外部 node 时误报）
#[tauri::command]
pub fn check_node_in_path() -> IpcResponse {
    let bin_path = crate::node_manager::get_current_bin_path();
    let in_path = bin_path.as_ref().is_some_and(|p| {
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
        "powerShellPolicyReady": crate::node_manager::is_powershell_execution_policy_configured(),
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
    match config::save_node_install_dir(value) {
        Ok(()) => IpcResponse::ok_msg("安装目录已更新"),
        Err(e) => IpcResponse::err(format!("保存安装目录失败: {}", e)),
    }
}

/// 设置项目的 Node 版本（写入对应的版本文件，如 .nvmrc）
#[tauri::command]
pub fn set_project_node_version(project_id: String, node_version: Option<String>) -> IpcResponse {
    let mut manager = detector::detect_node_version_manager();
    if manager == NodeVersionManager::None {
        manager = NodeVersionManager::Builtin;
    }

    // .as_deref() 把 Option<String> 转为 Option<&str>
    // 这是 Rust 所有权系统的常见操作：String 是拥有所有权的，&str 是借用的
    let nv = node_version.as_deref();
    let updated_proj = match config::update_project(&project_id, |item| {
        if !project::is_valid_path(&item.path) {
            return Err("项目路径不可用，请先重新定位".into());
        }
        if !project::set_node_version_file(&item.path, nv, &manager) {
            return Err("操作版本配置文件失败".into());
        }
        item.node_version = nv.filter(|v| !v.trim().is_empty()).map(str::to_string);
        Ok(())
    }) {
        Ok(project) => project,
        Err(error) => return IpcResponse::err(error),
    };

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
