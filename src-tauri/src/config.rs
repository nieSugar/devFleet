// 这个文件负责「配置持久化」：把项目列表保存到本地 JSON 文件
// 存储位置按操作系统标准：
//   Windows: %APPDATA%\devfleet\devfleet-config.json
//   macOS:   ~/Library/Application Support/devfleet/devfleet-config.json
//   Linux:   ~/.local/share/devfleet/devfleet-config.json

use crate::detector;
use crate::models::Project;
use crate::models::{AppSettings, CustomEditor, EditorCache, ProjectConfig};
use crate::project;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// const 编译时常量，类型 &str（字符串切片）
const CONFIG_FILE: &str = "devfleet-config.json";
pub const EDITOR_CACHE_VERSION: u32 = 1;
const EDITOR_CACHE_TTL_HOURS: i64 = 24;

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
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content).map_err(|e| format!("写入临时配置文件失败: {}", e))?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("替换配置文件失败: {}", e));
    }
    Ok(())
}

fn load_strict_from(path: &Path) -> Result<ProjectConfig, String> {
    if !path.exists() {
        return Ok(default_config());
    }

    let data = fs::read_to_string(path).map_err(|e| format!("读取配置文件失败: {}", e))?;
    serde_json::from_str(&data).map_err(|e| format!("解析配置文件失败: {}", e))
}

fn load_from(path: &Path) -> ProjectConfig {
    match load_strict_from(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[devfleet] {}", e);
            default_config()
        }
    }
}

/// 内部加载逻辑（调用方需已持有 config_lock）
fn load_unlocked() -> ProjectConfig {
    load_from(&get_config_path())
}

fn write_config(path: &Path, config: &ProjectConfig) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("序列化配置失败: {}", e))?;
    atomic_write(path, &json)
}

fn save_project_to(path: &Path, config: &ProjectConfig) -> Result<(), String> {
    let mut cfg = config.clone();
    let existing = load_strict_from(path)?;

    cfg.editors = existing.editors;
    cfg.settings = existing.settings.map(|mut settings| {
        normalize_pinned_project_ids(&mut settings.pinned_project_ids, &cfg.projects);
        settings
    });
    cfg.editor_cache_version = existing.editor_cache_version;
    cfg.editor_cache_updated_at = existing.editor_cache_updated_at;
    cfg.last_updated = chrono::Utc::now().to_rfc3339();

    write_config(path, &cfg)
}

/// 内部普通项目保存逻辑（调用方需已持有 config_lock）
fn save_unlocked(config: &ProjectConfig) -> bool {
    match save_project_to(&get_config_path(), config) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[devfleet] 保存配置失败: {}", e);
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
        editor_cache_version: None,
        editor_cache_updated_at: None,
    }
}

pub(crate) fn normalize_pinned_project_ids(ids: &mut Vec<String>, projects: &[Project]) {
    let mut normalized = Vec::with_capacity(ids.len());
    for id in ids.drain(..) {
        if projects.iter().any(|project| project.id == id)
            && !normalized.iter().any(|seen| seen == &id)
        {
            normalized.push(id);
        }
    }
    *ids = normalized;
}

/// 快速加载配置文件，直接反序列化 JSON，不做任何文件系统校验
pub fn load() -> ProjectConfig {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_unlocked()
}

/// 严格读取最新配置，配置损坏或读取失败时返回真实错误。
pub fn load_checked() -> Result<ProjectConfig, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_strict_from(&get_config_path())
}

/// 保存配置到文件，自动更新 last_updated 时间戳
pub fn save(config: &ProjectConfig) -> bool {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    save_unlocked(config)
}

/// 在同一把配置锁内完成读取、去重和写入，避免并发添加互相覆盖。
pub fn add_project_to_config(project: Project) -> Result<(), bool> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = get_config_path();
    let mut config = load_strict_from(&path).map_err(|_| false)?;
    let comparable = comparable_project_path(&project.path);
    if config
        .projects
        .iter()
        .any(|item| same_project_path(&comparable_project_path(&item.path), &comparable))
    {
        return Err(true);
    }
    config.projects.push(project);
    save_project_to(&path, &config).map_err(|_| false)
}

