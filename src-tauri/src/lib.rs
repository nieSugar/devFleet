#[cfg(target_os = "macos")]
use include_dir::{include_dir, Dir};
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::collections::HashSet;
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    time::Duration,
};
#[cfg(target_os = "macos")]
use sys_locale::get_locale;
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use tauri::Emitter;
#[cfg(target_os = "macos")]
use tauri::{
    menu::{
        Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, HELP_SUBMENU_ID, WINDOW_SUBMENU_ID,
    },
    AppHandle, TitleBarStyle, WebviewUrl, WebviewWindowBuilder,
};
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use tauri::{Manager, Runtime};

// mod 声明：告诉 Rust 编译器"把这些同目录下的 .rs 文件纳入编译"
// 每个 mod 对应 src/ 下的一个同名文件，比如 mod commands → commands.rs
// Rust 的模块系统：必须显式声明 mod，文件不会自动被编译（跟 JS 的 import 不同）
mod commands;
mod config;
mod detector;
mod models;
mod node_manager;
mod project;
mod shell_context;

#[cfg(target_os = "windows")]
const TRAY_SHOW_MENU_ID: &str = "tray-show-main-window";
#[cfg(target_os = "windows")]
const TRAY_QUIT_MENU_ID: &str = "tray-quit-app";
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
const PROJECTS_CHANGED_EVENT: &str = "projects://changed";
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
const COMMAND_MAGIC: &str = "DEVFLEET_COMMAND_V1";
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
const COMMAND_PORT_BASE: u16 = 49152;
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
const COMMAND_PORT_SPAN: u16 = 16384;
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
const COMMAND_APP_ID: &str = "com.niesugar.devfleet";

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum AppCommand {
    AddProject { path: String },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectAddOutcome {
    Added,
    Existing,
    Ignored,
}

impl ProjectAddOutcome {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    fn changed(self) -> bool {
        matches!(self, Self::Added)
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn show_main_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "windows")]
fn setup_windows_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show_item = tauri::menu::MenuItem::with_id(
        app,
        TRAY_SHOW_MENU_ID,
        "显示 DevFleet",
        true,
        None::<&str>,
    )?;
    let quit_item = tauri::menu::MenuItem::with_id(
        app,
        TRAY_QUIT_MENU_ID,
        "退出 DevFleet",
        true,
        None::<&str>,
    )?;
    let menu = tauri::menu::Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut tray = tauri::tray::TrayIconBuilder::with_id("main-tray")
        .tooltip("DevFleet")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_MENU_ID => show_main_window(app),
            TRAY_QUIT_MENU_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

fn add_project_path_to_config(path: std::path::PathBuf, source: &str) -> ProjectAddOutcome {
    if !path.is_dir() {
        return ProjectAddOutcome::Ignored;
    }

    let path = path.to_string_lossy().to_string();
    match project::add_to_config(&path) {
        Ok(project) => {
            eprintln!("[devfleet] added project from {source}: {}", project.path);
            ProjectAddOutcome::Added
        }
        Err(true) => {
            eprintln!("[devfleet] {source} project already exists: {}", path);
            ProjectAddOutcome::Existing
        }
        Err(false) => {
            eprintln!("[devfleet] ignored invalid {source} project: {}", path);
            ProjectAddOutcome::Ignored
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn emit_projects_changed<R: Runtime>(app: &tauri::AppHandle<R>) {
    let _ = app.emit(PROJECTS_CHANGED_EVENT, ());
}

fn startup_project_path_from_args_with<I, F>(args: I, is_dir: F) -> Option<std::path::PathBuf>
where
    I: IntoIterator<Item = std::ffi::OsString>,
    F: Fn(&std::path::Path) -> bool,
{
    let args: Vec<_> = args.into_iter().collect();
    if let Some(index) = args
        .iter()
        .position(|arg| arg.as_os_str() == std::ffi::OsStr::new("--add-project"))
    {
        return args
            .get(index + 1)
            .map(|arg| std::path::PathBuf::from(arg.as_os_str()))
            .filter(|path| is_dir(path));
    }

    args.into_iter().find_map(|arg| {
        let path = std::path::PathBuf::from(arg);
        if is_dir(&path) {
            Some(path)
        } else {
            None
        }
    })
}

fn startup_project_path_from_args() -> Option<std::path::PathBuf> {
    startup_project_path_from_args_with(std::env::args_os().skip(1), |path| path.is_dir())
}

fn add_startup_project_from_args() {
    if let Some(path) = startup_project_path_from_args() {
        add_project_path_to_config(path, "shell context menu");
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn forward_startup_project_to_running_instance() -> bool {
    let Some(path) = startup_project_path_from_args() else {
        return false;
    };

    let Ok(mut stream) =
        TcpStream::connect_timeout(&command_server_addr(), Duration::from_millis(200))
    else {
        return false;
    };

    let command = AppCommand::AddProject {
        path: path.to_string_lossy().to_string(),
    };
    let Ok(body) = serde_json::to_string(&command) else {
        return false;
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    if writeln!(stream, "{COMMAND_MAGIC}").is_err()
        || stream.write_all(body.as_bytes()).is_err()
        || stream.shutdown(Shutdown::Write).is_err()
    {
        return false;
    }

    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok() && command_response_is_success(&response)
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn command_server_addr() -> SocketAddr {
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    SocketAddr::from(([127, 0, 0, 1], command_server_port_for_user(&user)))
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn command_server_port_for_user(user: &str) -> u16 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;

    for byte in COMMAND_APP_ID
        .bytes()
        .chain(std::iter::once(0xff))
        .chain(user.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    COMMAND_PORT_BASE + (hash % u64::from(COMMAND_PORT_SPAN)) as u16
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn command_ack(success: bool) -> String {
    let status = if success { "OK" } else { "ERR" };
    format!("{COMMAND_MAGIC} {status}\n")
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn command_response_is_success(response: &str) -> bool {
    response.trim() == command_ack(true).trim()
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn handle_app_command<R: Runtime>(app: &tauri::AppHandle<R>, command: AppCommand) {
    match command {
        AppCommand::AddProject { path } => {
            let path = std::path::PathBuf::from(path);
            let outcome = add_project_path_to_config(path, "running instance command");
            if outcome.changed() {
                emit_projects_changed(app);
            }
            show_main_window(app);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn handle_command_stream<R: Runtime>(app: &tauri::AppHandle<R>, mut stream: TcpStream) {
    let mut request = String::new();
    let result = (|| -> Result<(), String> {
        stream
            .read_to_string(&mut request)
            .map_err(|e| format!("failed to read command: {e}"))?;

        let Some((magic, body)) = request.split_once('\n') else {
            return Err("missing command magic".to_string());
        };
        if magic.trim_end() != COMMAND_MAGIC {
            return Err("invalid command magic".to_string());
        }

        let command: AppCommand =
            serde_json::from_str(body).map_err(|e| format!("invalid command payload: {e}"))?;
        handle_app_command(app, command);
        Ok(())
    })();

    match result {
        Ok(()) => {
            let _ = stream.write_all(command_ack(true).as_bytes());
        }
        Err(e) => {
            eprintln!("[devfleet] ignored local command: {e}");
            let _ = stream.write_all(command_ack(false).as_bytes());
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn start_command_server<R: Runtime>(app: tauri::AppHandle<R>) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(command_server_addr()) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("[devfleet] local command server unavailable: {e}");
                return;
            }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_command_stream(&app, stream),
                Err(e) => eprintln!("[devfleet] failed to accept local command: {e}"),
            }
        }
    });
}

#[cfg(target_os = "macos")]
fn handle_macos_opened_urls<R: Runtime>(app: &tauri::AppHandle<R>, urls: Vec<tauri::Url>) {
    let mut handled = false;

    for url in urls {
        if url.scheme() != "file" {
            continue;
        }

        if let Ok(path) = url.to_file_path() {
            if path.is_dir() {
                let outcome = add_project_path_to_config(path, "macOS open event");
                if outcome.changed() {
                    emit_projects_changed(app);
                }
                handled = true;
            }
        }
    }

    if handled {
        show_main_window(app);
    }
}

#[cfg(target_os = "macos")]
const ABOUT_WINDOW_LABEL: &str = "about";
#[cfg(target_os = "macos")]
const ABOUT_WINDOW_KIND_INIT_SCRIPT: &str = "window.__DEVFLEET_WINDOW_KIND__ = 'about';";
#[cfg(target_os = "macos")]
const ABOUT_MENU_ID: &str = "open-about-window";
#[cfg(target_os = "macos")]
const SETTINGS_MENU_ID: &str = "open-settings-window";
#[cfg(target_os = "macos")]
const HELP_CENTER_MENU_ID: &str = "open-help-website";
#[cfg(target_os = "macos")]
const ADD_PROJECT_MENU_ID: &str = "trigger-add-project";
#[cfg(target_os = "macos")]
const ADD_PROJECT_EVENT: &str = "macos://add-project";
#[cfg(target_os = "macos")]
const OPEN_SETTINGS_EVENT: &str = "macos://open-settings";
#[cfg(target_os = "macos")]
const HELP_WEBSITE_URL: &str = "https://devfleet.ruiange.com";
#[cfg(target_os = "macos")]
static LOCALE_FILES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../src/i18n/locales");
#[cfg(target_os = "macos")]
static APP_LANGUAGE_OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[cfg(target_os = "macos")]
#[derive(Default, Deserialize)]
struct LocaleTranslationFile {
    #[serde(default, rename = "macStatusBar")]
    mac_status_bar: MacStatusBarTranslations,
}

#[cfg(target_os = "macos")]
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MacStatusBarTranslations {
    about: Option<String>,
    about_window_title: Option<String>,
    settings: Option<String>,
    file: Option<String>,
    edit: Option<String>,
    view: Option<String>,
    window: Option<String>,
    help: Option<String>,
    help_center: Option<String>,
    services: Option<String>,
    hide: Option<String>,
    hide_others: Option<String>,
    quit: Option<String>,
    add_project: Option<String>,
    close_window: Option<String>,
    undo: Option<String>,
    redo: Option<String>,
    cut: Option<String>,
    copy: Option<String>,
    paste: Option<String>,
    select_all: Option<String>,
    fullscreen: Option<String>,
    minimize: Option<String>,
    maximize: Option<String>,
}

#[cfg(target_os = "macos")]
struct ResolvedMacStatusBarTranslations {
    about: String,
    about_window_title: String,
    settings: String,
    file: String,
    edit: String,
    view: String,
    window: String,
    help: String,
    help_center: String,
    services: String,
    hide: String,
    hide_others: String,
    quit: String,
    add_project: String,
    close_window: String,
    undo: String,
    redo: String,
    cut: String,
    copy: String,
    paste: String,
    select_all: String,
    fullscreen: String,
    minimize: String,
    maximize: String,
}

#[cfg(target_os = "macos")]
fn embedded_locale_codes() -> Vec<String> {
    LOCALE_FILES
        .files()
        .filter_map(|file| file.path().file_stem()?.to_str().map(str::to_string))
        .collect()
}

#[cfg(target_os = "macos")]
fn app_language_override() -> &'static Mutex<Option<String>> {
    APP_LANGUAGE_OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "macos")]
fn set_app_language_override(language: Option<&str>) {
    if let Ok(mut guard) = app_language_override().lock() {
        *guard = language.map(str::to_string);
    }
}

#[cfg(target_os = "macos")]
fn get_effective_locale() -> Option<String> {
    if let Ok(guard) = app_language_override().lock() {
        if let Some(language) = guard.clone() {
            return Some(language);
        }
    }

    get_locale()
}

#[cfg(target_os = "macos")]
fn resolve_locale_code(candidate: &str) -> Option<String> {
    let supported = embedded_locale_codes();

    if let Some(exact) = supported
        .iter()
        .find(|locale| locale.eq_ignore_ascii_case(candidate))
    {
        return Some(exact.clone());
    }

    let normalized = candidate.to_ascii_lowercase();
    let language = normalized.split('-').next().unwrap_or_default();

    supported
        .iter()
        .find(|locale| {
            let locale_lower = locale.to_ascii_lowercase();
            locale_lower.starts_with(&format!("{normalized}-"))
                || normalized.starts_with(&format!("{locale_lower}-"))
                || (!language.is_empty() && locale_lower.starts_with(&format!("{language}-")))
        })
        .cloned()
}

#[cfg(target_os = "macos")]
fn load_mac_status_bar_translations(locale: &str) -> Option<MacStatusBarTranslations> {
    let resolved = resolve_locale_code(locale)?;
    let file = LOCALE_FILES.get_file(format!("{resolved}.json"))?;
    let raw = file.contents_utf8()?;
    let parsed = serde_json::from_str::<LocaleTranslationFile>(raw).ok()?;
    Some(parsed.mac_status_bar)
}

#[cfg(target_os = "macos")]
fn resolve_text(
    current: Option<String>,
    fallback: Option<String>,
    default: &str,
    app_name: &str,
) -> String {
    current
        .or(fallback)
        .unwrap_or_else(|| default.to_string())
        .replace("{{appName}}", app_name)
}

#[cfg(target_os = "macos")]
fn resolve_mac_status_bar_translations(app_name: &str) -> ResolvedMacStatusBarTranslations {
    let fallback = load_mac_status_bar_translations("zh-CN").unwrap_or_default();
    let current = get_effective_locale()
        .and_then(|locale| load_mac_status_bar_translations(&locale))
        .unwrap_or_default();

    ResolvedMacStatusBarTranslations {
        about: resolve_text(current.about, fallback.about, "关于 {{appName}}", app_name),
        about_window_title: resolve_text(
            current.about_window_title,
            fallback.about_window_title,
            "关于 {{appName}}",
            app_name,
        ),
        settings: resolve_text(current.settings, fallback.settings, "设置", app_name),
        file: resolve_text(current.file, fallback.file, "文件", app_name),
        edit: resolve_text(current.edit, fallback.edit, "编辑", app_name),
        view: resolve_text(current.view, fallback.view, "视图", app_name),
        window: resolve_text(current.window, fallback.window, "窗口", app_name),
        help: resolve_text(current.help, fallback.help, "帮助", app_name),
        help_center: resolve_text(
            current.help_center,
            fallback.help_center,
            "DevFleet 官网",
            app_name,
        ),
        services: resolve_text(current.services, fallback.services, "服务", app_name),
        hide: resolve_text(current.hide, fallback.hide, "隐藏 {{appName}}", app_name),
        hide_others: resolve_text(
            current.hide_others,
            fallback.hide_others,
            "隐藏其他",
            app_name,
        ),
        quit: resolve_text(current.quit, fallback.quit, "退出 {{appName}}", app_name),
        add_project: resolve_text(
            current.add_project,
            fallback.add_project,
            "添加项目",
            app_name,
        ),
        close_window: resolve_text(
            current.close_window,
            fallback.close_window,
            "关闭窗口",
            app_name,
        ),
        undo: resolve_text(current.undo, fallback.undo, "撤销", app_name),
        redo: resolve_text(current.redo, fallback.redo, "重做", app_name),
        cut: resolve_text(current.cut, fallback.cut, "剪切", app_name),
        copy: resolve_text(current.copy, fallback.copy, "复制", app_name),
        paste: resolve_text(current.paste, fallback.paste, "粘贴", app_name),
        select_all: resolve_text(current.select_all, fallback.select_all, "全选", app_name),
        fullscreen: resolve_text(
            current.fullscreen,
            fallback.fullscreen,
            "进入全屏",
            app_name,
        ),
        minimize: resolve_text(current.minimize, fallback.minimize, "最小化", app_name),
        maximize: resolve_text(current.maximize, fallback.maximize, "缩放", app_name),
    }
}

#[cfg(target_os = "macos")]
fn app_display_name<R: Runtime>(app: &AppHandle<R>) -> String {
    let package = app.package_info();
    app.config()
        .product_name
        .clone()
        .unwrap_or_else(|| package.name.clone())
}

#[cfg(target_os = "macos")]
fn refresh_about_window_title<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(ABOUT_WINDOW_LABEL) {
        let app_name = app_display_name(app);
        let translations = resolve_mac_status_bar_translations(&app_name);
        window.set_title(&translations.about_window_title)?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn sync_macos_app_language<R: Runtime>(
    app: &AppHandle<R>,
    language: Option<&str>,
) -> tauri::Result<()> {
    set_app_language_override(language);
    let menu = build_macos_menu(app)?;
    let _ = app.set_menu(menu)?;
    refresh_about_window_title(app)?;
    Ok(())
}

#[cfg(target_os = "macos")]
// 这套菜单和 About 窗口仅用于 macOS。
// Windows / Linux 仍然保持 Tauri 的原有行为，不会被这次改动影响。
fn build_macos_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let app_name = app_display_name(app);
    let translations = resolve_mac_status_bar_translations(&app_name);
    let about_item =
        MenuItem::with_id(app, ABOUT_MENU_ID, &translations.about, true, None::<&str>)?;
    let settings_item = MenuItem::with_id(
        app,
        SETTINGS_MENU_ID,
        &translations.settings,
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let add_project_item = MenuItem::with_id(
        app,
        ADD_PROJECT_MENU_ID,
        &translations.add_project,
        true,
        Some("CmdOrCtrl+N"),
    )?;

    let app_menu = Submenu::with_items(
        app,
        &app_name,
        true,
        &[
            &about_item,
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, Some(translations.services.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some(translations.hide.as_str()))?,
            &PredefinedMenuItem::hide_others(app, Some(translations.hide_others.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(translations.quit.as_str()))?,
        ],
    )?;

    let file_menu = Submenu::with_items(
        app,
        &translations.file,
        true,
        &[
            &add_project_item,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some(translations.close_window.as_str()))?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        &translations.edit,
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(translations.undo.as_str()))?,
            &PredefinedMenuItem::redo(app, Some(translations.redo.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(translations.cut.as_str()))?,
            &PredefinedMenuItem::copy(app, Some(translations.copy.as_str()))?,
            &PredefinedMenuItem::paste(app, Some(translations.paste.as_str()))?,
            &PredefinedMenuItem::select_all(app, Some(translations.select_all.as_str()))?,
        ],
    )?;

    let view_menu = Submenu::with_items(
        app,
        &translations.view,
        true,
        &[&PredefinedMenuItem::fullscreen(
            app,
            Some(translations.fullscreen.as_str()),
        )?],
    )?;

    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        &translations.window,
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(translations.minimize.as_str()))?,
            &PredefinedMenuItem::maximize(app, Some(translations.maximize.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some(translations.close_window.as_str()))?,
        ],
    )?;

    // Help 菜单至少保留一个真实菜单项，这样 macOS 才会把它当成可交互的原生 Help 菜单，
    // 并在顶部展示系统自带的搜索框。
    let help_center_item = MenuItem::with_id(
        app,
        HELP_CENTER_MENU_ID,
        &translations.help_center,
        true,
        None::<&str>,
    )?;

    let help_menu = Submenu::with_id_and_items(
        app,
        HELP_SUBMENU_ID,
        &translations.help,
        true,
        &[&help_center_item],
    )?;

    Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ],
    )
}

#[cfg(target_os = "macos")]
// 如果 About 窗口已经存在，就直接唤起；否则新建一个更紧凑的小尺寸窗口。
fn open_about_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(ABOUT_WINDOW_LABEL) {
        let _ = window.unminimize();
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let app_name = app_display_name(app);
    let translations = resolve_mac_status_bar_translations(&app_name);

    WebviewWindowBuilder::new(
        app,
        ABOUT_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    // About 窗口显式注入自己的 window kind，避免前端在打包后的首屏阶段
    // 因为 Tauri API 注入时机差异而误判成主窗口路由。
    .initialization_script(ABOUT_WINDOW_KIND_INIT_SCRIPT)
    .title(&translations.about_window_title)
    .inner_size(500.0, 300.0)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .center()
    .focused(true)
    .build()?;

    Ok(())
}

#[cfg(target_os = "macos")]
// 菜单事件只监听我们自定义的 About 菜单项，避免影响其他内建菜单行为。
fn handle_macos_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if event.id() == ABOUT_MENU_ID {
        if let Err(e) = open_about_window(app) {
            eprintln!("[devfleet] 打开关于窗口失败: {}", e);
        }
    } else if event.id() == SETTINGS_MENU_ID {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }

        if let Err(e) = app.emit_to("main", OPEN_SETTINGS_EVENT, ()) {
            eprintln!("[devfleet] 派发设置页面跳转事件失败: {}", e);
        }
    } else if event.id() == HELP_CENTER_MENU_ID {
        // Help 菜单直接打开官网，不再走应用内路由页。
        if let Err(e) = std::process::Command::new("open")
            .arg(HELP_WEBSITE_URL)
            .spawn()
        {
            eprintln!("[devfleet] 打开帮助网址失败: {}", e);
        }
    } else if event.id() == ADD_PROJECT_MENU_ID {
        // 原生菜单只负责发一个 macOS 专用事件，真正的添加流程仍由前端既有逻辑处理。
        if let Err(e) = app.emit_to("main", ADD_PROJECT_EVENT, ()) {
            eprintln!("[devfleet] 派发添加项目事件失败: {}", e);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn normalize_unix_gui_environment() {
    let Some(home) = dirs::home_dir() else {
        return;
    };

    // Finder / 桌面环境启动的 GUI 应用通常拿不到用户 shell 中追加的 PATH。
    // 这里补回常见的用户级工具目录，让打包后的应用也能找到 nvmd、volta、pnpm 等命令。
    if std::env::var_os("NVM_DIR").is_none() {
        let nvm_dir = home.join(".nvm");
        if nvm_dir.is_dir() {
            std::env::set_var("NVM_DIR", &nvm_dir);
        }
    }

    let mut preferred_paths = vec![
        home.join(".nvmd").join("bin"),
        home.join(".volta").join("bin"),
        home.join(".bun").join("bin"),
        home.join(".cargo").join("bin"),
        home.join(".local").join("bin"),
        home.join("bin"),
    ];

    #[cfg(target_os = "macos")]
    {
        preferred_paths.extend([
            std::path::PathBuf::from("/opt/homebrew/bin"),
            std::path::PathBuf::from("/opt/homebrew/sbin"),
            std::path::PathBuf::from("/usr/local/bin"),
            std::path::PathBuf::from("/usr/local/sbin"),
            home.join("Library/Application Support/JetBrains/Toolbox/scripts"),
        ]);
    }

    #[cfg(target_os = "linux")]
    {
        preferred_paths.extend([
            std::path::PathBuf::from("/usr/local/bin"),
            std::path::PathBuf::from("/usr/local/sbin"),
        ]);
    }

    let existing_paths: Vec<std::path::PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();

    let mut merged_paths = Vec::new();
    let mut seen = HashSet::new();

    for path in preferred_paths.into_iter().chain(existing_paths) {
        if !path.is_dir() {
            continue;
        }

        let key = path.to_string_lossy().to_string();
        if seen.insert(key) {
            merged_paths.push(path);
        }
    }

    if let Ok(joined) = std::env::join_paths(merged_paths) {
        std::env::set_var("PATH", joined);
    }
}

/// 应用主入口，构建并启动 Tauri 应用
pub fn run() {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    if forward_startup_project_to_running_instance() {
        return;
    }

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    #[cfg(target_os = "macos")]
    // 仅 macOS 接管菜单栏，其他平台完全跳过这段分支。
    let builder = builder
        .enable_macos_default_menu(false)
        .menu(build_macos_menu)
        .on_menu_event(handle_macos_menu_event);

    builder
        .setup(|app| {
            add_startup_project_from_args();

            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            start_command_server(app.handle().clone());

            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            {
                if let Err(e) = shell_context::refresh_existing_registration() {
                    eprintln!("[devfleet] failed to refresh shell context menu: {}", e);
                }
            }

            #[cfg(target_os = "windows")]
            {
                setup_windows_tray(app.handle())?;
            }

            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                // 仅在 Unix GUI 环境下补 PATH，避免 Windows 被这套用户目录规则影响。
                normalize_unix_gui_environment();
            }

            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            {
                // 开机启动由 Tauri 官方插件接管；默认不会自动启用，
                // 只有用户在设置页打开开关后才会注册到系统自启动。
                use tauri_plugin_autostart::MacosLauncher;

                app.handle().plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    None::<Vec<&str>>,
                ))?;
            }

            #[cfg(target_os = "macos")]
            {
                // macOS 上保留原生窗口装饰，避免无边框窗口吞掉点击或出现假死。
                if let Some(window) = app.get_webview_window("main") {
                    window.set_decorations(true)?;
                    window.set_title_bar_style(TitleBarStyle::Transparent)?;
                }
            }

            Ok(())
        })
        // invoke_handler 注册所有前端可调用的命令
        // generate_handler! 宏会自动为每个函数生成 IPC 路由
        // 前端通过 invoke('函数名', {参数}) 即可调用对应的 Rust 函数
        .invoke_handler(tauri::generate_handler![
            commands::get_package_scripts,
            commands::detect_package_manager,
            commands::load_project_config,
            commands::refresh_project_config,
            commands::save_project_config,
            commands::sync_app_language,
            commands::add_project_to_config,
            commands::remove_project_from_config,
            commands::get_shell_context_menu_state,
            commands::set_shell_context_menu_enabled,
            commands::run_script,
            commands::detect_editors,
            commands::open_in_editor,
            commands::get_nvm_info,
            commands::detect_project_node_version,
            commands::set_project_node_version,
            commands::fetch_remote_node_versions,
            commands::install_node_version,
            commands::switch_node_version,
            commands::uninstall_node_version,
            commands::get_node_mirror,
            commands::set_node_mirror,
            commands::get_node_install_dir,
            commands::set_node_install_dir,
            commands::setup_node_global_path,
            commands::check_node_in_path,
        ])
        // generate_context!() 宏在编译时读取 tauri.conf.json，生成应用上下文
        // run() 启动事件循环（类似前端的 app.mount()）
        .build(tauri::generate_context!())
        // unwrap_or_else：如果启动失败，执行闭包（打印错误并退出）
        // 这比直接 .unwrap()（直接 panic）更优雅，能输出有意义的错误信息
        .unwrap_or_else(|e| {
            eprintln!("[devfleet] Tauri 应用启动失败: {}", e);
            std::process::exit(1);
        })
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = _event {
                handle_macos_opened_urls(_app, urls);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::OsString, path::PathBuf};

    fn os(value: &str) -> OsString {
        OsString::from(value)
    }

    #[test]
    fn startup_project_path_prefers_add_project_argument() {
        let explicit = PathBuf::from("explicit-project");
        let fallback = PathBuf::from("fallback-project");
        let result = startup_project_path_from_args_with(
            vec![
                fallback.clone().into_os_string(),
                os("--add-project"),
                explicit.clone().into_os_string(),
            ],
            |path| path == explicit.as_path() || path == fallback.as_path(),
        );

        assert_eq!(result, Some(explicit));
    }

    #[test]
    fn startup_project_path_uses_first_directory_without_flag() {
        let folder = PathBuf::from("folder-project");
        let result = startup_project_path_from_args_with(
            vec![
                os("--verbose"),
                os("not-a-directory"),
                folder.clone().into_os_string(),
            ],
            |path| path == folder.as_path(),
        );

        assert_eq!(result, Some(folder));
    }

    #[test]
    fn startup_project_path_does_not_fallback_when_flag_target_is_invalid() {
        let fallback = PathBuf::from("fallback-project");
        let result = startup_project_path_from_args_with(
            vec![
                os("--add-project"),
                os("missing-project"),
                fallback.into_os_string(),
            ],
            |path| path.ends_with("fallback-project"),
        );

        assert_eq!(result, None);
    }

    #[test]
    fn command_server_port_is_stable_and_private() {
        let port = command_server_port_for_user("alice");

        assert_eq!(port, command_server_port_for_user("alice"));
        assert!(u32::from(port) >= u32::from(COMMAND_PORT_BASE));
        assert!(u32::from(port) < u32::from(COMMAND_PORT_BASE) + u32::from(COMMAND_PORT_SPAN));
    }

    #[test]
    fn command_server_port_changes_with_user() {
        assert_ne!(
            command_server_port_for_user("alice"),
            command_server_port_for_user("bob")
        );
    }

    #[test]
    fn app_command_add_project_serializes_with_kebab_case_type() {
        let path = r"C:\Users\gs\Projects\demo".to_string();
        let command = AppCommand::AddProject { path: path.clone() };
        let json = serde_json::to_value(command).expect("command should serialize");

        assert_eq!(json["type"], "add-project");
        assert_eq!(json["path"], path);
    }

    #[test]
    fn command_ack_includes_magic_and_status() {
        assert_eq!(command_ack(true), format!("{COMMAND_MAGIC} OK\n"));
        assert_eq!(command_ack(false), format!("{COMMAND_MAGIC} ERR\n"));
        assert!(command_response_is_success(&format!(
            "{COMMAND_MAGIC} OK\r\n"
        )));
        assert!(!command_response_is_success(&format!(
            "{COMMAND_MAGIC} ERR\n"
        )));
    }
}
