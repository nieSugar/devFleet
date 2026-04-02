// 这个文件负责「检测」：包管理器、编辑器、Node 版本管理器
// 核心思路：通过检查 lock 文件是否存在、CLI 命令是否可用来判断用户安装了什么

use crate::models::{EditorInfo, NodeVersion, NodeVersionManager, NvmInfo, PackageManager, RemoteNodeVersion};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, OnceLock};
use std::time::{Duration, Instant};

/// 回收子进程退出状态，通过单一 reaper 线程避免 Unix 上产生僵尸进程
fn detach_child(result: std::io::Result<std::process::Child>) -> bool {
    static REAPER_TX: OnceLock<std::sync::mpsc::Sender<std::process::Child>> = OnceLock::new();
    let tx = REAPER_TX.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<std::process::Child>();
        std::thread::spawn(move || {
            for mut child in rx {
                let _ = child.wait();
            }
        });
        tx
    });
    match result {
        Ok(child) => tx.send(child).is_ok(),
        Err(_) => false,
    }
}

/// Windows 上创建隐藏窗口的 cmd.exe Command，避免弹出黑色控制台窗口
fn new_cmd() -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
        return cmd;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("cmd")
    }
}

// ── 包管理器检测 ──

/// 通过 lock 文件来判断项目使用的包管理器
/// 优先级：bun > pnpm > yarn > npm（lock 文件最可靠）
/// 如果没有 lock 文件，回退到读 package.json 的 packageManager 字段
pub fn detect_package_manager(project_path: &str) -> PackageManager {
    // Path::new() 创建路径对象，.join() 拼接路径（自动处理路径分隔符）
    let p = Path::new(project_path);

    if p.join("bun.lockb").exists() || p.join("bun.lock").exists() {
        return PackageManager::Bun;
    }
    if p.join("pnpm-lock.yaml").exists() {
        return PackageManager::Pnpm;
    }
    if p.join("yarn.lock").exists() {
        return PackageManager::Yarn;
    }
    if p.join("package-lock.json").exists() {
        return PackageManager::Npm;
    }

    // 没有 lock 文件时，检查 package.json 的 "packageManager" 字段
    // 这是 Node.js 的 corepack 规范，格式如 "pnpm@8.15.0"
    let pkg_path = p.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(pm) = pkg.get("packageManager").and_then(|v| v.as_str()) {
                // "pnpm@8.15.0" → split('@') 取第一段 → "pnpm" → parse 成枚举
                let name = pm.split('@').next().unwrap_or("");
                if let Ok(m) = name.parse::<PackageManager>() {
                    return m;
                }
            }
        }
    }

    // 兜底默认值
    PackageManager::Npm
}

// ── 编辑器检测 ──
// 分层检测策略（命中即停，后续层不执行）：
//   Layer 1: OS 原生注册信息 — Win Registry / macOS .app / Linux .desktop（微秒级）
//   Layer 2: 已知安装路径探测 — fs::exists（微秒级）
//   Layer 3: CLI 兜底 — spawn 子进程（秒级，并发执行）

/// 编辑器规格表
#[allow(dead_code)]
struct EditorSpec {
    id: &'static str,
    /// 前端显示名称（作为 IPC 数据的 single source of truth）
    name: &'static str,
    /// Windows: URL scheme 注册表键名（HKCR\{scheme}\shell\open\command）
    url_schemes: &'static [&'static str],
    /// Windows: App Paths 注册表键名
    win_app_path_keys: &'static [&'static str],
    /// macOS: .app 名称列表（按优先级排列，用于检测和 open -a）
    mac_apps: &'static [&'static str],
    /// Linux: .desktop 文件名（不含后缀）
    linux_desktop_names: &'static [&'static str],
    /// 回退：已知安装路径（支持 %ENV_VAR% 展开）
    win_paths: &'static [&'static str],
    linux_paths: &'static [&'static str],
    /// 最终回退：CLI 命令名
    cli_cmds: &'static [&'static str],
}

