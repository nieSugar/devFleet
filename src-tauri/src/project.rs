// 这个文件负责「项目」相关的业务逻辑：
// 读取 package.json 脚本、创建/添加/删除项目、管理 Node 版本文件

use crate::config;
use crate::detector;
use crate::models::{NodeVersionManager, NpmScript, Project};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

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
    let project = match create_project(project_path) {
        Some(p) => p,
        None => return Err(false),
    };
    let mut config = config::load();

    if config.projects.iter().any(|p| p.path == project.path) {
        return Err(true); // 路径已存在
    }

    config.projects.push(project.clone());
    config::save(&config);
    Ok(project)
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

    // 最后尝试从 package.json 的 engines.node 字段提取版本号
    if let Ok(content) = fs::read_to_string(p.join("package.json")) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(node_ver) = pkg
                .get("engines")
                .and_then(|e| e.get("node"))
                .and_then(|n| n.as_str())
            {
                static VERSION_RE: LazyLock<regex_lite::Regex> =
                    LazyLock::new(|| regex_lite::Regex::new(r"(\d+\.\d+\.\d+)").unwrap());
                static MAJOR_RE: LazyLock<regex_lite::Regex> =
                    LazyLock::new(|| regex_lite::Regex::new(r"(\d+)").unwrap());

                if let Some(caps) = VERSION_RE.captures(node_ver) {
                    return Some(caps[1].to_string());
                }
                if let Some(caps) = MAJOR_RE.captures(node_ver) {
                    return Some(caps[1].to_string());
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