fn load_and_refresh_at(path: &Path) -> Result<ProjectConfig, String> {
    let mut config = load_strict_from(path)?;
    for item in &mut config.projects {
        if project::is_valid_path(&item.path) {
            item.scripts = project::get_package_scripts(&item.path);
            item.node_version = project::get_node_version(&item.path);
            if item.package_manager.is_none() {
                item.package_manager =
                    Some(detector::detect_package_manager(&item.path).to_string());
            }
            if item
                .selected_script
                .as_ref()
                .is_some_and(|selected| !item.scripts.iter().any(|script| &script.name == selected))
            {
                item.selected_script = None;
            }
        }
    }
    config.last_updated = chrono::Utc::now().to_rfc3339();
    write_config(path, &config)?;
    Ok(config)
}

/// 加载配置并刷新项目元数据；失效路径项目仍保留，失败时不覆盖配置。
pub fn load_and_refresh() -> Result<ProjectConfig, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_and_refresh_at(&get_config_path())
}

fn same_project_path(left: &str, right: &str) -> bool {
    if cfg!(target_os = "windows") {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn comparable_project_path(path: &str) -> String {
    project::canonicalize_path(path).unwrap_or_else(|| path.to_string())
}

fn relocate_project_at(
    path: &Path,
    project_id: &str,
    project_path: &str,
) -> Result<Project, String> {
    let mut config = load_strict_from(path)?;
    let canonical = project::canonicalize_path(project_path)
        .ok_or_else(|| "项目路径无效或不存在".to_string())?;
    if !project::is_valid_path(&canonical) {
        return Err("项目路径无效：目录必须存在且包含 package.json".to_string());
    }
    let comparable = comparable_project_path(&canonical);
    if config.projects.iter().any(|item| {
        item.id != project_id
            && same_project_path(&comparable_project_path(&item.path), &comparable)
    }) {
        return Err("项目路径已被其他项目使用".to_string());
    }

    let item = config
        .projects
        .iter_mut()
        .find(|item| item.id == project_id)
        .ok_or_else(|| "项目不存在".to_string())?;
    let selected_script = item.selected_script.clone();
    item.path = canonical.clone();
    item.name = project::get_project_name(&canonical);
    item.scripts = project::get_package_scripts(&canonical);
    item.node_version = project::get_node_version(&canonical);
    item.package_manager = Some(detector::detect_package_manager(&canonical).to_string());
    item.selected_script = selected_script
        .filter(|selected| item.scripts.iter().any(|script| &script.name == selected));
    let relocated = item.clone();
    config.last_updated = chrono::Utc::now().to_rfc3339();
    write_config(path, &config)?;
    Ok(relocated)
}

/// 将项目定位到新的有效目录，并保留项目 ID、备注及运行状态等用户字段。
pub fn relocate_project(project_id: &str, project_path: &str) -> Result<Project, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    relocate_project_at(&get_config_path(), project_id, project_path)
}

fn update_project_at(
    path: &Path,
    project_id: &str,
    update: impl FnOnce(&mut Project) -> Result<(), String>,
) -> Result<Project, String> {
    let mut config = load_strict_from(path)?;
    let project = config
        .projects
        .iter_mut()
        .find(|project| project.id == project_id)
        .ok_or_else(|| "项目不存在".to_string())?;
    update(project)?;
    let updated = project.clone();
    config.last_updated = chrono::Utc::now().to_rfc3339();
    write_config(path, &config)?;
    Ok(updated)
}

/// 在同一配置锁内基于磁盘最新值更新项目，避免旧快照覆盖重定位结果。
pub fn update_project(
    project_id: &str,
    update: impl FnOnce(&mut Project) -> Result<(), String>,
) -> Result<Project, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    update_project_at(&get_config_path(), project_id, update)
}

fn set_project_pinned_at(
    path: &Path,
    project_id: &str,
    pinned: bool,
) -> Result<Vec<String>, String> {
    if project_id.trim().is_empty() {
        return Err("项目 ID 不能为空".to_string());
    }
    let mut config = load_strict_from(path)?;
    if !config
        .projects
        .iter()
        .any(|project| project.id == project_id)
    {
        return Err("项目不存在".to_string());
    }
    let settings = config.settings.get_or_insert_with(default_settings);
    let before_normalize = settings.pinned_project_ids.clone();
    normalize_pinned_project_ids(&mut settings.pinned_project_ids, &config.projects);
    if pinned {
        if !settings
            .pinned_project_ids
            .iter()
            .any(|id| id == project_id)
        {
            settings.pinned_project_ids.push(project_id.to_string());
        }
    } else {
        settings.pinned_project_ids.retain(|id| id != project_id);
    }
    let project_ids = settings.pinned_project_ids.clone();
    if settings.pinned_project_ids == before_normalize
        && ((pinned && project_ids.iter().any(|id| id == project_id))
            || (!pinned && !project_ids.iter().any(|id| id == project_id)))
    {
        return Ok(project_ids);
    }
    config.last_updated = chrono::Utc::now().to_rfc3339();
    write_config(path, &config)?;
    Ok(project_ids)
}