const EDITORS: &[EditorSpec] = &[
    EditorSpec {
        id: "vscode",
        name: "VS Code",
        url_schemes: &["vscode"],
        win_app_path_keys: &["Code.exe"],
        mac_apps: &["Visual Studio Code"],
        linux_desktop_names: &["code", "code-url-handler", "visual-studio-code"],
        win_paths: &[
            r"%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe",
            r"%ProgramFiles%\Microsoft VS Code\Code.exe",
        ],
        linux_paths: &["/usr/bin/code", "/usr/share/code/code", "/snap/bin/code"],
        cli_cmds: &["code"],
    },
    EditorSpec {
        id: "vscode-insiders",
        name: "VS Code Insiders",
        url_schemes: &["vscode-insiders"],
        win_app_path_keys: &["Code - Insiders.exe"],
        mac_apps: &["Visual Studio Code - Insiders"],
        linux_desktop_names: &["code-insiders", "code-insiders-url-handler", "visual-studio-code-insiders"],
        win_paths: &[
            r"%LOCALAPPDATA%\Programs\Microsoft VS Code Insiders\Code - Insiders.exe",
            r"%ProgramFiles%\Microsoft VS Code Insiders\Code - Insiders.exe",
        ],
        linux_paths: &["/usr/bin/code-insiders", "/usr/share/code-insiders/code-insiders", "/snap/bin/code-insiders"],
        cli_cmds: &["code-insiders"],
    },
    EditorSpec {
        id: "cursor",
        name: "Cursor",
        url_schemes: &["cursor"],
        win_app_path_keys: &["Cursor.exe"],
        mac_apps: &["Cursor"],
        linux_desktop_names: &["cursor", "cursor-url-handler"],
        win_paths: &[
            r"%LOCALAPPDATA%\Programs\cursor\Cursor.exe",
            r"%LOCALAPPDATA%\cursor\Cursor.exe",
        ],
        linux_paths: &["/usr/bin/cursor", "/opt/Cursor/cursor"],
        cli_cmds: &["cursor"],
    },
    EditorSpec {
        id: "windsurf",
        name: "Windsurf",
        url_schemes: &["windsurf"],
        win_app_path_keys: &[],
        mac_apps: &["Windsurf"],
        linux_desktop_names: &["windsurf"],
        win_paths: &[
            r"%LOCALAPPDATA%\Programs\Windsurf\Windsurf.exe",
            r"%LOCALAPPDATA%\Programs\windsurf\Windsurf.exe",
        ],
        linux_paths: &["/usr/bin/windsurf"],
        cli_cmds: &["windsurf"],
    },
    EditorSpec {
        id: "trae",
        name: "Trae",
        url_schemes: &["trae"],
        win_app_path_keys: &[],
        mac_apps: &["Trae", "Trae CN"],
        linux_desktop_names: &["trae"],
        win_paths: &[
            r"%LOCALAPPDATA%\Programs\Trae\Trae.exe",
            r"%LOCALAPPDATA%\Programs\Trae CN\Trae CN.exe",
        ],
        linux_paths: &["/usr/bin/trae"],
        cli_cmds: &["trae"],
    },
    EditorSpec {
        id: "webstorm",
        name: "WebStorm",
        url_schemes: &[],
        win_app_path_keys: &[],
        mac_apps: &["WebStorm"],
        linux_desktop_names: &["webstorm", "jetbrains-webstorm"],
        win_paths: &[
            r"%LOCALAPPDATA%\JetBrains\Toolbox\scripts\webstorm.cmd",
        ],
        linux_paths: &["/usr/bin/webstorm", "/snap/bin/webstorm"],
        cli_cmds: &["webstorm", "webstorm64"],
    },
    EditorSpec {
        id: "idea",
        name: "IntelliJ IDEA",
        url_schemes: &[],
        win_app_path_keys: &[],
        mac_apps: &["IntelliJ IDEA", "IntelliJ IDEA CE"],
        linux_desktop_names: &[
            "idea", "jetbrains-idea",
            "intellij-idea-ultimate", "intellij-idea-community",
        ],
        win_paths: &[
            r"%LOCALAPPDATA%\JetBrains\Toolbox\scripts\idea.cmd",
        ],
        linux_paths: &[
            "/usr/bin/idea",
            "/snap/bin/intellij-idea-ultimate",
            "/snap/bin/intellij-idea-community",
        ],
        cli_cmds: &["idea", "idea64"],
    },
    EditorSpec {
        id: "zed",
        name: "Zed",
        url_schemes: &["zed"],
        win_app_path_keys: &["Zed.exe"],
        mac_apps: &["Zed"],
        linux_desktop_names: &["zed", "dev.zed.Zed"],
        win_paths: &[
            r"%LOCALAPPDATA%\Zed\zed.exe",
            r"%LOCALAPPDATA%\Programs\Zed\Zed.exe",
        ],
        linux_paths: &["/usr/bin/zed", "/usr/local/bin/zed"],
        cli_cmds: &["zed"],
    },
    EditorSpec {
        id: "kiro",
        name: "Kiro",
        url_schemes: &["kiro"],
        win_app_path_keys: &["Kiro.exe"],
        mac_apps: &["Kiro"],
        linux_desktop_names: &["kiro", "kiro-url-handler"],
        win_paths: &[
            r"%LOCALAPPDATA%\Programs\Kiro\Kiro.exe",
        ],
        linux_paths: &["/usr/bin/kiro", "~/.local/bin/kiro"],
        cli_cmds: &["kiro"],
    },
    EditorSpec {
        id: "antigravity",
        name: "Antigravity",
        url_schemes: &["antigravity"],
        win_app_path_keys: &["Antigravity.exe"],
        mac_apps: &["Antigravity"],
        linux_desktop_names: &["antigravity", "google-antigravity"],
        win_paths: &[
            r"%ProgramFiles%\Google\Antigravity\Antigravity.exe",
        ],
        linux_paths: &["/usr/bin/antigravity"],
        cli_cmds: &["antigravity", "agy"],
    },
];

/// 展开路径模板中的 %ENV_VAR% 和 ~ 前缀
/// 环境变量缺失时返回 None（该路径不可用），孤立的 % 不再导致整个函数提前返回
fn expand_env_path(template: &str) -> Option<PathBuf> {
    let mut result = template.to_string();
    for _ in 0..10 {
        let start = match result.find('%') {
            Some(i) => i,
            None => break,
        };
        let end = match result[start + 1..].find('%') {
            Some(e) => e,
            None => break,
        };
        let var_name = &result[start + 1..start + 1 + end];
        let value = std::env::var(var_name).ok()?;
        result = format!("{}{}{}", &result[..start], value, &result[start + 2 + end..]);
    }
    if result.starts_with('~') {
        let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok()?;
        result = format!("{}{}", home, &result[1..]);
    }
    Some(PathBuf::from(result))
}

// ── Layer 1: OS 原生应用注册信息 ──

/// 从注册表命令字符串中提取 exe 路径
/// 格式如: "C:\...\Code.exe" "--open-url" -- "%1"
#[cfg(target_os = "windows")]
fn parse_exe_from_command(cmd: &str) -> Option<PathBuf> {
    let trimmed = cmd.trim();
    let path_str = if trimmed.starts_with('"') {
        trimmed[1..].find('"').map(|end| &trimmed[1..1 + end])
    } else {
        trimmed.split_whitespace().next()
    }?;
    let p = PathBuf::from(path_str);
    if p.exists() { Some(p) } else { None }
}

