// 这个文件负责「配置持久化」：把项目列表保存到本地 JSON 文件
// 存储位置按操作系统标准：
//   Windows: %APPDATA%\devfleet\devfleet-config.json
//   macOS:   ~/Library/Application Support/devfleet/devfleet-config.json
//   Linux:   ~/.local/share/devfleet/devfleet-config.json

use crate::models::{AppSettings, EditorCache, ProjectConfig};
use crate::project;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// const 编译时常量，类型 &str（字符串切片）
const CONFIG_FILE: &str = "devfleet-config.json";

/// 获取配置文件的完整路径，如果目录不存在会自动创建
pub fn get_config_path() -> PathBuf {
    // cfg!() 宏在编译时求值，用于跨平台路径选择
    let base = if cfg!(target_os = "windows") {
        // Windows: 优先用 APPDATA 环境变量，否则用 dirs 库获取
        // dirs 是第三方库，封装了各平台的标准目录获取（在 Cargo.toml 中声明依赖）
        std::env::var("APPDATA")
            .map(PathBuf::from)
            // unwrap_or_else 在 Err 时执行闭包提供备选值
            .unwrap_or_else(|_| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()
            .map(|h| h.join("Library/Application Support"))
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        // Linux 及其他 Unix 系统
        dirs::home_dir()
            .map(|h| h.join(".local/share"))
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let app_dir = base.join("devfleet");
    if !app_dir.exists() {
        // create_dir_all 递归创建目录（类似 mkdir -p）
        if let Err(e) = fs::create_dir_all(&app_dir) {
            // eprintln! 输出到 stderr（不影响正常输出），适合记录错误日志
            eprintln!("[devfleet] 创建配置目录失败: {}", e);
        }
    }
    app_dir.join(CONFIG_FILE)
}

/// 全局配置文件锁，序列化所有配置文件的读写操作，防止并发竞争
fn config_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// 原子写入：先写临时文件再重命名，防止进程崩溃时配置文件损坏
fn atomic_write(path: &PathBuf, content: &str) -> bool {
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, content).is_err() {
        eprintln!("[devfleet] 写入临时配置文件失败");
        return false;
    }
    if fs::rename(&tmp, path).is_err() {
        eprintln!("[devfleet] 重命名配置文件失败");
        let _ = fs::remove_file(&tmp);
        return false;
    }
    true
}

/// 内部加载逻辑（调用方需已持有 config_lock）
fn load_unlocked() -> ProjectConfig {
    let config_path = get_config_path();

    if !config_path.exists() {
        return default_config();
    }

    let data = match fs::read_to_string(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[devfleet] 读取配置文件失败: {}", e);
            return default_config();
        }
    };

    match serde_json::from_str(&data) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[devfleet] 解析配置文件失败: {}", e);
            default_config()
        }
    }
}

/// 内部保存逻辑（调用方需已持有 config_lock）
/// 如果调用方未提供 editors 字段，会从磁盘已有配置中保留
fn save_unlocked(config: &ProjectConfig) -> bool {
    let config_path = get_config_path();
    let mut cfg = config.clone();
    cfg.last_updated = chrono::Utc::now().to_rfc3339();

    if cfg.editors.is_none() || cfg.settings.is_none() {
        if let Ok(data) = fs::read_to_string(&config_path) {
            if let Ok(existing) = serde_json::from_str::<ProjectConfig>(&data) {
                if cfg.editors.is_none() {
                    cfg.editors = existing.editors;
                }
                if cfg.settings.is_none() {
                    cfg.settings = existing.settings;
                }
            }
        }
    }

    match serde_json::to_string_pretty(&cfg) {
        Ok(json) => atomic_write(&config_path, &json),
        Err(e) => {
            eprintln!("[devfleet] 序列化配置失败: {}", e);
            false
        }
    }
}

/// 创建默认的空配置
fn default_config() -> ProjectConfig {
    ProjectConfig {
        projects: vec![],
        last_updated: chrono::Utc::now().to_rfc3339(),
        editors: None,
        settings: None,
    }
}

/// 快速加载配置文件，直接反序列化 JSON，不做任何文件系统校验
pub fn load() -> ProjectConfig {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_unlocked()
}

/// 保存配置到文件，自动更新 last_updated 时间戳
pub fn save(config: &ProjectConfig) -> bool {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    save_unlocked(config)
}

/// 加载配置并刷新：校验项目路径、重读 scripts、补充检测缺失字段
/// 仅在用户主动刷新时调用
pub fn load_and_refresh() -> ProjectConfig {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut config = load_unlocked();

    config.projects.retain_mut(|p| {
        if project::is_valid_path(&p.path) {
            p.scripts = project::get_package_scripts(&p.path);
            if p.node_version.is_none() {
                p.node_version = project::get_node_version(&p.path);
            }
            true
        } else {
            false
        }
    });

    config
}

/// 从配置文件加载编辑器缓存
pub fn load_editor_cache() -> Option<EditorCache> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_unlocked().editors
}

/// 保存编辑器缓存到配置文件（load-modify-save 模式）
pub fn save_editor_cache(cache: &EditorCache) {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut cfg = load_unlocked();
    cfg.editors = Some(cache.clone());
    save_unlocked(&cfg);
}

/// 读取 Node 镜像地址，None 表示使用官方默认源
pub fn load_node_mirror() -> Option<String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_unlocked()
        .settings
        .and_then(|s| s.node_mirror)
        .filter(|m| !m.trim().is_empty())
}

/// 保存 Node 镜像地址（传入 None 则清除，回退到官方默认源）
pub fn save_node_mirror(mirror: Option<&str>) {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut cfg = load_unlocked();
    let settings = cfg.settings.get_or_insert(AppSettings { node_mirror: None });
    settings.node_mirror = mirror
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    save_unlocked(&cfg);
}
