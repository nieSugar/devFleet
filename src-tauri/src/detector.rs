// 这个文件负责「检测」：包管理器、编辑器、Node 版本管理器
// 核心思路：通过检查 lock 文件是否存在、CLI 命令是否可用来判断用户安装了什么

use crate::models::{NodeVersion, NodeVersionManager, NvmInfo, PackageManager, RemoteNodeVersion};
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
    let mut cmd = Command::new("cmd");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
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

/// 通过执行 `cmd --version` 检测命令行工具是否可用
fn is_command_available(cmd: &str) -> bool {
    // cfg!() 是编译时宏，返回 bool
    // 注意：cfg!() 不会排除代码（两个分支都会编译），只是条件分支
    // 而 #[cfg()] 属性会真正排除代码（不编译另一个分支）
    let result = if cfg!(target_os = "windows") {
        new_cmd()
            .args(["/C", &format!("{} --version", cmd)])
            // Stdio::null() 丢弃输出，不需要看 --version 输出什么，只关心退出码
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    } else {
        // Unix 用 `command -v` 检查命令是否存在（比 which 更标准）
        Command::new("sh")
            .args(["-c", &format!("command -v {} >/dev/null 2>&1", cmd)])
            .status()
    };
    // matches!() 宏：模式匹配 + 布尔返回，相当于 match + true/false
    // Ok(s) if s.success() → 命令执行成功且退出码为 0
    matches!(result, Ok(s) if s.success())
}

/// macOS 特有：用 Spotlight 搜索检查 .app 是否安装
#[cfg(target_os = "macos")]
fn is_mac_app_installed(app_name: &str) -> bool {
    let result = Command::new("mdfind")
        .arg(format!(
            "kMDItemKind == \"Application\" && kMDItemDisplayName == \"{}\"",
            app_name
        ))
        .output();
    matches!(result, Ok(o) if o.status.success() && !o.stdout.is_empty())
}

/// 检测三种编辑器是否可用，返回 (vscode, cursor, webstorm) 布尔元组
/// 每个平台的检测方式不同：macOS 可以搜索 .app，Windows/Linux 只能检查 CLI
pub fn detect_editors() -> (bool, bool, bool) {
    #[cfg(target_os = "macos")]
    {
        let h1 = std::thread::spawn(|| {
            is_mac_app_installed("Visual Studio Code") || is_command_available("code")
        });
        let h2 =
            std::thread::spawn(|| is_mac_app_installed("Cursor") || is_command_available("cursor"));
        let h3 = std::thread::spawn(|| is_mac_app_installed("WebStorm"));

        let vscode = h1.join().unwrap_or(false);
        let cursor = h2.join().unwrap_or(false);
        let webstorm = h3.join().unwrap_or(false);
        return (vscode, cursor, webstorm);
    }

    #[cfg(target_os = "windows")]
    {
        let h1 = std::thread::spawn(|| is_command_available("code"));
        let h2 = std::thread::spawn(|| is_command_available("cursor"));
        let h3 = std::thread::spawn(|| {
            is_command_available("webstorm") || is_command_available("webstorm64")
        });

        let vscode = h1.join().unwrap_or(false);
        let cursor = h2.join().unwrap_or(false);
        let webstorm = h3.join().unwrap_or(false);
        return (vscode, cursor, webstorm);
    }

    #[cfg(target_os = "linux")]
    {
        let h1 = std::thread::spawn(|| is_command_available("code"));
        let h2 = std::thread::spawn(|| is_command_available("cursor"));
        let h3 = std::thread::spawn(|| {
            is_command_available("webstorm")
                || is_command_available("jetbrains-webstorm")
                || is_command_available("webstorm.sh")
        });

        let vscode = h1.join().unwrap_or(false);
        let cursor = h2.join().unwrap_or(false);
        let webstorm = h3.join().unwrap_or(false);
        return (vscode, cursor, webstorm);
    }

    // 理论上不会走到这里（上面三个平台已覆盖），但 Rust 要求所有分支都有返回值
    #[allow(unreachable_code)]
    (false, false, false)
}

/// 用指定编辑器打开项目目录
/// macOS 用 `open -a` 打开 .app，Windows/Linux 直接执行 CLI 命令
pub fn open_editor(editor: &str, project_path: &str) -> bool {
    match editor {
        "vscode" => {
            if cfg!(target_os = "macos") {
                detach_child(
                    Command::new("open")
                        .args(["-a", "Visual Studio Code", project_path])
                        .spawn(),
                )
            } else {
                detach_child(Command::new("code").arg(project_path).shell_spawn())
            }
        }
        "cursor" => {
            if cfg!(target_os = "macos") {
                detach_child(
                    Command::new("open")
                        .args(["-a", "Cursor", project_path])
                        .spawn(),
                )
            } else {
                detach_child(Command::new("cursor").arg(project_path).shell_spawn())
            }
        }
        "webstorm" => {
            if cfg!(target_os = "macos") {
                detach_child(
                    Command::new("open")
                        .args(["-a", "WebStorm", project_path])
                        .spawn(),
                )
            } else if cfg!(target_os = "windows") {
                for cmd in &["webstorm", "webstorm64", "webstorm.exe", "webstorm64.exe"] {
                    if is_command_available(cmd)
                        && detach_child(
                            Command::new(cmd).arg(project_path).shell_spawn(),
                        )
                    {
                        return true;
                    }
                }
                false
            } else {
                for cmd in &["webstorm", "jetbrains-webstorm", "webstorm.sh"] {
                    if is_command_available(cmd)
                        && detach_child(Command::new(cmd).arg(project_path).spawn())
                    {
                        return true;
                    }
                }
                false
            }
        }
        _ => false,
    }
}