/// Windows: 通过注册表检测编辑器，返回 exe 路径
/// 先查 URL Scheme（HKCR\{scheme}\shell\open\command），再查 App Paths
#[cfg(target_os = "windows")]
fn find_exe_via_registry(spec: &EditorSpec) -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    for scheme in spec.url_schemes {
        if let Ok(key) = RegKey::predef(HKEY_CLASSES_ROOT)
            .open_subkey(format!(r"{}\shell\open\command", scheme))
        {
            if let Ok(cmd_str) = key.get_value::<String, _>("") {
                if let Some(exe) = parse_exe_from_command(&cmd_str) {
                    return Some(exe);
                }
            }
        }
    }

    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        for key_name in spec.win_app_path_keys {
            let sub = format!(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{}",
                key_name
            );
            if let Ok(key) = RegKey::predef(root).open_subkey(&sub) {
                if let Ok(exe_str) = key.get_value::<String, _>("") {
                    let p = PathBuf::from(exe_str.trim_matches('"'));
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}

/// Linux: 解析 .desktop 文件中的 Exec= 行获取可执行文件路径
#[cfg(target_os = "linux")]
fn parse_desktop_exec(desktop_path: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(desktop_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Exec=") {
            let exec = trimmed.trim_start_matches("Exec=");
            let exe = exec.split_whitespace().next()?;
            return Some(PathBuf::from(exe));
        }
    }
    None
}

/// Linux: 从标准 .desktop 文件目录检测编辑器
#[cfg(target_os = "linux")]
fn find_exe_via_desktop_file(spec: &EditorSpec) -> Option<PathBuf> {
    const DESKTOP_DIRS: &[&str] = &[
        "/usr/share/applications",
        "/usr/local/share/applications",
        "/var/lib/snapd/desktop/applications",
        "/var/lib/flatpak/exports/share/applications",
    ];
    let home = std::env::var("HOME").ok();

    for name in spec.linux_desktop_names {
        let filename = format!("{}.desktop", name);
        for dir in DESKTOP_DIRS {
            let p = Path::new(dir).join(&filename);
            if let Some(exe) = parse_desktop_exec(&p) {
                return Some(exe);
            }
        }
        if let Some(ref h) = home {
            let p = Path::new(h).join(".local/share/applications").join(&filename);
            if let Some(exe) = parse_desktop_exec(&p) {
                return Some(exe);
            }
        }
    }

    None
}

// ── Layer 2: 已知安装路径探测 ──

#[cfg(target_os = "windows")]
fn find_win_exe(spec: &EditorSpec) -> Option<PathBuf> {
    spec.win_paths
        .iter()
        .find_map(|p| expand_env_path(p).filter(|ep| ep.exists()))
}

#[cfg(target_os = "linux")]
fn find_linux_exe(spec: &EditorSpec) -> Option<PathBuf> {
    spec.linux_paths
        .iter()
        .find_map(|p| expand_env_path(p).filter(|ep| ep.exists()))
}

// ── 综合快速检测 ──

/// 依次尝试 OS 原生检测 → 已知路径（全部微秒级，零子进程）
fn is_editor_found_fast(spec: &EditorSpec) -> bool {
    #[cfg(target_os = "windows")]
    {
        return find_exe_via_registry(spec).is_some()
            || spec.win_paths.iter().any(|p| {
                expand_env_path(p).map(|ep| ep.exists()).unwrap_or(false)
            });
    }

    #[cfg(target_os = "macos")]
    {
        return spec.mac_apps.iter().any(|app| {
            Path::new(&format!("/Applications/{}.app", app)).exists()
                || std::env::var("HOME")
                    .ok()
                    .map(|h| Path::new(&format!("{}/Applications/{}.app", h, app)).exists())
                    .unwrap_or(false)
        });
    }

    #[cfg(target_os = "linux")]
    {
        return find_exe_via_desktop_file(spec).is_some()
            || spec.linux_paths.iter().any(|p| {
                expand_env_path(p).map(|ep| ep.exists()).unwrap_or(false)
            });
    }

    #[allow(unreachable_code)]
    false
}

/// 通过 CLI 命令检测是否可用（慢，需要 spawn 进程，带 5 秒超时）
fn is_command_available(cmd: &str) -> bool {
    let child = if cfg!(target_os = "windows") {
        new_cmd()
            .args(["/C", &format!("{} --version", cmd)])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
    } else {
        Command::new("sh")
            .args(["-c", &format!("command -v {} >/dev/null 2>&1", cmd)])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
    };
    match child {
        Ok(mut c) => {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match c.try_wait() {
                    Ok(Some(s)) => return s.success(),
                    Ok(None) if Instant::now() >= deadline => {
                        let _ = c.kill();
                        let _ = c.wait();
                        return false;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                    Err(_) => return false,
                }
            }
        }
        Err(_) => false,
    }
}

/// 检测所有已知编辑器，返回 { id: EditorInfo } 映射
/// 快速检测（Registry / .app / .desktop + 已知路径）未命中的，并发走 CLI 兜底
pub fn detect_editors() -> HashMap<String, EditorInfo> {
    let mut result = HashMap::new();
    let mut need_cli: Vec<&EditorSpec> = Vec::new();

    for spec in EDITORS {
        if is_editor_found_fast(spec) {
            result.insert(spec.id.to_string(), EditorInfo {
                name: spec.name.to_string(),
                installed: true,
            });
        } else {
            need_cli.push(spec);
        }
    }

    if !need_cli.is_empty() {
        let handles: Vec<_> = need_cli
            .into_iter()
            .map(|spec| {
                let id = spec.id.to_string();
                let name = spec.name.to_string();
                let cmds: Vec<String> = spec.cli_cmds.iter().map(|s| s.to_string()).collect();
                std::thread::spawn(move || {
                    let found = cmds.iter().any(|cmd| is_command_available(cmd));
                    (id, name, found)
                })
            })
            .collect();

        for h in handles {
            if let Ok((id, name, found)) = h.join() {
                result.insert(id, EditorInfo { name, installed: found });
            }
        }
    }

    result
}

/// Windows: 启动 exe 或 cmd/bat 脚本打开项目
#[cfg(target_os = "windows")]
fn launch_win_exe(exe: &Path, project_path: &str) -> bool {
    let ext = exe.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat") {
        detach_child(
            new_cmd()
                .args(["/C", &exe.to_string_lossy(), project_path])
                .spawn(),
        )
    } else {
        detach_child(Command::new(exe).arg(project_path).shell_spawn())
    }
}

/// macOS: 检查 .app 是否存在于 /Applications 或 ~/Applications
#[cfg(target_os = "macos")]
fn mac_app_exists(app_name: &str) -> bool {
    Path::new(&format!("/Applications/{}.app", app_name)).exists()
        || std::env::var("HOME")
            .ok()
            .map(|h| Path::new(&format!("{}/Applications/{}.app", h, app_name)).exists())
            .unwrap_or(false)
}

