use crate::config;
use crate::models::NodeVersion;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const DEFAULT_NODE_DIST_BASE: &str = "https://nodejs.org/dist";

static HTTP_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(120))
        .build()
});

/// 获取 Node 安装根目录。优先读用户配置，无配置时使用平台默认路径。
pub fn get_install_dir() -> PathBuf {
    if let Some(custom) = config::load_node_install_dir() {
        let p = PathBuf::from(&custom);
        if !p.exists() {
            let _ = fs::create_dir_all(&p);
        }
        return p;
    }
    default_install_dir()
}

/// 平台默认安装目录
fn default_install_dir() -> PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")))
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    };

    let dir = base.join("devfleet").join("node");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// 构造 Node 二进制的下载 URL
fn build_download_url(version: &str, mirror: Option<&str>) -> (String, String) {
    let base = mirror.unwrap_or(DEFAULT_NODE_DIST_BASE).trim_end_matches('/');
    let ver = version.trim().trim_start_matches('v');

    let (os, arch, ext) = platform_triple();
    let filename = format!("node-v{}-{}-{}.{}", ver, os, arch, ext);
    let url = format!("{}/v{}/{}", base, ver, filename);
    (url, filename)
}

fn platform_triple() -> (&'static str, &'static str, &'static str) {
    let os = if cfg!(target_os = "windows") {
        "win"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86"
    };

    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };

    (os, arch, ext)
}

/// 获取指定版本在安装目录下的路径
fn version_dir(version: &str) -> PathBuf {
    let ver = version.trim().trim_start_matches('v');
    get_install_dir().join(format!("v{}", ver))
}

/// 获取指定版本的 Node 二进制目录（用于 PATH 注入）
pub fn get_bin_dir(version: &str) -> Option<PathBuf> {
    let ver = version.trim().trim_start_matches('v');
    let base = version_dir(ver);
    if !base.is_dir() {
        return None;
    }

    if cfg!(target_os = "windows") {
        if base.join("node.exe").exists() {
            return Some(base);
        }
    } else {
        let bin = base.join("bin");
        if bin.join("node").exists() {
            return Some(bin);
        }
    }
    None
}

/// 扫描安装目录，返回已安装版本列表
pub fn list_installed_versions() -> Vec<NodeVersion> {
    let root = get_install_dir();
    if !root.is_dir() {
        return vec![];
    }

    let current = get_current_version();

    static VERSION_RE: LazyLock<regex_lite::Regex> =
        LazyLock::new(|| regex_lite::Regex::new(r"^v(\d+\.\d+\.\d+)$").unwrap());

    let mut versions: Vec<NodeVersion> = fs::read_dir(&root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let caps = VERSION_RE.captures(&name)?;
            let ver = caps[1].to_string();

            let has_node = if cfg!(target_os = "windows") {
                entry.path().join("node.exe").exists()
            } else {
                entry.path().join("bin").join("node").exists()
            };
            if !has_node {
                return None;
            }

            let is_current = current.as_deref() == Some(ver.as_str());
            Some(NodeVersion {
                full_version: format!("v{}", ver),
                version: ver,
                path: Some(entry.path().to_string_lossy().to_string()),
                is_current: Some(is_current),
            })
        })
        .collect();

    versions.sort_by(|a, b| {
        let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };
        parse(&b.version).cmp(&parse(&a.version))
    });
    versions
}

/// 从 devfleet 配置中读取 builtin 管理器的"当前版本"
fn get_current_version() -> Option<String> {
    config::load_builtin_current_version()
}

/// 下载并安装指定版本的 Node.js
pub fn install_version(version: &str) -> Result<String, String> {
    let ver = version.trim().trim_start_matches('v');
    let dest = version_dir(ver);

    if dest.is_dir() && get_bin_dir(ver).is_some() {
        return Err(format!("Node.js v{} 已安装", ver));
    }

    let mirror = config::load_node_mirror();
    let (url, _filename) = build_download_url(ver, mirror.as_deref());

    eprintln!("[devfleet] 下载 Node.js v{} from {}", ver, url);

    let resp = HTTP_AGENT
        .get(&url)
        .call()
        .map_err(|e| format!("下载失败: {}", e))?;

    let content_length = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = Vec::with_capacity(if content_length > 0 { content_length } else { 32 * 1024 * 1024 });
    resp.into_reader()
        .take(512 * 1024 * 1024) // 512MB 上限
        .read_to_end(&mut body)
        .map_err(|e| format!("读取下载数据失败: {}", e))?;

    if body.len() < 1024 {
        return Err("下载数据异常（文件过小），请检查版本号或镜像地址".to_string());
    }

    let tmp_dir = get_install_dir().join(format!(".tmp-v{}", ver));
    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    let extract_result = if cfg!(target_os = "windows") {
        extract_zip(&body, &tmp_dir)
    } else {
        extract_tar_gz(&body, &tmp_dir)
    };

    if let Err(e) = extract_result {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(format!("解压失败: {}", e));
    }

    // 归整目录结构：解压后通常有一层 node-vX.X.X-os-arch/ 子目录，需要提升到版本根
    let inner = find_single_subdir(&tmp_dir);
    if dest.exists() {
        let _ = fs::remove_dir_all(&dest);
    }

    let source = inner.unwrap_or(tmp_dir.clone());
    fs::rename(&source, &dest).or_else(|_| {
        copy_dir_recursive(&source, &dest).map_err(|e| format!("移动安装目录失败: {}", e))
    }).map_err(|e| format!("{}", e))?;

    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    if get_bin_dir(ver).is_none() {
        let _ = fs::remove_dir_all(&dest);
        return Err("安装后未找到 Node 可执行文件，可能架构不匹配".to_string());
    }

    // 如果没有当前版本（首次安装），自动设为当前版本并创建 current link
    if get_current_version().is_none() {
        config::save_builtin_current_version(Some(ver));
        update_current_link(ver);
    }

    Ok(format!("Node.js v{} 安装成功（{}）", ver, dest.display()))
}

