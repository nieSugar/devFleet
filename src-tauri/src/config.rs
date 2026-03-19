// 这个文件负责「配置持久化」：把项目列表保存到本地 JSON 文件
// 存储位置按操作系统标准：
//   Windows: %APPDATA%\devfleet\devfleet-config.json
//   macOS:   ~/Library/Application Support/devfleet/devfleet-config.json
//   Linux:   ~/.local/share/devfleet/devfleet-config.json

use crate::models::ProjectConfig;
use crate::project;
use std::fs;
use std::path::PathBuf;

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

/// 加载配置文件，同时验证每个项目路径是否仍然有效
/// 无效的项目（目录已删除/package.json 不存在）会被自动剔除
pub fn load() -> ProjectConfig {
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

    // serde_json::from_str 把 JSON 字符串反序列化为 Rust 结构体
    // 泛型 ::<ProjectConfig> 告诉编译器要转成什么类型
    let mut config: ProjectConfig = match serde_json::from_str(&data) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[devfleet] 解析配置文件失败: {}", e);
            return default_config();
        }
    };

    // retain_mut：保留满足条件的元素，同时允许修改元素（mut 版本）
    // 这里做了两件事：1) 过滤掉无效项目 2) 刷新有效项目的脚本列表和 Node 版本
    config.projects.retain_mut(|p| {
        if project::is_valid_path(&p.path) {
            // 每次加载时重新读取 scripts，确保跟 package.json 同步
            p.scripts = project::get_package_scripts(&p.path);
            if p.node_version.is_none() {
                p.node_version = project::get_node_version(&p.path);
            }
            true  // 保留
        } else {
            false // 剔除
        }
    });

    config
}

/// 保存配置到文件，自动更新 last_updated 时间戳
pub fn save(config: &ProjectConfig) -> bool {
    let config_path = get_config_path();
    // .clone() 深拷贝，避免修改传入的引用（Rust 的 &config 是只读借用）
    let mut cfg = config.clone();
    // chrono 库处理时间，Utc::now() 获取 UTC 时间，to_rfc3339() 转为标准时间字符串
    cfg.last_updated = chrono::Utc::now().to_rfc3339();

    match serde_json::to_string_pretty(&cfg) {
        // fs::write 一步到位写文件（创建或覆盖），.is_ok() 转成 bool
        Ok(json) => fs::write(&config_path, json).is_ok(),
        Err(_) => false,
    }
}

/// 创建默认的空配置
fn default_config() -> ProjectConfig {
    ProjectConfig {
        projects: vec![], // vec![] 宏创建空 Vec（类似 JS 的 []）
        last_updated: chrono::Utc::now().to_rfc3339(),
    }
}