/// 用指定编辑器打开项目目录
/// macOS 先检测 .app 是否存在再调用 open -a（避免系统弹错误弹窗），
/// Windows 优先注册表路径，Linux 优先 .desktop Exec 路径，最终 CLI 兜底
pub fn open_editor(editor_id: &str, project_path: &str) -> bool {
    let spec = match EDITORS.iter().find(|s| s.id == editor_id) {
        Some(s) => s,
        None => return false,
    };

    #[cfg(target_os = "macos")]
    {
        for app in spec.mac_apps {
            if mac_app_exists(app) {
                if detach_child(
                    Command::new("open")
                        .args(["-a", app, project_path])
                        .spawn(),
                ) {
                    return true;
                }
            }
        }
        for cmd in spec.cli_cmds {
            if detach_child(Command::new(cmd).arg(project_path).spawn()) {
                return true;
            }
        }
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(exe) = find_exe_via_registry(spec).or_else(|| find_win_exe(spec)) {
            return launch_win_exe(&exe, project_path);
        }
        for cmd in spec.cli_cmds {
            if detach_child(Command::new(cmd).arg(project_path).shell_spawn()) {
                return true;
            }
        }
        return false;
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(exe) = find_exe_via_desktop_file(spec).or_else(|| find_linux_exe(spec)) {
            if detach_child(Command::new(&exe).arg(project_path).spawn()) {
                return true;
            }
        }
        for cmd in spec.cli_cmds {
            if detach_child(Command::new(cmd).arg(project_path).spawn()) {
                return true;
            }
        }
        return false;
    }

    #[allow(unreachable_code)]
    false
}

// ── Node 版本管理器检测 ──

/// 检测 Unix 系统下 nvm 是否安装
/// nvm 是 shell 函数不是可执行文件，所以不能用 is_command_available 检测
/// 必须先 source nvm.sh 再检查 command -v nvm
/// 注意：不使用 -l（login shell），避免 source 完整 profile 导致卡顿
fn is_unix_nvm_installed() -> bool {
    let nvm_dir_check = std::env::var("NVM_DIR")
        .ok()
        .or_else(|| std::env::var("HOME").ok().map(|h| format!("{}/.nvm", h)));
    if let Some(dir) = &nvm_dir_check {
        if Path::new(dir).join("nvm.sh").exists() {
            return true;
        }
    }
    let mut cmd = Command::new("bash");
    cmd.args([
        "-c",
        r#"export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && command -v nvm >/dev/null 2>&1"#,
    ]);
    command_succeeds_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
}

/// 检测系统中安装了哪种 Node 版本管理器
/// 优先级：builtin（有已安装版本时）> nvmd > nvs > nvm/nvm-windows > builtin（兜底）
pub fn detect_node_version_manager() -> NodeVersionManager {
    if !crate::node_manager::list_installed_versions().is_empty() {
        return NodeVersionManager::Builtin;
    }

    // nvmd（跨平台 GUI 版本管理器）
    let nvmd_available = if cfg!(target_os = "windows") {
        let mut cmd = new_cmd();
        cmd.args(["/C", "nvmd", "--help"]);
        command_succeeds_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
    } else {
        let mut cmd = Command::new("nvmd");
        cmd.arg("--help");
        command_succeeds_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
    };
    if nvmd_available {
        return NodeVersionManager::Nvmd;
    }

    // nvs（跨平台）
    let nvs_available = if cfg!(target_os = "windows") {
        let mut cmd = new_cmd();
        cmd.args(["/C", "nvs", "--version"]);
        command_succeeds_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
    } else {
        let mut cmd = Command::new("nvs");
        cmd.arg("--version");
        command_succeeds_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
    };
    if nvs_available {
        return NodeVersionManager::Nvs;
    }

    // nvm / nvm-windows
    if cfg!(target_os = "windows") {
        let mut cmd = new_cmd();
        cmd.args(["/C", "nvm", "version"]);
        if command_succeeds_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS) {
            return NodeVersionManager::NvmWindows;
        }
    } else if is_unix_nvm_installed() {
        return NodeVersionManager::Nvm;
    }

    // 所有外部管理器都没找到，fallback 到内建管理器
    NodeVersionManager::Builtin
}

/// 获取当前系统正在使用的 Node.js 版本号
pub fn get_current_node_version() -> Option<String> {
    let output = if cfg!(target_os = "windows") {
        let mut cmd = new_cmd();
        cmd.args(["/C", "node", "--version"]);
        output_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
    } else {
        let mut cmd = Command::new("node");
        cmd.arg("--version");
        output_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
    };

    output.ok().and_then(|o| {
        if o.status.success() {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            Some(v.trim_start_matches('v').to_string())
        } else {
            None
        }
    })
}

/// 通过版本管理器自身的命令获取当前版本（比 `node --version` 更可靠）
/// nvmd 的 node shim 在某些进程上下文中可能返回缓存值，
/// 直接用 `nvmd current` 可以绕过 shim 读取真实配置。
fn get_current_version_by_manager(manager: &NodeVersionManager) -> Option<String> {
    if *manager == NodeVersionManager::Builtin {
        return crate::config::load_builtin_current_version();
    }

    let output = match manager {
        NodeVersionManager::Nvmd => {
            if cfg!(target_os = "windows") {
                let mut cmd = new_cmd();
                cmd.args(["/C", "nvmd", "current"]);
                output_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
            } else {
                let mut cmd = Command::new("nvmd");
                cmd.arg("current");
                output_with_timeout(cmd, DISCOVERY_TIMEOUT_SECS)
            }
        }
        _ => return get_current_node_version(),
    };

    let result = output.ok().and_then(|o| {
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        let trimmed = combined.trim().trim_start_matches('v').to_string();
        if !trimmed.is_empty() && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            Some(trimmed)
        } else {
            None
        }
    });

    result.or_else(get_current_node_version)
}