/// 切换 builtin 管理器的"当前版本"，同时更新 current 链接
pub fn switch_version(version: &str) -> Result<String, String> {
    let ver = version.trim().trim_start_matches('v');
    if get_bin_dir(ver).is_none() {
        return Err(format!("Node.js v{} 未安装", ver));
    }
    config::save_builtin_current_version(Some(ver));
    update_current_link(ver);
    Ok(format!("已切换到 Node.js v{}", ver))
}

/// 获取 current 链接应该指向的 bin 目录路径（用于加入 PATH）
pub fn get_current_bin_path() -> Option<PathBuf> {
    let link = get_install_dir().join("current");
    if !link.exists() {
        return None;
    }
    if cfg!(target_os = "windows") {
        if link.join("node.exe").exists() {
            return Some(link);
        }
    } else {
        let bin = link.join("bin");
        if bin.join("node").exists() {
            return Some(bin);
        }
    }
    None
}

/// 创建/更新 current 链接指向指定版本目录
fn update_current_link(ver: &str) {
    let root = get_install_dir();
    let link_path = root.join("current");
    let target = root.join(format!("v{}", ver));

    if !target.is_dir() {
        return;
    }

    // 先删除旧链接
    if link_path.exists() || link_path.is_symlink() {
        remove_link_or_dir(&link_path);
    }

    create_dir_link(&link_path, &target);
}

fn remove_link_or_dir(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        // junction 在 Windows 上要用 rmdir 删除，不能 remove_dir_all（会删目标内容）
        let _ = std::process::Command::new("cmd")
            .args(["/C", "rmdir", &path.to_string_lossy()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = fs::remove_file(path);
    }
}

fn create_dir_link(link: &Path, target: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args([
                "/C", "mklink", "/J",
                &link.to_string_lossy(),
                &target.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::os::unix::fs::symlink(target, link);
    }
}

/// 将 current bin 目录添加到用户 PATH 环境变量
pub fn add_to_system_path() -> Result<String, String> {
    let bin_path = get_current_bin_path()
        .ok_or_else(|| "当前没有已选中的 Node 版本，请先切换一个版本".to_string())?;
    let bin_str = bin_path.to_string_lossy().to_string();

    if is_in_system_path(&bin_str) {
        return Ok(format!("{} 已在系统 PATH 中", bin_str));
    }

    add_to_user_path(&bin_str)?;

    Ok(format!(
        "已将 {} 添加到用户 PATH。请重新打开终端使其生效。",
        bin_str
    ))
}

/// 检查指定路径是否已在当前 PATH 中
pub fn is_path_configured(dir: &str) -> bool {
    is_in_system_path(dir)
}

fn is_in_system_path(dir: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        let sep = if cfg!(target_os = "windows") { ';' } else { ':' };
        let target = dir.trim_end_matches(['/', '\\']);
        for entry in path_var.split(sep) {
            let entry = entry.trim_end_matches(['/', '\\']);
            if cfg!(target_os = "windows") {
                if entry.eq_ignore_ascii_case(target) {
                    return true;
                }
            } else if entry == target {
                return true;
            }
        }
    }
    false
}

#[cfg(target_os = "windows")]
fn add_to_user_path(dir: &str) -> Result<(), String> {
    // 通过 PowerShell 读取和修改用户级 PATH（不影响系统级，不需要管理员权限）
    let read_cmd = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "[Environment]::GetEnvironmentVariable('Path', 'User')",
        ])
        .output()
        .map_err(|e| format!("读取 PATH 失败: {}", e))?;

    let current = String::from_utf8_lossy(&read_cmd.stdout).trim().to_string();

    let new_path = if current.is_empty() {
        dir.to_string()
    } else {
        format!("{};{}", dir, current)
    };

    let set_cmd = format!(
        "[Environment]::SetEnvironmentVariable('Path', '{}', 'User')",
        new_path.replace('\'', "''")
    );

    let result = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &set_cmd])
        .output()
        .map_err(|e| format!("设置 PATH 失败: {}", e))?;

    if !result.status.success() {
        let err = String::from_utf8_lossy(&result.stderr);
        return Err(format!("设置 PATH 失败: {}", err));
    }

    // 广播 WM_SETTINGCHANGE 让其他程序感知 PATH 变化
    let _ = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "Add-Type -Namespace Win32 -Name NativeMethods -MemberDefinition '[DllImport(\"user32.dll\", SetLastError = true, CharSet = CharSet.Auto)]public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);'; $HWND_BROADCAST = [IntPtr]0xffff; $WM_SETTINGCHANGE = 0x1a; $result = [UIntPtr]::Zero; [Win32.NativeMethods]::SendMessageTimeout($HWND_BROADCAST, $WM_SETTINGCHANGE, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result) | Out-Null",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    Ok(())
}

