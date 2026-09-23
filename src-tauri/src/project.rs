// 这个文件负责「项目」相关的业务逻辑：
// 读取 package.json 脚本、创建/添加/删除项目、管理 Node 版本文件

use crate::config;
use crate::detector;
use crate::models::{NodeVersionManager, NpmScript, Project};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_SCAN_DEPTH: usize = 3;
pub const MAX_SCAN_DIRECTORIES: usize = 5_000;
pub const MAX_SCAN_CANDIDATES: usize = 500;

static PROJECT_SCAN_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScanCandidate {
    pub path: String,
    pub name: String,
    pub package_manager: String,
    pub added: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScanResult {
    pub candidates: Vec<ProjectScanCandidate>,
    pub warnings: Vec<ProjectScanWarning>,
    pub truncated: bool,
    pub cancelled: bool,
    pub visited_directories: usize,
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScanWarning {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub fn begin_project_scan() -> u64 {
    PROJECT_SCAN_GENERATION.fetch_add(1, Ordering::AcqRel) + 1
}

pub fn cancel_project_scan() {
    PROJECT_SCAN_GENERATION.fetch_add(1, Ordering::AcqRel);
}

fn scan_cancelled(generation: u64) -> bool {
    PROJECT_SCAN_GENERATION.load(Ordering::Acquire) != generation
}

fn path_key(path: &Path) -> String {
    let key = path.to_string_lossy().replace('\\', "/");
    if cfg!(target_os = "windows") {
        key.to_ascii_lowercase()
    } else {
        key
    }
}

fn is_directory_link(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return true;
    };
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

fn read_package_object(project_path: &Path) -> Option<serde_json::Value> {
    let package_path = project_path.join("package.json");
    if is_directory_link(&package_path) {
        return None;
    }
    let content = fs::read_to_string(package_path).ok()?;
    let package = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    package.is_object().then_some(package)
}

pub fn has_valid_package_json(project_path: &str) -> bool {
    read_package_object(Path::new(project_path)).is_some()
}

pub fn scan_project_candidates(root_path: &str, generation: u64) -> ProjectScanResult {
    let mut result = ProjectScanResult {
        candidates: Vec::new(),
        warnings: Vec::new(),
        truncated: false,
        cancelled: false,
        visited_directories: 0,
    };
    let config = match config::load_checked() {
        Ok(config) => config,
        Err(error) => {
            push_warning(
                &mut result.warnings,
                "CONFIG_READ_FAILED",
                None,
                Some(error),
            );
            return result;
        }
    };
    let existing: HashSet<String> = config
        .projects
        .iter()
        .filter_map(|project| canonicalize_path(&project.path))
        .map(|path| path_key(Path::new(&path)))
        .collect();
    scan_project_candidates_with_existing(root_path, generation, existing)
}

fn scan_project_candidates_with_existing(
    root_path: &str,
    generation: u64,
    existing: HashSet<String>,
) -> ProjectScanResult {
    let mut result = ProjectScanResult {
        candidates: Vec::new(),
        warnings: Vec::new(),
        truncated: false,
        cancelled: false,
        visited_directories: 0,
    };

    let Some(root) = canonicalize_path(root_path).map(PathBuf::from) else {
        push_warning(&mut result.warnings, "ROOT_UNAVAILABLE", None, None);
        return result;
    };
    if !root.is_dir() || is_directory_link(Path::new(root_path)) {
        push_warning(
            &mut result.warnings,
            "ROOT_INVALID",
            Some(root_path.to_string()),
            None,
        );
        return result;
    }

    let mut seen = HashSet::new();
    let mut pending = vec![(root, 0usize)];

    while let Some((directory, depth)) = pending.pop() {
        if scan_cancelled(generation) {
            result.cancelled = true;
            break;
        }
        if result.visited_directories >= MAX_SCAN_DIRECTORIES {
            result.truncated = true;
            push_warning(
                &mut result.warnings,
                "DIRECTORY_LIMIT",
                None,
                Some(MAX_SCAN_DIRECTORIES.to_string()),
            );
            break;
        }
        result.visited_directories += 1;

        let directory_key = path_key(&directory);
        if !seen.insert(directory_key) {
            continue;
        }

        if read_package_object(&directory).is_some() {
            if result.candidates.len() >= MAX_SCAN_CANDIDATES {
                result.truncated = true;
                push_warning(
                    &mut result.warnings,
                    "CANDIDATE_LIMIT",
                    None,
                    Some(MAX_SCAN_CANDIDATES.to_string()),
                );
                break;
            }
            let key = path_key(&directory);
            result.candidates.push(ProjectScanCandidate {
                path: directory.to_string_lossy().to_string(),
                name: get_project_name(&directory.to_string_lossy()),
                package_manager: detector::detect_package_manager(&directory.to_string_lossy())
                    .to_string(),
                added: existing.contains(&key),
            });
            if result.candidates.len() == MAX_SCAN_CANDIDATES {
                result.truncated = true;
                push_warning(
                    &mut result.warnings,
                    "CANDIDATE_LIMIT",
                    None,
                    Some(MAX_SCAN_CANDIDATES.to_string()),
                );
                break;
            }
        }

        if depth >= MAX_SCAN_DEPTH {
            continue;
        }
        let Ok(entries) = fs::read_dir(&directory) else {
            push_warning(
                &mut result.warnings,
                "DIRECTORY_UNREADABLE",
                Some(directory.to_string_lossy().to_string()),
                None,
            );
            continue;
        };
        let mut directories: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                !matches!(
                    name.to_ascii_lowercase().as_str(),
                    ".git"
                        | "node_modules"
                        | "dist"
                        | "build"
                        | "out"
                        | ".next"
                        | ".nuxt"
                        | ".output"
                        | ".svelte-kit"
                        | ".astro"
                        | ".turbo"
                        | "coverage"
                        | "target"
                ) && !is_directory_link(path)
                    && path.is_dir()
            })
            .collect();
        directories.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        pending.extend(directories.into_iter().rev().map(|path| (path, depth + 1)));
    }

    if scan_cancelled(generation) {
        result.cancelled = true;
    }
    result
}

fn push_warning(
    warnings: &mut Vec<ProjectScanWarning>,
    code: &str,
    path: Option<String>,
    detail: Option<String>,
) {
    const MAX_WARNINGS: usize = 50;
    if warnings.len() < MAX_WARNINGS {
        warnings.push(ProjectScanWarning {
            code: code.to_string(),
            path,
            detail,
        });
    }
}

/// 读取项目 package.json 中的 scripts 字段，返回脚本列表
pub fn get_package_scripts(project_path: &str) -> Vec<NpmScript> {
    let pkg_path = Path::new(project_path).join("package.json");
    // match 处理 Result：Ok(c) 取值继续，Err(_) 提前返回空列表
    // 这是 Rust 的错误处理惯用模式（没有 try-catch，用 Result + match 代替）
    let content = match fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    // 把 JSON 字符串解析为动态类型 serde_json::Value（类似 JS 的 JSON.parse）
    let pkg: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    // 链式 Option 操作：.get() 取字段 → .and_then() 尝试转为 object
    // 如果任何一步返回 None，整个链就短路返回 None
    let scripts = match pkg.get("scripts").and_then(|s| s.as_object()) {
        Some(s) => s,
        None => return vec![],
    };

    // .iter() 遍历 HashMap，.map() 转换每个元素，.collect() 收集成 Vec
    // 这是 Rust 迭代器链（iterator chain），类似 JS 的 .map().filter() 但性能更好（零开销抽象）
    scripts
        .iter()
        .map(|(name, cmd)| NpmScript {
            name: name.clone(),
            command: cmd.as_str().unwrap_or("").to_string(),
        })
        .collect()
}

/// 规范化路径：解析符号链接、相对路径等，返回绝对路径
/// Windows 上 canonicalize 会返回 \\?\ 前缀的 UNC 路径，需要去掉
pub fn canonicalize_path(project_path: &str) -> Option<String> {
    fs::canonicalize(project_path).ok().map(|p| {
        let s = p.to_string_lossy().to_string();
        // Windows 的 canonicalize 结果如 "\\?\E:\github\devFleet"
        // strip_prefix 去掉 "\\?\" 前缀，让路径更正常
        s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
    })
}

/// 验证路径是否为有效的 Node.js 项目（目录存在且包含 package.json）
pub fn is_valid_path(project_path: &str) -> bool {
    let canonical = match canonicalize_path(project_path) {
        Some(c) => c,
        None => return false,
    };
    let p = Path::new(&canonical);
    p.is_dir() && p.join("package.json").exists()
}

/// 从路径中提取项目名（取最后一级目录名）
pub fn get_project_name(project_path: &str) -> String {
    Path::new(project_path)
        // file_name() 返回路径的最后一段，如 "/a/b/my-project" → "my-project"
        .file_name()
        // OsStr → String 的转换链：to_string_lossy 处理可能的非 UTF-8 字符
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| project_path.to_string())
}