// ── Node 版本管理器检测 ──

/// 检测 Unix 系统下 nvm 是否安装
/// nvm 是 shell 函数不是可执行文件，所以不能用 is_command_available 检测
/// 必须先 source nvm.sh 再检查 command -v nvm
fn is_unix_nvm_installed() -> bool {
    let result = Command::new("bash")
        .args([
            "-lc",
            // r#"..."# 是 Rust 的原始字符串语法，里面的 \ 和 " 不需要转义
            r#"export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && command -v nvm >/dev/null 2>&1"#,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    matches!(result, Ok(s) if s.success())
}

/// 检测系统中安装了哪种 Node 版本管理器
/// 优先级：nvmd > nvs > nvm/nvm-windows
pub fn detect_node_version_manager() -> NodeVersionManager {
    // nvmd（跨平台 GUI 版本管理器）
    if let Ok(r) = if cfg!(target_os = "windows") {
        new_cmd()
            .args(["/C", "nvmd", "--help"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    } else {
        Command::new("nvmd")
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    } {
        if r.success() {
            return NodeVersionManager::Nvmd;
        }
    }

    // nvs（跨平台）
    let nvs_result = if cfg!(target_os = "windows") {
        new_cmd()
            .args(["/C", "nvs", "--version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    } else {
        Command::new("nvs")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    };
    if matches!(nvs_result, Ok(s) if s.success()) {
        return NodeVersionManager::Nvs;
    }

    // nvm / nvm-windows
    if cfg!(target_os = "windows") {
        let r = new_cmd()
            .args(["/C", "nvm", "version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if matches!(r, Ok(s) if s.success()) {
            return NodeVersionManager::NvmWindows;
        }
    } else if is_unix_nvm_installed() {
        return NodeVersionManager::Nvm;
    }

    NodeVersionManager::None
}

/// 获取当前系统正在使用的 Node.js 版本号
pub fn get_current_node_version() -> Option<String> {
    let output = if cfg!(target_os = "windows") {
        new_cmd()
            .args(["/C", "node", "--version"])
            .output()
    } else {
        Command::new("node").arg("--version").output()
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
    let output = match manager {
        NodeVersionManager::Nvmd => {
            if cfg!(target_os = "windows") {
                new_cmd().args(["/C", "nvmd", "current"]).output()
            } else {
                Command::new("nvmd").arg("current").output()
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
    let current = get_current_version_by_manager(manager);

    // 不同版本管理器用不同的命令列出已安装版本
    let output = match manager {
        NodeVersionManager::Nvmd => {
            if cfg!(target_os = "windows") {
                new_cmd().args(["/C", "nvmd", "ls"]).output()
            } else {
                Command::new("nvmd").arg("ls").output()
            }
        }
        NodeVersionManager::Nvs => {
            if cfg!(target_os = "windows") {
                new_cmd().args(["/C", "nvs", "ls"]).output()
            } else {
                Command::new("nvs").arg("ls").output()
            }
        }
        NodeVersionManager::NvmWindows => {
            new_cmd().args(["/C", "nvm", "list"]).output()
        }
        NodeVersionManager::Nvm => {
            Command::new("bash")
                .args(["-lc", r#"export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"; [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm ls"#])
                .output()
        }
        NodeVersionManager::None => return vec![],
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
    let mut seen = std::collections::HashMap::new();

    for line in text.lines() {
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

const CMD_TIMEOUT_SECS: u64 = 30;

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
        if let Some(mut r) = stdout { let _ = r.read_to_end(&mut buf); }
        buf
    });
    let stderr_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = Vec::new();
        if let Some(mut r) = stderr { let _ = r.read_to_end(&mut buf); }
        buf
    });

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout_bytes = stdout_handle.join().unwrap_or_default();
                let stderr_bytes = stderr_handle.join().unwrap_or_default();
                return Ok(std::process::Output { status, stdout: stdout_bytes, stderr: stderr_bytes });
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
                Command::new("bash")
                    .args(["-lc", &script])
                    .output()
                    .map_err(|e| e.to_string())
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
            Command::new("bash")
                .args(["-lc", &script])
                .output()
                .map_err(|e| e.to_string())
        }
        NodeVersionManager::None => return Err("未检测到 Node 版本管理器".to_string()),
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
    let ver = version.trim_start_matches('v');

    match manager {
        NodeVersionManager::Nvmd => {
            let nvmd_out = if cfg!(target_os = "windows") {
                new_cmd()
                    .args(["/C", "nvmd", "use", ver])
                    .output()
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
                                new_cmd()
                                    .args(["/C", "nvmd", "use", ver])
                                    .output()
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
            match Command::new("bash").args(["-lc", &script]).output() {
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
        NodeVersionManager::None => Err("未检测到 Node 版本管理器".to_string()),
    }
}

/// 通过版本管理器卸载指定已安装版本的 Node.js
/// nvmd CLI 不支持 uninstall，在 Windows 下 fallback 到 nvm-windows
pub fn uninstall_node_version(
    version: &str,
    manager: &NodeVersionManager,
) -> Result<String, String> {
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
                Command::new("bash")
                    .args(["-lc", &script])
                    .output()
                    .map_err(|e| e.to_string())
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
            Command::new("bash")
                .args(["-lc", &script])
                .output()
                .map_err(|e| e.to_string())
        }
        NodeVersionManager::None => return Err("未检测到 Node 版本管理器".to_string()),
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