/// 获取指定版本管理器已安装的所有 Node 版本列表
pub fn get_node_versions(manager: &NodeVersionManager) -> Vec<NodeVersion> {
    if *manager == NodeVersionManager::Builtin {
        return crate::node_manager::list_installed_versions();
    }

    let current = get_current_version_by_manager(manager);

    // 不同版本管理器用不同的命令列出已安装版本
    let output = match manager {
        NodeVersionManager::Nvmd => {
            if cfg!(target_os = "windows") {
                let mut cmd = new_cmd();
                cmd.args(["/C", "nvmd", "ls"]);
                output_with_timeout(cmd, VERSION_LIST_TIMEOUT_SECS)
            } else {
                let mut cmd = Command::new("nvmd");
                cmd.arg("ls");
                output_with_timeout(cmd, VERSION_LIST_TIMEOUT_SECS)
            }
        }
        NodeVersionManager::Nvs => {
            if cfg!(target_os = "windows") {
                let mut cmd = new_cmd();
                cmd.args(["/C", "nvs", "ls"]);
                output_with_timeout(cmd, VERSION_LIST_TIMEOUT_SECS)
            } else {
                let mut cmd = Command::new("nvs");
                cmd.arg("ls");
                output_with_timeout(cmd, VERSION_LIST_TIMEOUT_SECS)
            }
        }
        NodeVersionManager::NvmWindows => {
            let mut cmd = new_cmd();
            cmd.args(["/C", "nvm", "list"]);
            output_with_timeout(cmd, VERSION_LIST_TIMEOUT_SECS)
        }
        NodeVersionManager::Nvm => {
            // 仅手动 source nvm.sh，避免 login shell 读取 profile 导致 macOS 首屏卡住。
            let mut cmd = Command::new("bash");
            cmd.args([
                "-c",
                r#"export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm ls"#,
            ]);
            output_with_timeout(cmd, VERSION_LIST_TIMEOUT_SECS)
        }
        NodeVersionManager::Builtin | NodeVersionManager::None => return vec![],
    };

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    // nvmd 把版本列表输出到 stderr 而不是 stdout（它的特殊行为）
    let text = if *manager == NodeVersionManager::Nvmd {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    static VERSION_RE: LazyLock<regex_lite::Regex> =
        LazyLock::new(|| regex_lite::Regex::new(r"(?:node/)?v?(\d+\.\d+\.\d+)").unwrap());
    let version_re = &*VERSION_RE;
    // HashMap 用于去重，同一版本号只保留第一次出现的
    let mut seen = HashMap::new();

    for line in text.lines() {
        // nvm ls 输出中 "N/A" 表示 alias 指向的版本未安装（如 lts/argon -> v4.9.1 (-> N/A)），跳过
        if line.contains("N/A") {
            continue;
        }
        if let Some(caps) = version_re.captures(line) {
            // caps[1] 是正则的第一个捕获组（括号里的部分）
            let ver = caps[1].to_string();
            if seen.contains_key(&ver) {
                continue;
            }
            // 多种方式判断是否为当前使用的版本
            let is_current = current.as_deref() == Some(&ver)
                || line.contains("(currently)")
                || line.contains("(current)")
                || line.trim().starts_with('>');

            seen.insert(
                ver.clone(),
                NodeVersion {
                    full_version: format!("v{}", ver),
                    version: ver,
                    path: None,
                    is_current: Some(is_current),
                },
            );
        }
    }

    // into_values() 消耗 HashMap，只取 values（丢弃 keys）
    let mut versions: Vec<NodeVersion> = seen.into_values().collect();
    // 按版本号降序排列（最新版在前）
    versions.sort_by(|a, b| {
        let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };
        parse(&b.version).cmp(&parse(&a.version))
    });
    versions
}

/// 汇总 Node 版本管理器的完整信息
pub fn get_nvm_info() -> NvmInfo {
    let manager = detect_node_version_manager();
    let is_installed = manager != NodeVersionManager::None;

    if !is_installed {
        return NvmInfo {
            is_installed,
            manager,
            current_version: None,
            available_versions: vec![],
        };
    }

    let mgr_clone = manager.clone();
    let versions_handle = std::thread::spawn(move || get_node_versions(&mgr_clone));
    let current_version = get_current_version_by_manager(&manager);
    let available_versions = versions_handle.join().unwrap_or_default();

    NvmInfo {
        is_installed,
        manager,
        current_version,
        available_versions,
    }
}

// ── 远程版本获取 ──

const DEFAULT_NODE_DIST_BASE: &str = "https://nodejs.org/dist";

/// 获取远程 Node.js 版本列表，支持自定义镜像地址
/// `mirror` 传入镜像 base URL（如 `https://npmmirror.com/mirrors/node`），
/// 为 None 时使用官方源。
pub fn fetch_remote_node_versions(mirror: Option<&str>) -> Result<Vec<RemoteNodeVersion>, String> {
    let base = mirror.unwrap_or(DEFAULT_NODE_DIST_BASE);
    let url = format!("{}/index.json", base.trim_end_matches('/'));

    static HTTP_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(30))
            .build()
    });

    let resp = HTTP_AGENT
        .get(&url)
        .call()
        .map_err(|e| format!("请求 Node.js 版本列表失败（{}）: {}", url, e))?;

    resp.into_json::<Vec<RemoteNodeVersion>>()
        .map_err(|e| format!("解析版本数据失败: {}", e))
}

/// 检测 nvm-windows 是否可用（Windows only）
fn is_nvm_windows_available() -> bool {
    if !cfg!(target_os = "windows") {
        return false;
    }
    let r = new_cmd()
        .args(["/C", "nvm", "version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    matches!(r, Ok(s) if s.success())
}

/// 解析命令输出，合并 stdout + stderr
fn collect_output(o: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
    format!("{}{}", stdout, stderr)
}

const DISCOVERY_TIMEOUT_SECS: u64 = 5;
const VERSION_LIST_TIMEOUT_SECS: u64 = 10;
const CMD_TIMEOUT_SECS: u64 = 30;

/// 带超时地检查命令是否执行成功，用于启动阶段的轻量探测。
fn command_succeeds_with_timeout(cmd: Command, timeout_secs: u64) -> bool {
    output_with_timeout(cmd, timeout_secs)
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 带超时保护的命令执行，防止 nvm-windows 等工具卡死时阻塞整个应用。
/// 在独立线程中读取 stdout/stderr 以避免管道缓冲区满导致死锁。
fn output_with_timeout(
    mut cmd: Command,
    timeout_secs: u64,
) -> Result<std::process::Output, String> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动命令失败: {}", e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = Vec::new();
        if let Some(mut r) = stdout {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    });
    let stderr_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = Vec::new();
        if let Some(mut r) = stderr {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout_bytes = stdout_handle.join().unwrap_or_default();
                let stderr_bytes = stderr_handle.join().unwrap_or_default();
                return Ok(std::process::Output {
                    status,
                    stdout: stdout_bytes,
                    stderr: stderr_bytes,
                });
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "命令执行超时（{}秒），可能需要管理员权限或存在环境冲突",
                    timeout_secs
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("等待命令完成失败: {}", e));
            }
        }
    }
}