/// 生成唯一 ID：时间戳 + 随机数的十六进制拼接
fn generate_id() -> String {
    use rand::Rng;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    // thread_rng() 获取当前线程的随机数生成器，.gen() 生成一个随机 u32
    let rand_part: u32 = rand::thread_rng().gen();
    // {:x} 是十六进制格式化
    format!("{:x}{:x}", ts, rand_part)
}

/// 根据项目路径创建完整的 Project 结构体
/// 自动检测脚本、包管理器、Node 版本等信息
pub fn create_project(project_path: &str) -> Option<Project> {
    // ? 操作符：Option 版本的提前返回
    // 如果 canonicalize_path 返回 None，整个函数立刻返回 None
    // 等价于 match ... { Some(v) => v, None => return None }
    let canonical = canonicalize_path(project_path)?;
    if !is_valid_path(&canonical) {
        return None;
    }

    let scripts = get_package_scripts(&canonical);
    // .first() 取 Vec 第一个元素（返回 Option），.map() 提取 name 字段
    let selected_script = scripts.first().map(|s| s.name.clone());
    let node_version = get_node_version(&canonical);

    // 构造结构体实例，所有字段必须赋值（Rust 没有"部分初始化"的概念）
    Some(Project {
        id: generate_id(),
        name: get_project_name(&canonical),
        path: canonical.clone(),
        scripts,
        selected_script,
        is_running: Some(false),
        last_run_time: None,
        package_manager: Some(detector::detect_package_manager(&canonical).to_string()),
        node_version,
        note: None,
    })
}

