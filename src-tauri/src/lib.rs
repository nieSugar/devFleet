// mod 声明：告诉 Rust 编译器"把这些同目录下的 .rs 文件纳入编译"
// 每个 mod 对应 src/ 下的一个同名文件，比如 mod commands → commands.rs
// Rust 的模块系统：必须显式声明 mod，文件不会自动被编译（跟 JS 的 import 不同）
mod commands;
mod config;
mod detector;
mod models;
mod project;

/// 应用主入口，构建并启动 Tauri 应用
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