/// 更新项目置顶状态；项目不存在或 ID 为空时拒绝写入。
pub fn set_project_pinned(project_id: &str, pinned: bool) -> Result<Vec<String>, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    set_project_pinned_at(&get_config_path(), project_id, pinned)
}

fn load_editor_cache_from(path: &Path) -> Result<Option<EditorCache>, String> {
    let cfg = load_strict_from(path)?;
    if cfg.editor_cache_version != Some(EDITOR_CACHE_VERSION) {
        return Ok(None);
    }
    let Some(updated_at) = cfg
        .editor_cache_updated_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
    else {
        return Ok(None);
    };
    let age = chrono::Utc::now().signed_duration_since(updated_at.with_timezone(&chrono::Utc));
    if age < chrono::Duration::zero() || age >= chrono::Duration::hours(EDITOR_CACHE_TTL_HOURS) {
        return Ok(None);
    }
    let Some(cache) = cfg.editors else {
        return Ok(None);
    };
    if cache
        .values()
        .any(|editor| editor.installed && editor.launch.is_none())
    {
        return Ok(None);
    }
    Ok(Some(cache))
}

/// 仅返回版本、时间和启动描述均有效的编辑器缓存；旧缓存会触发重新扫描。
pub fn load_editor_cache() -> Result<Option<EditorCache>, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_editor_cache_from(&get_config_path())
}

fn save_editor_cache_to(path: &Path, cache: &EditorCache) -> Result<(), String> {
    let mut cfg = load_strict_from(path)?;
    let now = chrono::Utc::now().to_rfc3339();
    cfg.editors = Some(cache.clone());
    cfg.editor_cache_version = Some(EDITOR_CACHE_VERSION);
    cfg.editor_cache_updated_at = Some(now.clone());
    cfg.last_updated = now;
    write_config(path, &cfg)
}

/// 保存编辑器缓存到磁盘最新配置，损坏配置会拒绝覆盖
pub fn save_editor_cache(cache: &EditorCache) -> Result<(), String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    save_editor_cache_to(&get_config_path(), cache)
}

/// 从磁盘最新配置严格读取自定义编辑器
#[allow(dead_code)] // S04 的编辑器管理命令会调用；S01 先建立严格读取边界。
pub fn load_custom_editors() -> Result<Vec<CustomEditor>, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    Ok(load_strict_from(&get_config_path())?
        .settings
        .unwrap_or_default()
        .custom_editors)
}

fn update_custom_editors_at<T>(
    path: &Path,
    update: impl FnOnce(&mut Vec<CustomEditor>) -> Result<T, String>,
) -> Result<T, String> {
    let mut cfg = load_strict_from(path)?;
    let result = update(
        &mut cfg
            .settings
            .get_or_insert_with(default_settings)
            .custom_editors,
    )?;
    cfg.last_updated = chrono::Utc::now().to_rfc3339();
    write_config(path, &cfg)?;
    Ok(result)
}

/// 在同一配置锁内按磁盘最新值修改自定义编辑器
#[allow(dead_code)] // S04 的 CRUD 会调用；当前步骤只提供并验证原子 helper。
pub fn update_custom_editors<T>(
    update: impl FnOnce(&mut Vec<CustomEditor>) -> Result<T, String>,
) -> Result<T, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    update_custom_editors_at(&get_config_path(), update)
}

fn update_settings_at<T>(
    path: &Path,
    update: impl FnOnce(&mut AppSettings) -> T,
) -> Result<T, String> {
    let mut cfg = load_strict_from(path)?;
    let result = update(cfg.settings.get_or_insert_with(default_settings));
    cfg.last_updated = chrono::Utc::now().to_rfc3339();
    write_config(path, &cfg)?;
    Ok(result)
}

fn load_default_editor_id_at(path: &Path) -> Result<Option<String>, String> {
    Ok(load_strict_from(path)?
        .settings
        .and_then(|settings| settings.default_editor_id))
}

