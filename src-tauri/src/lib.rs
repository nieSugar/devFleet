#[cfg(target_os = "macos")]
use tauri::{
    menu::{
        HELP_SUBMENU_ID, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
        WINDOW_SUBMENU_ID,
    },
    AppHandle, Manager, Runtime, TitleBarStyle, WebviewUrl, WebviewWindowBuilder,
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
// 这套菜单和 About 窗口仅用于 macOS。
// Windows / Linux 仍然保持 Tauri 的原有行为，不会被这次改动影响。
fn build_macos_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let package = app.package_info();
    let app_name = app
        .config()
        .product_name
        .clone()
        .unwrap_or_else(|| package.name.clone());
    let about_item = MenuItem::with_id(
        app,
        ABOUT_MENU_ID,
        format!("关于 {}", app_name),
        true,
        None::<&str>,
    )?;

    let app_menu = Submenu::with_items(
        app,
        &app_name,
        true,
        &[
            &about_item,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[&PredefinedMenuItem::close_window(app, None)?],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, None)?],
    )?;

    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let help_menu = Submenu::with_id_and_items(app, HELP_SUBMENU_ID, "Help", true, &[])?;

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

    WebviewWindowBuilder::new(app, ABOUT_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
        .title("关于 devFleet")
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