/// 添加项目到配置文件，路径已存在则返回 None（由调用方区分"重复"和"失败"）
/// 返回 Result：Ok(Project) 成功添加，Err(true) 路径已存在，Err(false) 其他失败
pub fn add_to_config(project_path: &str) -> Result<Project, bool> {
    if read_package_object(Path::new(project_path)).is_none() {
        return Err(false);
    }
    let project = match create_project(project_path) {
        Some(p) => p,
        None => return Err(false),
    };
    config::add_project_to_config(project.clone()).map(|()| project)
}

/// 从配置中删除指定 ID 的项目
pub fn remove_from_config(project_id: &str) -> bool {
    let mut config = config::load();
    let before = config.projects.len();
    config.projects.retain(|p| p.id != project_id);
    if config.projects.len() < before {
        config::save(&config)
    } else {
        false
    }
}

// ── Node 版本检测 ──

/// 从项目目录中的各种版本文件检测 Node 版本
/// 检测顺序：.nvmdrc → .node-version → .nvmrc → package.json engines.node
pub fn get_node_version(project_path: &str) -> Option<String> {
    let p = Path::new(project_path);

    // 依次尝试读取各种 Node 版本配置文件
    // if let Ok(v) = ... 是 match 的语法糖：只处理 Ok 分支，Err 跳过
    if let Ok(v) = fs::read_to_string(p.join(".nvmdrc")) {
        // trim() 去空白，trim_start_matches('v') 去掉可能的 "v" 前缀
        let v = v.trim().trim_start_matches('v');
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    if let Ok(v) = fs::read_to_string(p.join(".node-version")) {
        let v = v.trim().trim_start_matches('v');
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    if let Ok(v) = fs::read_to_string(p.join(".nvmrc")) {
        let v = v.trim().trim_start_matches('v');
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    // 保留完整需求，不能把 >=22.0.0 等范围误报成已选定 22.0.0。
    if let Ok(content) = fs::read_to_string(p.join("package.json")) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(node_ver) = pkg
                .get("engines")
                .and_then(|e| e.get("node"))
                .and_then(|n| n.as_str())
            {
                let requirement = node_ver.trim();
                if !requirement.is_empty() {
                    return Some(requirement.to_string());
                }
            }
        }
    }

    None
}

/// 设置或删除项目的 Node 版本文件
/// node_version = Some("18.17.0") → 创建/覆盖版本文件
/// node_version = None 或空字符串 → 删除版本文件
pub fn set_node_version_file(
    project_path: &str,
    node_version: Option<&str>,
    manager: &NodeVersionManager,
) -> bool {
    let file_name = match manager {
        NodeVersionManager::Builtin => ".node-version",
        NodeVersionManager::Nvmd => ".nvmdrc",
        NodeVersionManager::Nvs => ".node-version",
        _ => ".nvmrc",
    };

    let file_path = Path::new(project_path).join(file_name);

    match node_version {
        // 模式守卫：匹配 Some 且内容非空 → 写入版本文件
        Some(v) if !v.trim().is_empty() => {
            let content = v.trim_start_matches('v');
            fs::write(&file_path, content).is_ok()
        }
        // 其他情况（None 或空字符串）→ 删除版本文件
        _ => {
            if file_path.exists() {
                fs::remove_file(&file_path).is_ok()
            } else {
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn temp_root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be available")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("devfleet-scan-{label}-{suffix}"));
        fs::create_dir_all(&path).expect("temp root should be created");
        path
    }

    fn package(path: &Path, content: &str) {
        fs::create_dir_all(path).expect("project directory should be created");
        fs::write(path.join("package.json"), content).expect("package should be written");
    }

    #[test]
    fn scan_finds_valid_projects_and_skips_invalid_json() {
        let _guard = test_lock();
        let root = temp_root("valid");
        package(&root.join("app"), r#"{"name":"app","scripts":{}}"#);
        package(&root.join("broken"), "{not-json");
        let generation = begin_project_scan();
        let result = scan_project_candidates(root.to_str().unwrap(), generation);
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].name, "app");
        assert!(!result.candidates[0].added);
        fs::remove_dir_all(root).expect("owned temp root should be removable");
    }

    #[test]
    fn scan_stops_at_depth_limit() {
        let _guard = test_lock();
        let root = temp_root("depth");
        let nested = root.join("a").join("b").join("c").join("too-deep");
        package(&nested, r#"{"name":"too-deep"}"#);
        let generation = begin_project_scan();
        let result = scan_project_candidates(root.to_str().unwrap(), generation);
        assert!(result.candidates.is_empty());
        assert!(!result.truncated);
        fs::remove_dir_all(root).expect("owned temp root should be removable");
    }

    #[test]
    fn cancelled_scan_does_not_return_candidates() {
        let _guard = test_lock();
        let root = temp_root("cancel");
        package(&root.join("app"), r#"{"name":"app"}"#);
        let generation = begin_project_scan();
        cancel_project_scan();
        let result = scan_project_candidates(root.to_str().unwrap(), generation);
        assert!(result.cancelled);
        assert!(result.candidates.is_empty());
        fs::remove_dir_all(root).expect("owned temp root should be removable");
        let _ = begin_project_scan();
    }

    #[test]
    fn scan_includes_monorepo_projects_through_depth_three_only() {
        let _guard = test_lock();
        let root = temp_root("monorepo");
        package(&root, r#"{"name":"root"}"#);
        package(&root.join("packages").join("one"), r#"{"name":"one"}"#);
        package(
            &root.join("packages").join("one").join("nested"),
            r#"{"name":"nested"}"#,
        );
        package(
            &root
                .join("packages")
                .join("one")
                .join("nested")
                .join("deep"),
            r#"{"name":"deep"}"#,
        );
        package(
            &root
                .join("packages")
                .join("one")
                .join("nested")
                .join("deep")
                .join("too-deep"),
            r#"{"name":"too-deep"}"#,
        );
        let result = scan_project_candidates_with_existing(
            root.to_str().unwrap(),
            begin_project_scan(),
            HashSet::new(),
        );
        assert_eq!(result.candidates.len(), 3);
        assert!(!result
            .candidates
            .iter()
            .any(|candidate| candidate.name == "too-deep"));
        fs::remove_dir_all(root).expect("owned temp root should be removable");
    }

    #[test]
    fn scan_marks_existing_canonical_path_as_added() {
        let _guard = test_lock();
        let root = temp_root("existing");
        let app = root.join("app");
        package(&app, r#"{"name":"app"}"#);
        let existing = [path_key(Path::new(
            &canonicalize_path(app.to_str().unwrap()).unwrap(),
        ))]
        .into_iter()
        .collect();
        let result = scan_project_candidates_with_existing(
            root.to_str().unwrap(),
            begin_project_scan(),
            existing,
        );
        assert!(result.candidates.iter().any(|candidate| candidate.added));
        fs::remove_dir_all(root).expect("owned temp root should be removable");
    }

    #[test]
    fn scan_caps_candidates_at_five_hundred() {
        let _guard = test_lock();
        let root = temp_root("cap");
        for index in 0..501 {
            package(
                &root.join(format!("project-{index:03}")),
                r#"{"name":"app"}"#,
            );
        }
        let result = scan_project_candidates_with_existing(
            root.to_str().unwrap(),
            begin_project_scan(),
            HashSet::new(),
        );
        assert_eq!(result.candidates.len(), MAX_SCAN_CANDIDATES);
        assert!(result.truncated);
        fs::remove_dir_all(root).expect("owned temp root should be removable");
    }

    #[test]
    fn newer_scan_generation_invalidates_older_scan() {
        let _guard = test_lock();
        let root = temp_root("generation");
        package(&root.join("app"), r#"{"name":"app"}"#);
        let old = begin_project_scan();
        let _new = begin_project_scan();
        let result =
            scan_project_candidates_with_existing(root.to_str().unwrap(), old, HashSet::new());
        assert!(result.cancelled);
        assert!(result.candidates.is_empty());
        fs::remove_dir_all(root).expect("owned temp root should be removable");
    }

    #[cfg(unix)]
    #[test]
    fn scan_does_not_follow_symlinked_directory() {
        use std::os::unix::fs::symlink;
        let _guard = test_lock();
        let root = temp_root("symlink");
        let outside = temp_root("symlink-target");
        package(&outside.join("outside"), r#"{"name":"outside"}"#);
        symlink(&outside, root.join("linked")).expect("symlink should be available");
        let result = scan_project_candidates_with_existing(
            root.to_str().unwrap(),
            begin_project_scan(),
            HashSet::new(),
        );
        assert!(result.candidates.is_empty());
        fs::remove_dir_all(root).expect("owned temp root should be removable");
        fs::remove_dir_all(outside).expect("owned temp target should be removable");
    }
}
