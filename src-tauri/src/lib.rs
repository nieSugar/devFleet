#[cfg(target_os = "macos")]
use include_dir::{include_dir, Dir};
#[cfg(target_os = "macos")]
use serde::Deserialize;
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "macos")]
use sys_locale::get_locale;
#[cfg(target_os = "macos")]
use tauri::{
    menu::{
        HELP_SUBMENU_ID, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
        WINDOW_SUBMENU_ID,
    },
    AppHandle, Emitter, Manager, Runtime, TitleBarStyle, WebviewUrl, WebviewWindowBuilder,
};

// mod 声明：告诉 Rust 编译器"把这些同目录下的 .rs 文件纳入编译"
// 每个 mod 对应 src/ 下的一个同名文件，比如 mod commands → commands.rs
// Rust 的模块系统：必须显式声明 mod，文件不会自动被编译（跟 JS 的 import 不同）
mod commands;
mod config;
mod detector;
mod models;
mod node_manager;
mod project;

#[cfg(target_os = "macos")]
const ABOUT_WINDOW_LABEL: &str = "about";
#[cfg(target_os = "macos")]
const ABOUT_MENU_ID: &str = "open-about-window";
#[cfg(target_os = "macos")]
const SETTINGS_MENU_ID: &str = "open-settings-window";
#[cfg(target_os = "macos")]
const ADD_PROJECT_MENU_ID: &str = "trigger-add-project";
#[cfg(target_os = "macos")]
const ADD_PROJECT_EVENT: &str = "macos://add-project";
#[cfg(target_os = "macos")]
const OPEN_SETTINGS_EVENT: &str = "macos://open-settings";
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
                || (!language.is_empty()
                    && locale_lower.starts_with(&format!("{language}-")))
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
fn resolve_text(current: Option<String>, fallback: Option<String>, default: &str, app_name: &str) -> String {
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
        about: resolve_text(
            current.about,
            fallback.about,
            "关于 {{appName}}",
            app_name,
        ),
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
        services: resolve_text(current.services, fallback.services, "服务", app_name),
        hide: resolve_text(
            current.hide,
            fallback.hide,
            "隐藏 {{appName}}",
            app_name,
        ),
        hide_others: resolve_text(
            current.hide_others,
            fallback.hide_others,
            "隐藏其他",
            app_name,
        ),
        quit: resolve_text(
            current.quit,
            fallback.quit,
            "退出 {{appName}}",
            app_name,
        ),
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
        select_all: resolve_text(
            current.select_all,
            fallback.select_all,
            "全选",
            app_name,
        ),
        fullscreen: resolve_text(
            current.fullscreen,
            fallback.fullscreen,
            "进入全屏",
            app_name,
        ),
        minimize: resolve_text(
            current.minimize,
            fallback.minimize,
            "最小化",
            app_name,
        ),
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
    let about_item = MenuItem::with_id(
        app,
        ABOUT_MENU_ID,
        &translations.about,
        true,
        None::<&str>,
    )?;
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
            &PredefinedMenuItem::close_window(
                app,
                Some(translations.close_window.as_str()),
            )?,
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
            &PredefinedMenuItem::close_window(
                app,
                Some(translations.close_window.as_str()),
            )?,
        ],
    )?;

    let help_menu =
        Submenu::with_id_and_items(app, HELP_SUBMENU_ID, &translations.help, true, &[])?;

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

    WebviewWindowBuilder::new(app, ABOUT_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
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
    } else if event.id() == ADD_PROJECT_MENU_ID {
        // 原生菜单只负责发一个 macOS 专用事件，真正的添加流程仍由前端既有逻辑处理。
        if let Err(e) = app.emit_to("main", ADD_PROJECT_EVENT, ()) {
            eprintln!("[devfleet] 派发添加项目事件失败: {}", e);
        }
    }
}

/// 应用主入口，构建并启动 Tauri 应用
pub fn run() {
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
        .setup(|_app| {
            #[cfg(target_os = "macos")]
            {
                // macOS 上保留原生窗口装饰，避免无边框窗口吞掉点击或出现假死。
                if let Some(window) = _app.get_webview_window("main") {
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
        .run(tauri::generate_context!())
        // unwrap_or_else：如果启动失败，执行闭包（打印错误并退出）
        // 这比直接 .unwrap()（直接 panic）更优雅，能输出有意义的错误信息
        .unwrap_or_else(|e| {
            eprintln!("[devfleet] Tauri 应用启动失败: {}", e);
            std::process::exit(1);
        });
}