/// nvmd CLI 的 exit code 不可信（成功和失败都返回 0），
/// 需要通过输出文本中的关键词判断真实结果
fn nvmd_output_has_error(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("has not been installed")
        || lower.contains("not installed")
        || lower.contains("not found")
        || lower.contains("error")
}

// ── 版本管理器路径缓存 ──
// 各管理器的安装目录在应用运行期间不会变化，用 OnceLock 缓存避免重复检测

static NVM_WINDOWS_ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

/// 获取 nvm-windows 的版本安装根目录（缓存结果）
/// 优先读取 NVM_HOME 环境变量，回退到 `nvm root` 命令
fn get_nvm_windows_root() -> Option<PathBuf> {
    NVM_WINDOWS_ROOT
        .get_or_init(|| {
            if let Ok(home) = std::env::var("NVM_HOME") {
                let p = PathBuf::from(&home);
                if p.is_dir() {
                    return Some(p);
                }
            }
            if cfg!(target_os = "windows") {
                if let Ok(output) = new_cmd().args(["/C", "nvm", "root"]).output() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    for line in text.lines() {
                        let trimmed = line
                            .trim()
                            .trim_start_matches("Current Root: ")
                            .trim_end_matches('\\');
                        let p = PathBuf::from(trimmed);
                        if p.is_dir() {
                            return Some(p);
                        }
                    }
                }
            }
            None
        })
        .clone()
}

static NVS_ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

/// 获取 nvs 的根目录（缓存结果）
fn get_nvs_root() -> Option<PathBuf> {
    NVS_ROOT
        .get_or_init(|| {
            if let Ok(home) = std::env::var("NVS_HOME") {
                let p = PathBuf::from(&home);
                if p.is_dir() {
                    return Some(p);
                }
            }
            let fallback = if cfg!(target_os = "windows") {
                std::env::var("LOCALAPPDATA")
                    .ok()
                    .map(|la| PathBuf::from(la).join("nvs"))
            } else {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".nvs"))
            };
            fallback.filter(|p| p.is_dir())
        })
        .clone()
}

static NVM_UNIX_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// 获取 Unix nvm 的目录（缓存结果）
fn get_nvm_unix_dir() -> Option<PathBuf> {
    NVM_UNIX_DIR
        .get_or_init(|| {
            if let Ok(dir) = std::env::var("NVM_DIR") {
                let p = PathBuf::from(&dir);
                if p.is_dir() {
                    return Some(p);
                }
            }
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".nvm"))
                .filter(|p| p.is_dir())
        })
        .clone()
}

/// 获取指定版本 Node.js 的二进制文件所在目录
/// 用于 PATH 注入，实现进程级别的版本隔离（不修改全局状态）
pub fn get_node_bin_dir(version: &str, manager: &NodeVersionManager) -> Option<PathBuf> {
    let ver = version.trim().trim_start_matches('v');
    if ver.is_empty() {
        return None;
    }

    let dir = match manager {
        NodeVersionManager::NvmWindows => {
            let root = get_nvm_windows_root()?;
            let with_v = root.join(format!("v{}", ver));
            if with_v.is_dir() {
                with_v
            } else {
                root.join(ver)
            }
        }
        NodeVersionManager::Nvs => {
            let root = get_nvs_root()?;
            let arch = if cfg!(target_arch = "x86_64") {
                "x64"
            } else if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "x86"
            };
            let base = root.join("node").join(ver).join(arch);
            if cfg!(unix) {
                base.join("bin")
            } else {
                base
            }
        }
        NodeVersionManager::Nvm => {
            let nvm_dir = get_nvm_unix_dir()?;
            nvm_dir
                .join("versions")
                .join("node")
                .join(format!("v{}", ver))
                .join("bin")
        }
        NodeVersionManager::Builtin => {
            return crate::node_manager::get_bin_dir(ver);
        }
        // nvmd 通过 shim 自动处理，无需 PATH 注入
        NodeVersionManager::Nvmd | NodeVersionManager::None => return None,
    };

    if dir.is_dir() {
        Some(dir)
    } else {
        None
    }
}

/// 读取 nvmd 配置中的版本存储目录（`~/.nvmd/setting.json` 的 `directory` 字段）
fn get_nvmd_versions_dir() -> Option<PathBuf> {
    let home = std::env::var(if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    })
    .ok()?;
    let content =
        std::fs::read_to_string(Path::new(&home).join(".nvmd").join("setting.json")).ok()?;
    let settings: serde_json::Value = serde_json::from_str(&content).ok()?;
    Some(PathBuf::from(settings.get("directory")?.as_str()?))
}

/// nvm-windows 用 `v{ver}/` 目录，nvmd 用 `{ver}/` 目录。
/// 当版本通过 nvm-windows 安装后，创建目录 junction 让 nvmd 也能找到。
/// 在 macOS/Linux 上，如果通过 Unix nvm 安装，则从 nvm 安装路径创建 symlink。
fn bridge_nvm_to_nvmd(ver: &str) -> bool {
    let base = match get_nvmd_versions_dir() {
        Some(d) => d,
        None => return false,
    };
    let nvmd_dir = base.join(ver);

    if nvmd_dir.exists() {
        return false;
    }

    // 先在 nvmd 目录下查找 v-前缀 的同名目录（nvm-windows 风格）
    let nvm_dir_in_base = base.join(format!("v{}", ver));
    if nvm_dir_in_base.exists() {
        return create_dir_link(&nvmd_dir, &nvm_dir_in_base);
    }

    // Unix nvm 的安装路径：~/.nvm/versions/node/v{ver}/
    if let Some(nvm_unix) = get_nvm_unix_dir() {
        let nvm_node_dir = nvm_unix
            .join("versions")
            .join("node")
            .join(format!("v{}", ver));
        if nvm_node_dir.exists() {
            return create_dir_link(&nvmd_dir, &nvm_node_dir);
        }
    }

    false
}

/// 创建目录链接：Windows 用 junction，Unix 用 symlink
fn create_dir_link(link: &Path, target: &Path) -> bool {
    if cfg!(target_os = "windows") {
        new_cmd()
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &target.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }
}

