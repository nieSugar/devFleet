// cfg_attr 是 Rust 的条件编译属性：
//   - not(debug_assertions) 表示"在 release 模式下"（debug 模式不生效）
//   - windows_subsystem = "windows" 告诉 Windows：这是个 GUI 程序，不要弹出控制台黑窗口
//   - 如果你 debug 时想看 println! 输出，这行会自动跳过，所以 debug 模式下控制台还在
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Tauri 应用的入口点，实际的应用构建逻辑在 lib.rs 里的 run() 函数中
// 之所以把逻辑放在 lib.rs 而不是 main.rs，是因为：
//   1. Tauri 移动端（Android/iOS）不走 main.rs，而是直接调用 lib crate
//   2. 把核心逻辑放 lib.rs 可以同时支持桌面端和移动端
fn main() {
    devfleet_lib::run()
}
