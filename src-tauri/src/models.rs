// serde 是 Rust 生态最核心的序列化框架
// Serialize = Rust 结构体 → JSON（发给前端）
// Deserialize = JSON → Rust 结构体（从前端接收）
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Child 代表一个操作系统子进程的句柄，用来管理（查状态、kill）已启动的进程
use std::process::Child;
// Arc = Atomic Reference Counted，线程安全的引用计数智能指针
// 允许多个线程共享同一份数据的所有权（Rust 默认不允许多所有者，Arc 是个例外）
// Mutex = 互斥锁，保证同一时刻只有一个线程能修改数据
use std::sync::{Arc, Mutex};

// ── 应用全局状态 ──
// 这个结构体通过 Tauri 的 manage() 注册，全生命周期只有一个实例
// 任何 #[tauri::command] 函数都可以通过 State<AppState> 参数访问它

pub struct AppState {
    // HashMap<String, Child>：project_id → 子进程
    // 用 Mutex 包裹是因为多个 Tauri command 可能同时访问（IPC 调用是并发的）
    pub running_processes: Mutex<HashMap<String, Child>>,
    // 双层 Mutex 设计：外层锁 HashMap，内层锁单个项目的输出缓冲区
    // Arc 使得读输出的线程和写输出的线程可以共享同一个 String
    pub script_outputs: Mutex<HashMap<String, Arc<Mutex<String>>>>,
}

// ── IPC 统一响应体 ──
// 前后端通信的标准化数据格式，类似前端的 { success, data, error } 模式

// #[derive(...)] 是 Rust 的派生宏（derive macro）
// 编译器会自动为这个 struct 生成对应 trait 的实现代码：
//   Serialize → 自动生成 to JSON 的代码
//   Clone → 允许 .clone() 深拷贝
//   Debug → 允许 {:?} 格式化打印（调试用）
#[derive(Serialize, Clone, Debug)]
pub struct IpcResponse {
    pub success: bool,
    // Option<T> 是 Rust 的"可空类型"，类似 TS 的 T | null
    // Some(值) 表示有值，None 表示没值
    // skip_serializing_if：序列化为 JSON 时，如果值是 None 就不包含这个字段
    // 这样前端收到的 JSON 更干净，成功时没有 error 字段，失败时没有 data 字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// impl 块：为结构体添加方法（类似 JS class 里写方法）
// Self 指代当前类型（这里就是 IpcResponse）
impl IpcResponse {
    // 泛型函数：T 可以是任何实现了 Serialize 的类型
    // 这意味着你传 String、Vec、HashMap、自定义 struct 都行，只要它能序列化
    pub fn ok<T: Serialize>(data: T) -> Self {
        Self {
            success: true,
            // serde_json::to_value() 把任意类型转成 serde_json::Value（通用 JSON 值）
            // .ok() 把 Result 转成 Option（出错就变成 None，不 panic）
            data: serde_json::to_value(data).ok(),
            error: None,
        }
    }

    // impl Into<String> 是 trait bound：接受任何能转成 String 的类型
    // 比如 &str、String、Cow<str> 都行，调用时更灵活
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }

    pub fn ok_msg(message: impl Into<String>) -> Self {
        Self {
            success: true,
            // serde_json::json!() 宏：快速构造 JSON 值，语法类似 JS 对象字面量
            data: serde_json::to_value(serde_json::json!({ "message": message.into() })).ok(),
            error: None,
        }
    }
}

// ── 项目相关类型 ──

// rename_all = "camelCase"：序列化/反序列化时，字段名自动转为驼峰命名
// Rust 命名规范用 snake_case（selected_script），但前端 JS 用 camelCase（selectedScript）
// 这个属性让两边都用自己的习惯，serde 自动转换
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NpmScript {
    pub name: String,
    pub command: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub scripts: Vec<NpmScript>,
    pub selected_script: Option<String>,
    // #[serde(default)]：反序列化时如果 JSON 里缺少这个字段，使用类型的默认值
    // Option 的默认值是 None，bool 的默认值是 false，Vec 的默认值是 []
    #[serde(default)]
    pub is_running: Option<bool>,
    pub last_run_time: Option<String>,
    pub package_manager: Option<String>,
    pub node_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    pub projects: Vec<Project>,
    pub last_updated: String,
}

// ── Node 版本管理器类型 ──

// PartialEq：允许用 == 和 != 比较两个枚举值
// rename_all = "kebab-case"：序列化时转为短横线命名，如 NvmWindows → "nvm-windows"
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NodeVersionManager {
    Nvm,
    NvmWindows,
    Nvmd,
    Nvs,
    None,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeVersion {
    pub version: String,
    pub full_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub is_current: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NvmInfo {
    pub is_installed: bool,
    pub manager: NodeVersionManager,
    pub current_version: Option<String>,
    pub available_versions: Vec<NodeVersion>,
}

// ── 远程 Node 版本（nodejs.org dist API） ──

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum LtsValue {
    Name(String),
    Bool(bool),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoteNodeVersion {
    pub version: String,
    pub date: String,
    #[serde(default)]
    pub files: Vec<String>,
    pub npm: Option<String>,
    pub v8: Option<String>,
    pub lts: LtsValue,
    pub security: bool,
}

// ── 包管理器类型 ──

// enum（枚举）是 Rust 最强大的特性之一，比 TS 的 enum 强得多
// Rust 的 enum 每个变体可以携带数据（这里没有，但可以）
// rename_all = "lowercase"：Npm → "npm", Yarn → "yarn"
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl PackageManager {
    // &self 表示"借用自身的引用"（不转移所有权）
    // 类似 JS 的 this，但 Rust 区分"只读借用 &self"和"可变借用 &mut self"
    pub fn run_command(&self, script_name: &str) -> String {
        // match 是 Rust 的模式匹配（pattern matching），类似 switch 但更强
        // Rust 的 match 必须覆盖所有可能性（穷举检查），少一个编译器就报错
        match self {
            PackageManager::Npm => format!("npm run {}", script_name),
            PackageManager::Yarn => format!("yarn {}", script_name),
            PackageManager::Pnpm => format!("pnpm {}", script_name),
            PackageManager::Bun => format!("bun {}", script_name),
        }
    }
}

// Display trait：让 PackageManager 能用 .to_string() 和 {} 格式化
// 类似 JS 对象重写 toString() 方法
impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageManager::Npm => write!(f, "npm"),
            PackageManager::Yarn => write!(f, "yarn"),
            PackageManager::Pnpm => write!(f, "pnpm"),
            PackageManager::Bun => write!(f, "bun"),
        }
    }
}

// FromStr trait：让字符串能通过 .parse::<PackageManager>() 转成枚举
// 类似反向的 toString()，"npm" → PackageManager::Npm
impl std::str::FromStr for PackageManager {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "npm" => Ok(PackageManager::Npm),
            "yarn" => Ok(PackageManager::Yarn),
            "pnpm" => Ok(PackageManager::Pnpm),
            "bun" => Ok(PackageManager::Bun),
            _ => Err(format!("Unknown package manager: {}", s)),
        }
    }
}