/// 通过版本管理器安装指定版本的 Node.js
/// nvmd CLI 不支持 install，在 Windows 下 fallback 到 nvm-windows，
/// 安装完成后自动创建目录 junction 桥接 nvm-windows → nvmd 格式。
pub fn install_node_version(version: &str, manager: &NodeVersionManager) -> Result<String, String> {
    if *manager == NodeVersionManager::Builtin {
        return crate::node_manager::install_version(version);
    }

    let ver = version.trim_start_matches('v');
    let via_nvm_for_nvmd = *manager == NodeVersionManager::Nvmd;

    let result = match manager {
        NodeVersionManager::Nvmd => {
            if cfg!(target_os = "windows") && is_nvm_windows_available() {
                let mut cmd = new_cmd();
                cmd.args(["/C", "nvm", "install", ver]);
                output_with_timeout(cmd, 120)
            } else if cfg!(target_os = "windows") {
                return Err(format!(
                    "nvmd 不支持命令行安装，且未检测到 nvm-windows 作为备选。请在 nvm-desktop 中安装 Node.js {}",
                    ver
                ));
            } else if is_unix_nvm_installed() {
                let script = format!(
                    r#"export NVM_DIR="${{NVM_DIR:-$HOME/.nvm}}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm install {}"#,
                    ver
                );
                let mut cmd = Command::new("bash");
                cmd.args(["-c", &script]);
                output_with_timeout(cmd, 120)
            } else {
                return Err(format!(
                    "nvmd 不支持命令行安装 Node.js {}，且未检测到 nvm 作为备选。请在 nvm-desktop 桌面应用中下载安装",
                    ver
                ));
            }
        }
        NodeVersionManager::Nvs => {
            let node_ver = format!("node/{}", ver);
            if cfg!(target_os = "windows") {
                new_cmd()
                    .args(["/C", "nvs", "add", &node_ver])
                    .output()
                    .map_err(|e| e.to_string())
            } else {
                Command::new("nvs")
                    .args(["add", &node_ver])
                    .output()
                    .map_err(|e| e.to_string())
            }
        }
        NodeVersionManager::NvmWindows => {
            let mut cmd = new_cmd();
            cmd.args(["/C", "nvm", "install", ver]);
            output_with_timeout(cmd, 120)
        }
        NodeVersionManager::Nvm => {
            let script = format!(
                r#"export NVM_DIR="${{NVM_DIR:-$HOME/.nvm}}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm install {}"#,
                ver
            );
            let mut cmd = Command::new("bash");
            cmd.args(["-c", &script]);
            output_with_timeout(cmd, 120)
        }
        NodeVersionManager::Builtin | NodeVersionManager::None => {
            return Err("未检测到 Node 版本管理器".to_string())
        }
    };

    match result {
        Ok(o) => {
            let combined = collect_output(&o);
            if o.status.success() && !nvmd_output_has_error(&combined) {
                if via_nvm_for_nvmd {
                    bridge_nvm_to_nvmd(ver);
                }
                Ok(combined.trim().to_string())
            } else {
                Err(format!("安装失败: {}", combined.trim()))
            }
        }
        Err(e) => Err(format!("执行安装命令失败: {}", e)),
    }
}

/// 通过 `node --version` 验证切换是否真正生效（用于 nvm-windows 等不完全可信的管理器）
fn verify_switch(expected: &str, output: String) -> Result<String, String> {
    match get_current_node_version() {
        Some(actual) if actual == expected => Ok(output),
        Some(actual) => Err(format!(
            "切换命令已执行，但当前 Node.js 版本仍为 v{}，未成功切换到 v{}。\
            可能原因：nvm-windows 需要管理员权限，或与 nvmd 存在冲突。",
            actual, expected
        )),
        None => Ok(output),
    }
}

/// 通过版本管理器切换到指定已安装版本的 Node.js
/// nvmd use 的 exit code 不可信（成功/失败都返回 0），必须解析输出文本。
/// nvmd 的错误通过关键词匹配判断，成功时信任其 CLI 输出。
/// nvm-windows / nvs 切换后会通过 `node --version` 二次验证。
pub fn switch_node_version(version: &str, manager: &NodeVersionManager) -> Result<String, String> {
    if *manager == NodeVersionManager::Builtin {
        return crate::node_manager::switch_version(version);
    }

    let ver = version.trim_start_matches('v');

    match manager {
        NodeVersionManager::Nvmd => {
            let nvmd_out = if cfg!(target_os = "windows") {
                new_cmd().args(["/C", "nvmd", "use", ver]).output()
            } else {
                Command::new("nvmd").args(["use", ver]).output()
            };
            match nvmd_out {
                Ok(ref o) => {
                    let text = collect_output(o);
                    if text.to_lowercase().contains("now using") {
                        return Ok(text.trim().to_string());
                    }
                    if nvmd_output_has_error(&text) {
                        if bridge_nvm_to_nvmd(ver) {
                            let retry = if cfg!(target_os = "windows") {
                                new_cmd().args(["/C", "nvmd", "use", ver]).output()
                            } else {
                                Command::new("nvmd").args(["use", ver]).output()
                            };
                            if let Ok(ref ro) = retry {
                                let rt = collect_output(ro);
                                if rt.to_lowercase().contains("now using") {
                                    return Ok(rt.trim().to_string());
                                }
                            }
                        }
                        return Err(format!(
                            "该版本未在 nvmd 中安装，请在 nvm-desktop 中安装 Node.js {} 后再切换",
                            ver
                        ));
                    }
                    Ok(text.trim().to_string())
                }
                Err(e) => Err(format!("执行切换命令失败: {}", e)),
            }
        }
        NodeVersionManager::Nvs => {
            let node_ver = format!("node/{}", ver);
            let result = if cfg!(target_os = "windows") {
                new_cmd()
                    .args(["/C", "nvs", "use", &node_ver])
                    .output()
                    .map_err(|e| e.to_string())
            } else {
                Command::new("nvs")
                    .args(["use", &node_ver])
                    .output()
                    .map_err(|e| e.to_string())
            };
            match result {
                Ok(o) => {
                    let combined = collect_output(&o);
                    if o.status.success() && !nvmd_output_has_error(&combined) {
                        return verify_switch(ver, combined.trim().to_string());
                    }
                    Err(format!("切换失败: {}", combined.trim()))
                }
                Err(e) => Err(format!("执行切换命令失败: {}", e)),
            }
        }
        NodeVersionManager::NvmWindows => {
            let mut cmd = new_cmd();
            cmd.args(["/C", "nvm", "use", ver]);
            match output_with_timeout(cmd, CMD_TIMEOUT_SECS) {
                Ok(o) => {
                    let combined = collect_output(&o);
                    let lower = combined.to_lowercase();
                    if lower.contains("now using") || o.status.success() {
                        return verify_switch(ver, combined.trim().to_string());
                    }
                    Err(format!("切换失败: {}", combined.trim()))
                }
                Err(e) => Err(e),
            }
        }
        NodeVersionManager::Nvm => {
            let script = format!(
                r#"export NVM_DIR="${{NVM_DIR:-$HOME/.nvm}}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm alias default {} && nvm use {}"#,
                ver, ver
            );
            let mut cmd = Command::new("bash");
            cmd.args(["-c", &script]);
            match output_with_timeout(cmd, CMD_TIMEOUT_SECS) {
                Ok(o) => {
                    let combined = collect_output(&o);
                    if o.status.success() {
                        return verify_switch(ver, combined.trim().to_string());
                    }
                    Err(format!("切换失败: {}", combined.trim()))
                }
                Err(e) => Err(format!("执行切换命令失败: {}", e)),
            }
        }
        NodeVersionManager::Builtin | NodeVersionManager::None => {
            Err("未检测到 Node 版本管理器".to_string())
        }
    }
}

