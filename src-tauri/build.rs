// build.rs 是 Rust 的「构建脚本」，在编译你的代码之前自动执行
// Cargo 会先编译并运行这个文件，然后才编译你的 src/ 代码
//
// tauri_build::build() 做了几件事：
//   1. 解析 tauri.conf.json 配置，生成编译时需要的常量
//   2. 在 Windows 上嵌入应用图标到 .exe 文件中（资源文件 resource.rc）
//   3. 处理 capabilities（权限声明），生成 ACL 清单
//   4. 生成前端 JS 桥接代码（__global-api-script.js）
fn main() {
    tauri_build::build()
}