/// 读取全局默认编辑器 ID；未设置或旧配置返回 None。
pub fn load_default_editor_id() -> Result<Option<String>, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_default_editor_id_at(&get_config_path())
}

fn normalize_default_editor_id(id: Option<&str>) -> Result<Option<String>, String> {
    let Some(id) = id.map(str::trim).filter(|id| !id.is_empty()) else {
        return Ok(None);
    };
    if id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_:.".contains(&byte))
    {
        return Err(
            "编辑器 ID 无效：仅允许 ASCII 字母、数字、连字符、下划线、冒号和点，长度不超过 128"
                .to_string(),
        );
    }
    Ok(Some(id.to_string()))
}

fn save_default_editor_id_at(path: &Path, id: Option<&str>) -> Result<Option<String>, String> {
    let normalized = normalize_default_editor_id(id)?;
    update_settings_at(path, |settings| {
        settings.default_editor_id = normalized.clone();
        normalized
    })
}

/// 保存全局默认编辑器 ID；空白输入清除设置，不要求编辑器当前可用。
pub fn save_default_editor_id(id: Option<&str>) -> Result<Option<String>, String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    save_default_editor_id_at(&get_config_path(), id)
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
pub fn save_node_mirror(mirror: Option<&str>) -> Result<(), String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = get_config_path();
    update_settings_at(&path, |settings| {
        settings.node_mirror = mirror
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty());
    })
}

/// 读取 Node 安装目录，None 表示使用默认路径
pub fn load_node_install_dir() -> Option<String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_unlocked()
        .settings
        .and_then(|s| s.node_install_dir)
        .filter(|d| !d.trim().is_empty())
}

/// 保存 Node 安装目录（传入 None 恢复默认路径）
pub fn save_node_install_dir(dir: Option<&str>) -> Result<(), String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = get_config_path();
    update_settings_at(&path, |settings| {
        settings.node_install_dir = dir.map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
    })
}

/// 读取 builtin 管理器的当前版本
pub fn load_builtin_current_version() -> Option<String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_unlocked()
        .settings
        .and_then(|s| s.builtin_current_version)
        .filter(|v| !v.trim().is_empty())
}

/// 保存 builtin 管理器的当前版本
pub fn save_builtin_current_version(version: Option<impl Into<String>>) -> Result<(), String> {
    let _guard = config_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = get_config_path();
    update_settings_at(&path, |settings| {
        settings.builtin_current_version = version
            .map(|v| {
                let s: String = v.into();
                s.trim().trim_start_matches('v').to_string()
            })
            .filter(|v| !v.is_empty());
    })
}