/// 通过版本管理器卸载指定已安装版本的 Node.js
/// nvmd CLI 不支持 uninstall，在 Windows 下 fallback 到 nvm-windows
pub fn uninstall_node_version(
    version: &str,
    manager: &NodeVersionManager,
) -> Result<String, String> {
    if *manager == NodeVersionManager::Builtin {
        return crate::node_manager::uninstall_version(version);
    }

    let ver = version.trim_start_matches('v');

    let result = match manager {
        NodeVersionManager::Nvmd => {
            if cfg!(target_os = "windows") && is_nvm_windows_available() {
                let mut cmd = new_cmd();
                cmd.args(["/C", "nvm", "uninstall", ver]);
                output_with_timeout(cmd, CMD_TIMEOUT_SECS)
            } else if cfg!(target_os = "windows") {
                return Err(format!(
                    "nvmd 不支持命令行卸载，且未检测到 nvm-windows 作为备选。请在 nvm-desktop 中卸载 Node.js {}",
                    ver
                ));
            } else if is_unix_nvm_installed() {
                // 先清理 nvmd 目录下的 symlink
                if let Some(base) = get_nvmd_versions_dir() {
                    let nvmd_dir = base.join(ver);
                    if nvmd_dir.is_symlink() {
                        let _ = std::fs::remove_file(&nvmd_dir);
                    }
                }
                let script = format!(
                    r#"export NVM_DIR="${{NVM_DIR:-$HOME/.nvm}}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm uninstall {}"#,
                    ver
                );
                let mut cmd = Command::new("bash");
                cmd.args(["-c", &script]);
                output_with_timeout(cmd, CMD_TIMEOUT_SECS)
            } else {
                return Err(format!(
                    "nvmd 不支持命令行卸载 Node.js {}，且未检测到 nvm 作为备选。请在 nvm-desktop 桌面应用中操作",
                    ver
                ));
            }
        }
        NodeVersionManager::Nvs => {
            let node_ver = format!("node/{}", ver);
            if cfg!(target_os = "windows") {
                new_cmd()
                    .args(["/C", "nvs", "remove", &node_ver])
                    .output()
                    .map_err(|e| e.to_string())
            } else {
                Command::new("nvs")
                    .args(["remove", &node_ver])
                    .output()
                    .map_err(|e| e.to_string())
            }
        }
        NodeVersionManager::NvmWindows => {
            let mut cmd = new_cmd();
            cmd.args(["/C", "nvm", "uninstall", ver]);
            output_with_timeout(cmd, CMD_TIMEOUT_SECS)
        }
        NodeVersionManager::Nvm => {
            let script = format!(
                r#"export NVM_DIR="${{NVM_DIR:-$HOME/.nvm}}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm uninstall {}"#,
                ver
            );
            let mut cmd = Command::new("bash");
            cmd.args(["-c", &script]);
            output_with_timeout(cmd, CMD_TIMEOUT_SECS)
        }
        NodeVersionManager::Builtin | NodeVersionManager::None => {
            return Err("未检测到 Node 版本管理器".to_string())
        }
    };

    match result {
        Ok(o) => {
            let combined = collect_output(&o);
            if o.status.success() && !nvmd_output_has_error(&combined) {
                Ok(combined.trim().to_string())
            } else {
                Err(format!("卸载失败: {}", combined.trim()))
            }
        }
        Err(e) => Err(format!("执行卸载命令失败: {}", e)),
    }
}

// ── Helper Trait：扩展 Command 的能力 ──

// trait 类似 TS 的 interface，定义一组方法签名
// 区别：Rust 的 trait 可以为已有类型添加方法（扩展方法），TS 的 interface 不行
// 这里为标准库的 Command 类型添加了 shell_spawn 方法
trait CommandShellSpawn {
    fn shell_spawn(&mut self) -> std::io::Result<std::process::Child>;
}

// impl Trait for Type：为已有类型实现 trait（类似 JS 的原型扩展，但更安全）
impl CommandShellSpawn for Command {
    /// Windows 上通过 cmd /C 间接执行命令
    /// 原因：某些通过 PATH 注册的命令（如 code、cursor），直接 spawn 找不到
    /// 必须通过 cmd.exe 的 PATH 解析才能找到
    fn shell_spawn(&mut self) -> std::io::Result<std::process::Child> {
        #[cfg(target_os = "windows")]
        {
            let prog = format!("{:?}", self.get_program());
            let args: Vec<String> = self
                .get_args()
                .map(|a| a.to_string_lossy().to_string())
                .collect();
            new_cmd()
                .arg("/C")
                .arg(prog.trim_matches('"'))
                .args(args)
                .spawn()
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.spawn()
        }
    }
}