#[cfg(target_os = "macos")]
fn add_to_user_path(dir: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "无法获取 HOME 目录".to_string())?;
    let export_line = format!("export PATH=\"{}:$PATH\"", dir);

    // 按优先级选择 shell 配置文件
    let rc_file = if Path::new(&format!("{}/.zshrc", home)).exists() {
        format!("{}/.zshrc", home)
    } else {
        format!("{}/.bash_profile", home)
    };

    let content = fs::read_to_string(&rc_file).unwrap_or_default();
    if content.contains(dir) {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_file)
        .map_err(|e| format!("写入 {} 失败: {}", rc_file, e))?;

    use std::io::Write;
    writeln!(file, "\n# devFleet Node.js\n{}", export_line)
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn add_to_user_path(dir: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "无法获取 HOME 目录".to_string())?;
    let export_line = format!("export PATH=\"{}:$PATH\"", dir);

    let rc_file = if Path::new(&format!("{}/.bashrc", home)).exists() {
        format!("{}/.bashrc", home)
    } else {
        format!("{}/.profile", home)
    };

    let content = fs::read_to_string(&rc_file).unwrap_or_default();
    if content.contains(dir) {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_file)
        .map_err(|e| format!("写入 {} 失败: {}", rc_file, e))?;

    use std::io::Write;
    writeln!(file, "\n# devFleet Node.js\n{}", export_line)
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(())
}

/// 卸载指定版本
pub fn uninstall_version(version: &str) -> Result<String, String> {
    let ver = version.trim().trim_start_matches('v');
    let dir = version_dir(ver);
    if !dir.exists() {
        return Err(format!("Node.js v{} 未安装", ver));
    }

    if let Some(current) = get_current_version() {
        if current == ver {
            config::save_builtin_current_version(None::<&str>);
        }
    }

    fs::remove_dir_all(&dir).map_err(|e| format!("删除失败: {}", e))?;
    Ok(format!("Node.js v{} 已卸载", ver))
}

// ── 解压实现 ──

#[cfg(target_os = "windows")]
fn extract_zip(data: &[u8], dest: &Path) -> Result<(), String> {
    use std::io::Cursor;
    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("打开 zip 失败: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取 zip entry 失败: {}", e))?;

        let out_path = dest.join(
            file.enclosed_name()
                .ok_or_else(|| "zip 包含不安全路径".to_string())?,
        );

        if file.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("创建目录失败: {}", e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("创建父目录失败: {}", e))?;
            }
            let mut out_file = fs::File::create(&out_path)
                .map_err(|e| format!("创建文件失败: {}", e))?;
            io::copy(&mut file, &mut out_file)
                .map_err(|e| format!("写入文件失败: {}", e))?;
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn extract_zip(_data: &[u8], _dest: &Path) -> Result<(), String> {
    Err("zip 解压仅在 Windows 上使用".to_string())
}

#[cfg(not(target_os = "windows"))]
fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<(), String> {
    use flate2::read::GzDecoder;
    let gz = GzDecoder::new(data);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(dest)
        .map_err(|e| format!("解压 tar.gz 失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn extract_tar_gz(_data: &[u8], _dest: &Path) -> Result<(), String> {
    Err("tar.gz 解压仅在 Unix 上使用".to_string())
}

// ── 辅助函数 ──

/// 如果目录下只有一个子目录（解压后的典型结构），返回它的路径
fn find_single_subdir(dir: &Path) -> Option<PathBuf> {
    let entries: Vec<_> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().is_some_and(|ft| ft.is_dir()))
        .collect();

    if entries.len() == 1 {
        Some(entries[0].path())
    } else {
        None
    }
}

/// 递归复制目录（跨盘时 rename 会失败，fallback 用复制）
fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
