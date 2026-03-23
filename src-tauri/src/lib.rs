// mod 声明：告诉 Rust 编译器"把这些同目录下的 .rs 文件纳入编译"
// 每个 mod 对应 src/ 下的一个同名文件，比如 mod commands → commands.rs
// Rust 的模块系统：必须显式声明 mod，文件不会自动被编译（跟 JS 的 import 不同）
mod commands;
mod config;
mod detector;
mod models;
mod project;

// use 是 Rust 的导入语法，类似 JS 的 import
// crate 前缀表示"当前项目"（类似 JS 里的相对路径 ./）
use models::AppState;
use std::collections::HashMap;
// Mutex（互斥锁）：Rust 的线程安全机制
// 多个线程要同时读写同一份数据时，必须用 Mutex 包裹，保证同一时刻只有一个线程能访问
use std::sync::Mutex;

/// 应用主入口，构建并启动 Tauri 应用
pub fn run() {
    // tauri::Builder::default() 创建一个 Tauri 应用构建器，采用链式调用（Builder 模式）
    tauri::Builder::default()
        // 注册 Tauri 官方的「文件对话框」插件，提供打开文件夹选择器的能力
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // manage() 注册全局状态，之后在任何 #[tauri::command] 函数中
        // 都可以通过 State<AppState> 参数拿到这个状态（依赖注入）
        .manage(AppState {
            // Mutex<HashMap> 的组合：线程安全的键值存储
            // key = project_id, value = 子进程句柄（用于管理运行中的脚本）
            running_processes: Mutex::new(HashMap::new()),
            // 同上，存储每个项目的脚本输出文本
            script_outputs: Mutex::new(HashMap::new()),
        })
        // invoke_handler 注册所有前端可调用的命令
        // generate_handler! 宏会自动为每个函数生成 IPC 路由
        // 前端通过 invoke('函数名', {参数}) 即可调用对应的 Rust 函数
        .invoke_handler(tauri::generate_handler![
            commands::get_package_scripts,
            commands::detect_package_manager,
            commands::load_project_config,
            commands::save_project_config,
            commands::add_project_to_config,
            commands::remove_project_from_config,
            commands::run_script,
            commands::stop_script,
            commands::check_script_status,
            commands::get_script_output,
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