fn default_settings() -> AppSettings {
    AppSettings::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EditorInfo, EditorLaunch, NpmScript, Project};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEST_FILE: AtomicUsize = AtomicUsize::new(0);

    struct TestConfigFile {
        path: PathBuf,
    }

    impl TestConfigFile {
        fn new(label: &str) -> Self {
            let id = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "devfleet-config-test-{}-{}-{}.json",
                std::process::id(),
                id,
                label
            ));
            let _ = fs::remove_file(&path);
            let _ = fs::remove_file(path.with_extension("json.tmp"));
            Self { path }
        }
    }

    impl Drop for TestConfigFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_file(self.path.with_extension("json.tmp"));
        }
    }

    fn project(id: &str) -> Project {
        Project {
            id: id.to_string(),
            name: id.to_string(),
            path: format!("/tmp/{}", id),
            scripts: vec![NpmScript {
                name: "dev".to_string(),
                command: "node index.js".to_string(),
            }],
            selected_script: Some("dev".to_string()),
            is_running: Some(false),
            last_run_time: None,
            package_manager: Some("npm".to_string()),
            node_version: None,
            note: None,
        }
    }

    fn custom_editor(id: &str) -> CustomEditor {
        CustomEditor {
            id: id.to_string(),
            name: id.to_string(),
            launch: EditorLaunch::Executable {
                path: format!("/opt/{}", id),
                args: vec![],
                working_directory: None,
            },
            icon_source: None,
        }
    }

    fn editor_cache(id: &str) -> EditorCache {
        HashMap::from([(
            id.to_string(),
            EditorInfo {
                name: id.to_string(),
                installed: true,
                launch: None,
                icon_source: None,
            },
        )])
    }

    #[test]
    fn project_save_preserves_disk_owned_editor_fields() {
        let file = TestConfigFile::new("project-authority");
        let existing = ProjectConfig {
            projects: vec![project("old-project")],
            last_updated: "old".to_string(),
            editors: Some(editor_cache("disk-editor")),
            settings: Some(AppSettings {
                custom_editors: vec![custom_editor("disk-custom")],
                ..AppSettings::default()
            }),
            editor_cache_version: Some(7),
            editor_cache_updated_at: Some("disk-cache-time".to_string()),
        };
        write_config(&file.path, &existing).unwrap();

        let stale = ProjectConfig {
            projects: vec![project("new-project")],
            last_updated: "stale".to_string(),
            editors: Some(editor_cache("stale-editor")),
            settings: Some(AppSettings {
                custom_editors: vec![custom_editor("stale-custom")],
                ..AppSettings::default()
            }),
            editor_cache_version: Some(1),
            editor_cache_updated_at: Some("stale-cache-time".to_string()),
        };
        save_project_to(&file.path, &stale).unwrap();

        let saved = load_strict_from(&file.path).unwrap();
        assert_eq!(saved.projects[0].id, "new-project");
        assert!(saved.editors.unwrap().contains_key("disk-editor"));
        assert_eq!(saved.settings.unwrap().custom_editors[0].id, "disk-custom");
        assert_eq!(saved.editor_cache_version, Some(7));
        assert_eq!(
            saved.editor_cache_updated_at.as_deref(),
            Some("disk-cache-time")
        );
    }

    #[test]
    fn explicit_empty_custom_editor_list_is_persisted() {
        let file = TestConfigFile::new("empty-custom-editors");
        let mut config = default_config();
        config.settings = Some(AppSettings {
            custom_editors: vec![custom_editor("custom-one")],
            ..AppSettings::default()
        });
        write_config(&file.path, &config).unwrap();

        update_custom_editors_at(&file.path, |editors| {
            editors.clear();
            Ok(())
        })
        .unwrap();

        let json = fs::read_to_string(&file.path).unwrap();
        let saved = load_strict_from(&file.path).unwrap();
        assert!(saved.settings.unwrap().custom_editors.is_empty());
        assert!(json.contains(r#""customEditors": []"#));
    }

    #[test]
    fn editor_updates_merge_with_the_latest_disk_config() {
        let file = TestConfigFile::new("editor-merge");
        let mut config = default_config();
        config.projects = vec![project("project")];
        config.settings = Some(AppSettings {
            custom_editors: vec![custom_editor("custom-one")],
            ..AppSettings::default()
        });
        write_config(&file.path, &config).unwrap();

        save_editor_cache_to(&file.path, &editor_cache("cached")).unwrap();
        update_custom_editors_at(&file.path, |editors| {
            editors.push(custom_editor("custom-two"));
            Ok(())
        })
        .unwrap();

        let saved = load_strict_from(&file.path).unwrap();
        assert_eq!(saved.projects[0].id, "project");
        assert!(saved.editors.unwrap().contains_key("cached"));
        assert_eq!(saved.editor_cache_version, Some(EDITOR_CACHE_VERSION));
        assert!(saved.editor_cache_updated_at.is_some());
        assert_eq!(saved.settings.unwrap().custom_editors.len(), 2);
    }

    #[test]
    fn legacy_or_incomplete_editor_cache_is_invalidated() {
        let file = TestConfigFile::new("legacy-editor-cache");
        let mut config = default_config();
        config.editors = Some(editor_cache("legacy"));
        write_config(&file.path, &config).unwrap();
        assert!(load_editor_cache_from(&file.path).unwrap().is_none());

        config.editor_cache_version = Some(EDITOR_CACHE_VERSION);
        config.editor_cache_updated_at = Some(chrono::Utc::now().to_rfc3339());
        write_config(&file.path, &config).unwrap();
        assert!(load_editor_cache_from(&file.path).unwrap().is_none());
    }

    #[test]
    fn current_editor_cache_is_loaded_and_expired_cache_is_invalidated() {
        let file = TestConfigFile::new("current-editor-cache");
        let mut cache = editor_cache("current");
        cache.get_mut("current").unwrap().launch = Some(EditorLaunch::Executable {
            path: "/opt/current".to_string(),
            args: vec![],
            working_directory: None,
        });
        save_editor_cache_to(&file.path, &cache).unwrap();
        assert!(load_editor_cache_from(&file.path).unwrap().is_some());

        let mut config = load_strict_from(&file.path).unwrap();
        config.editor_cache_updated_at = Some(
            (chrono::Utc::now() - chrono::Duration::hours(EDITOR_CACHE_TTL_HOURS + 1)).to_rfc3339(),
        );
        write_config(&file.path, &config).unwrap();
        assert!(load_editor_cache_from(&file.path).unwrap().is_none());
    }

    #[test]
    fn malformed_json_is_not_overwritten_by_editor_updates() {
        let file = TestConfigFile::new("malformed");
        let malformed = "{ definitely not valid json";
        fs::write(&file.path, malformed).unwrap();

        assert!(save_editor_cache_to(&file.path, &editor_cache("cached")).is_err());
        assert!(update_custom_editors_at(&file.path, |_| Ok(())).is_err());
        assert_eq!(fs::read_to_string(&file.path).unwrap(), malformed);
    }

    #[test]
    fn malformed_json_is_not_overwritten_by_settings_updates() {
        let file = TestConfigFile::new("malformed-settings");
        let malformed = "{ definitely not valid json";
        fs::write(&file.path, malformed).unwrap();

        assert!(update_settings_at(&file.path, |settings| {
            settings.node_mirror = Some("https://example.invalid".to_string());
        })
        .is_err());
        assert_eq!(fs::read_to_string(&file.path).unwrap(), malformed);
    }

    #[test]
    fn editor_update_reports_write_failures() {
        let file = TestConfigFile::new("missing-parent");
        let missing_path = file.path.with_extension("missing").join("config.json");

        let error = save_editor_cache_to(&missing_path, &editor_cache("cached")).unwrap_err();

        assert!(error.contains("写入临时配置文件失败"));
    }

    struct TestProjectDir {
        path: PathBuf,
    }

    impl TestProjectDir {
        fn new(label: &str, scripts: &[(&str, &str)]) -> Self {
            let id = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
            let nonce = format!(
                "{}-{}-{}",
                std::process::id(),
                id,
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            let path =
                std::env::temp_dir().join(format!("devfleet-project-test-{}-{}", nonce, label));
            fs::create_dir_all(&path).unwrap();
            let scripts = scripts
                .iter()
                .map(|(name, command)| format!(r#""{}":"{}""#, name, command))
                .collect::<Vec<_>>()
                .join(",");
            fs::write(
                path.join("package.json"),
                format!(r#"{{"scripts":{{{}}}}}"#, scripts),
            )
            .unwrap();
            Self { path }
        }
    }

    impl Drop for TestProjectDir {
        fn drop(&mut self) {
            let temp = std::env::temp_dir();
            if self.path.parent() == Some(temp.as_path())
                && self
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("devfleet-project-test-"))
            {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
    }

    #[test]
    fn refresh_preserves_missing_project_and_refreshes_after_restore() {
        let file = TestConfigFile::new("refresh-missing-project");
        let dir = TestProjectDir::new("restorable", &[("serve", "node server.js")]);
        let mut saved = default_config();
        let mut item = project("kept");
        item.path = dir.path.to_string_lossy().to_string();
        item.selected_script = Some("gone".to_string());
        item.note = Some("keep me".to_string());
        saved.projects.push(item);
        write_config(&file.path, &saved).unwrap();
        let missing_path = dir.path.with_file_name(format!(
            "{}-missing",
            dir.path.file_name().unwrap().to_string_lossy()
        ));
        fs::rename(&dir.path, &missing_path).unwrap();

        let refreshed = load_and_refresh_at(&file.path).unwrap();
        assert_eq!(refreshed.projects[0].note.as_deref(), Some("keep me"));
        assert_eq!(refreshed.projects[0].path, saved.projects[0].path);

        fs::rename(&missing_path, &dir.path).unwrap();
        let restored = load_and_refresh_at(&file.path).unwrap();
        assert_eq!(restored.projects[0].scripts[0].name, "serve");
        assert_eq!(restored.projects[0].selected_script, None);
    }

    #[test]
    fn relocate_preserves_project_fields_and_settings() {
        let file = TestConfigFile::new("relocate-preserves-fields");
        let dir = TestProjectDir::new("relocated", &[("dev", "node dev.js")]);
        let mut saved = default_config();
        let mut item = project("stable-id");
        item.path = "/missing/project".to_string();
        item.note = Some("important".to_string());
        item.is_running = Some(true);
        item.selected_script = Some("dev".to_string());
        saved.projects.push(item);
        saved.settings = Some(AppSettings {
            node_mirror: Some("https://mirror.invalid".to_string()),
            ..AppSettings::default()
        });
        write_config(&file.path, &saved).unwrap();

        let relocated =
            relocate_project_at(&file.path, "stable-id", &dir.path.to_string_lossy()).unwrap();
        assert_eq!(relocated.id, "stable-id");
        assert_eq!(relocated.note.as_deref(), Some("important"));
        assert_eq!(relocated.is_running, Some(true));
        assert_eq!(relocated.selected_script.as_deref(), Some("dev"));
        assert_eq!(
            load_strict_from(&file.path)
                .unwrap()
                .settings
                .unwrap()
                .node_mirror
                .as_deref(),
            Some("https://mirror.invalid")
        );
    }

    #[test]
    fn update_project_uses_latest_relocated_project_and_preserves_settings() {
        let file = TestConfigFile::new("update-project");
        let dir = TestProjectDir::new("updated", &[("dev", "node dev.js")]);
        let mut saved = default_config();
        let mut item = project("stable-id");
        item.path = "/missing/project".to_string();
        saved.projects.push(item);
        saved.settings = Some(AppSettings {
            node_mirror: Some("https://mirror.invalid".to_string()),
            ..AppSettings::default()
        });
        write_config(&file.path, &saved).unwrap();
        relocate_project_at(&file.path, "stable-id", &dir.path.to_string_lossy()).unwrap();

        let updated = update_project_at(&file.path, "stable-id", |project| {
            project.note = Some("new note".to_string());
            project.scripts[0].command = "node changed.js".to_string();
            Ok(())
        })
        .unwrap();
        assert_eq!(updated.path, dir.path.to_string_lossy());
        assert_eq!(updated.note.as_deref(), Some("new note"));
        assert_eq!(updated.scripts[0].command, "node changed.js");
        assert_eq!(
            load_strict_from(&file.path)
                .unwrap()
                .settings
                .unwrap()
                .node_mirror
                .as_deref(),
            Some("https://mirror.invalid")
        );
    }

    #[test]
    fn update_project_closure_error_leaves_config_unchanged() {
        let file = TestConfigFile::new("update-project-error");
        let mut saved = default_config();
        saved.projects.push(project("stable-id"));
        write_config(&file.path, &saved).unwrap();
        let before = fs::read(&file.path).unwrap();

        let result = update_project_at(&file.path, "stable-id", |project| {
            project.note = Some("must not persist".to_string());
            Err("拒绝更新".to_string())
        });

        assert_eq!(result.unwrap_err(), "拒绝更新");
        assert_eq!(fs::read(&file.path).unwrap(), before);
    }

    #[test]
    fn default_editor_id_supports_legacy_read_persist_clear_and_preserves_settings() {
        let file = TestConfigFile::new("default-editor");
        fs::write(&file.path, r#"{"projects":[],"lastUpdated":"legacy"}"#).unwrap();
        assert_eq!(load_default_editor_id_at(&file.path).unwrap(), None);

        let mut config = default_config();
        config.settings = Some(AppSettings {
            custom_editors: vec![custom_editor("custom")],
            node_mirror: Some("https://mirror.invalid".to_string()),
            ..AppSettings::default()
        });
        write_config(&file.path, &config).unwrap();
        assert_eq!(
            save_default_editor_id_at(&file.path, Some("custom:editor.v1")).unwrap(),
            Some("custom:editor.v1".to_string())
        );
        assert_eq!(
            load_default_editor_id_at(&file.path).unwrap().as_deref(),
            Some("custom:editor.v1")
        );
        let saved = load_strict_from(&file.path).unwrap();
        let settings = saved.settings.unwrap();
        assert_eq!(settings.custom_editors.len(), 1);
        assert_eq!(
            settings.node_mirror.as_deref(),
            Some("https://mirror.invalid")
        );

        assert_eq!(
            save_default_editor_id_at(&file.path, Some("  ")).unwrap(),
            None
        );
        assert_eq!(load_default_editor_id_at(&file.path).unwrap(), None);
    }

    #[test]
    fn invalid_default_editor_id_does_not_change_config() {
        let file = TestConfigFile::new("default-editor-invalid");
        let mut config = default_config();
        config.settings = Some(AppSettings {
            default_editor_id: Some("existing".to_string()),
            ..AppSettings::default()
        });
        write_config(&file.path, &config).unwrap();
        let before = fs::read(&file.path).unwrap();

        assert!(save_default_editor_id_at(&file.path, Some("bad/editor")).is_err());
        assert!(save_default_editor_id_at(&file.path, Some(&"x".repeat(129))).is_err());
        assert_eq!(fs::read(&file.path).unwrap(), before);
    }

    #[test]
    fn pinned_projects_support_legacy_default_toggle_and_duplicate_free_updates() {
        let file = TestConfigFile::new("pinned-projects");
        fs::write(
            &file.path,
            r#"{"projects":[{"id":"one","name":"one","path":"missing","scripts":[],"selectedScript":null,"isRunning":false,"lastRunTime":null,"packageManager":null,"nodeVersion":null}],"lastUpdated":"legacy"}"#,
        )
        .unwrap();
        assert!(load_strict_from(&file.path).unwrap().settings.is_none());

        assert_eq!(
            set_project_pinned_at(&file.path, "one", true).unwrap(),
            vec!["one".to_string()]
        );
        assert_eq!(
            set_project_pinned_at(&file.path, "one", true).unwrap(),
            vec!["one".to_string()]
        );
        assert_eq!(
            set_project_pinned_at(&file.path, "one", false).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            set_project_pinned_at(&file.path, "one", true).unwrap(),
            vec!["one".to_string()]
        );
        assert_eq!(
            load_strict_from(&file.path)
                .unwrap()
                .settings
                .unwrap()
                .pinned_project_ids,
            vec!["one"]
        );
    }

    #[test]
    fn pinned_project_rejects_unknown_id_without_writing_and_save_removes_deleted_ids() {
        let file = TestConfigFile::new("pinned-projects-safety");
        let mut existing = default_config();
        existing.projects = vec![project("one"), project("two")];
        existing.settings = Some(AppSettings {
            default_editor_id: Some("editor".to_string()),
            node_mirror: Some("https://mirror.invalid".to_string()),
            pinned_project_ids: vec!["one".to_string(), "two".to_string()],
            ..AppSettings::default()
        });
        write_config(&file.path, &existing).unwrap();

        let before = fs::read(&file.path).unwrap();
        assert!(set_project_pinned_at(&file.path, "missing", true).is_err());
        assert!(set_project_pinned_at(&file.path, "", true).is_err());
        assert_eq!(fs::read(&file.path).unwrap(), before);

        let mut updated = default_config();
        updated.projects = vec![project("one")];
        save_project_to(&file.path, &updated).unwrap();
        let settings = load_strict_from(&file.path).unwrap().settings.unwrap();
        assert_eq!(settings.pinned_project_ids, vec!["one"]);
        assert_eq!(settings.default_editor_id.as_deref(), Some("editor"));
        assert_eq!(
            settings.node_mirror.as_deref(),
            Some("https://mirror.invalid")
        );
    }

    #[test]
    fn dirty_pinned_ids_are_filtered_and_deduplicated_before_write() {
        let file = TestConfigFile::new("pinned-projects-dirty");
        let mut config = default_config();
        config.projects = vec![project("one"), project("two")];
        config.settings = Some(AppSettings {
            pinned_project_ids: vec![
                "missing".to_string(),
                "one".to_string(),
                "one".to_string(),
                "two".to_string(),
            ],
            ..AppSettings::default()
        });
        write_config(&file.path, &config).unwrap();

        set_project_pinned_at(&file.path, "one", true).unwrap();
        assert_eq!(
            load_strict_from(&file.path)
                .unwrap()
                .settings
                .unwrap()
                .pinned_project_ids,
            vec!["one", "two"]
        );
    }

    #[test]
    fn relocate_rejects_invalid_or_duplicate_without_writing() {
        let file = TestConfigFile::new("relocate-rejects");
        let first = TestProjectDir::new("first", &[("dev", "node dev.js")]);
        let second = TestProjectDir::new("second", &[("dev", "node dev.js")]);
        let mut saved = default_config();
        let mut one = project("one");
        one.path = first.path.to_string_lossy().to_string();
        let mut two = project("two");
        two.path = second.path.to_string_lossy().to_string();
        saved.projects = vec![one, two];
        write_config(&file.path, &saved).unwrap();
        let before = fs::read(&file.path).unwrap();

        assert!(relocate_project_at(&file.path, "one", "missing").is_err());
        assert_eq!(fs::read(&file.path).unwrap(), before);
        assert!(relocate_project_at(&file.path, "one", &second.path.to_string_lossy()).is_err());
        assert_eq!(fs::read(&file.path).unwrap(), before);
    }
}
