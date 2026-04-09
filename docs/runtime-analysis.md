# devFleet 运行机制分析

本文档解释 `devFleet` 这款桌面应用是如何从启动到响应用户操作一步步运行起来的，重点不是“怎么开发”，而是“程序在运行时实际做了什么”。

## 1. 一句话概括

`devFleet` 的运行模式可以概括为：

前端负责界面和状态，Tauri 负责把前端和本地系统连起来，Rust 负责执行真正的本地能力，包括读取配置、探测环境、打开编辑器、管理 Node 版本以及在外部终端运行脚本。

## 2. 整体运行架构

```mermaid
flowchart LR
  A["用户操作"] --> B["React 组件 / Hooks"]
  B --> C["src/lib/tauri.ts"]
  C --> D["Tauri IPC invoke"]
  D --> E["src-tauri/src/commands.rs"]
  E --> F["Rust 业务模块"]
  F --> G["本地文件系统 / 终端 / 编辑器 / Node 管理器 / 网络"]
  G --> F
  F --> E
  E --> D
  D --> C
  C --> B
```

运行时有两条并行存在的主线：

- UI 主线：React 把界面渲染出来，接收用户点击、输入和选择。
- 系统能力主线：Rust 调用本地文件、终端、外部编辑器、Node 管理器和网络接口。

## 3. 应用启动时发生了什么

### 3.1 开发模式启动

执行 `pnpm tauri dev` 时，Tauri 会按 `src-tauri/tauri.conf.json` 中的配置做两件事：

1. 先运行 `beforeDevCommand`，也就是 `pnpm dev`，启动 Vite 开发服务器。
2. 再运行 Rust 侧的 Tauri 应用，并让桌面窗口加载 `http://localhost:1420`。

所以开发模式下其实是两个进程一起工作：

- Vite 提供前端页面和热更新。
- `cargo run` 启动本地桌面壳层和 Rust 后端能力。

### 3.2 生产模式启动

执行 `pnpm tauri build` 之后，前端会先构建到 `dist/`，再被打包进桌面应用。  
这时应用启动后不再访问本地开发服务器，而是直接加载打包后的前端资源。

开发模式和生产模式的区别主要在“前端资源从哪里来”，但 Rust 命令、IPC 机制和业务逻辑是一套。

### 3.3 Rust 入口链路

应用启动的 Rust 链路如下：

1. `src-tauri/src/main.rs`
   调用 `devfleet_lib::run()`。
2. `src-tauri/src/lib.rs`
   创建 `tauri::Builder`，注册插件、设置窗口、挂载 IPC 命令。
3. `invoke_handler(...)`
   把前端可调用的命令统一注册进去。
4. `.run(...)`
   启动事件循环，桌面应用正式进入运行状态。

其中 `src-tauri/src/lib.rs` 做了几件很关键的事：

- 初始化 `dialog`、`updater`、`process` 等 Tauri 插件。
- 在 macOS 上对窗口装饰和标题栏做平台修正。
- 注册所有 `#[tauri::command]` 命令，供前端通过 `invoke(...)` 调用。

## 4. 前端是如何渲染起来的

前端启动链路比较标准：

1. `src/renderer.tsx` 挂载 React 根节点。
2. `src/App.tsx` 组装全局主题、Ant Design 配置、错误边界和主布局。
3. 主界面由几个核心模块组成：
   - `TitleBar`
   - `ProjectManager`
   - `NodeVersionDrawer`
   - `ThemeProvider`

其中真正承载业务主流程的是 `ProjectManager`。

它在页面加载后会做三类初始化：

- 读取项目配置
- 检测可用编辑器
- 获取 Node 版本管理信息

这些初始化不是直接在组件里硬写，而是拆到了 Hook 中：

- `useProjects`
- `useEditors`
- `useNvmInfo`

这样组件负责界面拼装，Hook 负责数据获取和交互逻辑。

## 5. 配置文件是如何驱动应用运行的

`devFleet` 不是纯临时状态应用，它会把项目列表、编辑器缓存和一些设置保存到本地 JSON 文件。

配置文件由 `src-tauri/src/config.rs` 统一管理，路径按平台分别落在标准目录：

- Windows: `%APPDATA%/devfleet/devfleet-config.json`
- macOS: `~/Library/Application Support/devfleet/devfleet-config.json`
- Linux: `~/.local/share/devfleet/devfleet-config.json`

这个模块承担了几个关键职责：

- 生成配置路径
- 读取配置文件
- 保存配置文件
- 刷新项目列表中的动态信息
- 保存编辑器缓存
- 保存 Node 镜像、安装目录和 builtin 当前版本

为了避免配置损坏，它用了两层保护：

- `Mutex + OnceLock` 对配置读写加锁，避免并发竞争。
- 原子写入策略：先写临时文件，再重命名覆盖正式文件。

所以应用启动后展示的项目列表，并不是写死在代码里，而是来自这份本地配置。

## 6. 项目列表是如何加载出来的

页面加载后，`ProjectManager` 会通过 `useProjects().loadProjects()` 调用：

```text
React Hook
-> src/lib/tauri.ts
-> invoke("load_project_config")
-> src-tauri/src/commands.rs
-> config::load()
```

Rust 返回的 `ProjectConfig` 会被送回前端，前端再把 `projects` 放进 React 状态中并渲染为卡片列表。

每个项目对象通常包含：

- 项目名称
- 绝对路径
- `package.json` 中的 scripts
- 当前选中的脚本
- 识别出的包管理器
- 检测到的 Node 版本
- 备注

项目不是页面层现场推导出来的，而是添加项目时就已经被 Rust 规范化并写入配置。

## 7. 添加项目时，底层到底做了什么

用户点击“添加项目”后，执行链路如下：

1. 前端通过 Tauri Dialog 打开文件夹选择器。
2. 选中文件夹后，前端调用 `add_project_to_config`。
3. Rust 在 `project.rs` 中验证路径是否合法。
4. 如果目录中存在 `package.json`，就进一步创建 `Project` 结构。
5. 创建项目时会同步做三件事：
   - 读取 scripts
   - 检测包管理器
   - 检测项目声明的 Node 版本
6. 生成唯一 ID 后写入配置文件。
7. 前端收到结果后把新项目插入当前列表状态。

这里有两个设计很关键：

- 路径会先 `canonicalize`，避免软链接、相对路径和平台路径差异导致重复项目。
- 包管理器不是靠用户手动选，而是优先通过 lock 文件和 `packageManager` 字段自动识别。

## 8. 点击“运行脚本”时，软件是怎么工作的

这是整个应用最核心的运行路径之一。

### 8.1 前端发起运行请求

用户在项目卡片里选择脚本并点击“运行”后：

1. `ProjectCard` 调用 `onRun(project)`。
2. `useProjects().runScript()` 调用 `tauriAPI.runScript(...)`。
3. 前端通过 IPC 触发 Rust 的 `run_script` 命令。

### 8.2 Rust 侧生成实际命令

`src-tauri/src/commands.rs` 中的 `run_script` 会先做几件事：

1. 校验脚本名，只允许安全字符，防止命令注入。
2. 确定该项目应该用哪个包管理器。
3. 生成基础运行命令。

例如：

- npm: `npm run dev`
- pnpm: `pnpm dev`
- yarn: `yarn dev`
- bun: `bun dev`

### 8.3 按项目注入 Node 版本

如果项目绑定了某个 Node 版本，Rust 会先找到该版本的 Node 二进制目录，再把它插入命令执行时的 `PATH` 最前面。

这一步非常关键，因为它意味着：

- 不必全局切换 Node 版本。
- 不同项目可以在不同终端里同时使用不同 Node 版本。
- 应用只影响当前启动脚本的那个进程环境，不污染系统全局状态。

如果项目没有显式设置版本，则会尝试注入 builtin 管理器当前版本的 `current/bin` 目录，保证 `node`、`npm`、`pnpm` 这类命令尽量可用。

### 8.4 真正启动脚本

命令生成后，应用不会把脚本跑在自己窗口里，而是启动外部终端：

- Windows: `cmd /K`
- macOS: 通过 `osascript` 控制 `Terminal.app`
- Linux: 尝试 `gnome-terminal`、`konsole`、`xterm`

所以 `devFleet` 的角色更像“项目启动调度器”，而不是内置终端模拟器。

这也是为什么运行脚本后，你会看到系统终端窗口弹出来。

## 9. 编辑器按钮为什么能直接打开项目

编辑器功能由两部分组成：

- 检测系统里装了哪些编辑器
- 用对应编辑器打开当前项目目录

### 9.1 编辑器检测

`detect_editors` 采用分层策略：

1. 快速检测
   - Windows 查注册表
   - macOS 查 `.app`
   - Linux 查 `.desktop` 或常见路径
2. macOS 额外用 Spotlight 做补充搜索
3. 最后才回退到 CLI 命令探测

检测结果会写进配置文件作为缓存，所以应用每次启动时不需要都做一轮完整慢扫描。

### 9.2 打开编辑器

用户点击编辑器图标后，前端调用 `open_in_editor`，Rust 根据平台采取不同方式：

- macOS 优先 `open -a <AppName> <project_path>`
- Windows 优先查已知安装路径和注册表
- Linux 优先从桌面文件或可执行路径启动

如果平台特定方案都不成功，才回退到 CLI 命令。

## 10. Node 版本抽屉是如何工作的

Node 版本管理是本项目第二条大主线。

### 10.1 抽屉打开后会做什么

打开 `NodeVersionDrawer` 后，前端会并发调用：

- `fetchRemoteNodeVersions()`
- `getNvmInfo()`
- `getNodeMirror()`
- `getNodeInstallDir()`
- `checkNodeInPath()`

也就是说，这个抽屉不是简单展示静态数据，而是一次性汇总：

- 远程可安装版本
- 本机已安装版本
- 当前使用版本
- 当前版本管理器类型
- 镜像源配置
- builtin 安装目录
- PATH 是否可用

### 10.2 版本管理器识别逻辑

Rust 会优先判断当前系统应该使用哪一套版本管理器：

1. 如果 devFleet 内建目录下已经有版本，优先用 `builtin`
2. 否则依次检测 `nvmd`、`nvs`、`nvm` 或 `nvm-windows`
3. 如果外部管理器都没有，也会回退到 `builtin`

这意味着 `devFleet` 的 Node 管理不是只能依赖系统已有 nvm，它自己也能工作。

### 10.3 builtin 模式怎么运行

如果使用 `builtin`，应用会自己做这些事：

- 从 Node 官方源或镜像拉取版本列表
- 下载对应平台压缩包
- 解压到 `~/devfleet/node` 或自定义目录
- 记录当前版本
- 创建 `current` 链接指向当前版本目录
- 必要时把 `current/bin` 加入用户 PATH

所以 builtin 本质上是一套轻量级 Node 版本管理器。

### 10.4 外部管理器模式怎么运行

如果系统里已有 `nvm`、`nvs`、`nvmd` 等工具，应用会尽量调用它们已有的能力：

- 列出已安装版本
- 读取当前版本
- 安装版本
- 切换版本
- 卸载版本

也就是说，`devFleet` 更像是这些工具的桌面 UI 外壳，而不是完全重写它们的全部逻辑。

## 11. 为什么界面不会被阻塞

这个项目在运行设计上比较重视“别把桌面应用卡死”，所以很多慢操作都放到了后台线程。

例如这些操作大多用了 `tokio::task::spawn_blocking(...)`：

- 刷新项目配置
- 检测编辑器
- 获取 Node 管理信息
- 拉取远程 Node 版本
- 安装、切换、卸载 Node 版本

另外，命令探测和 CLI 调用还做了超时保护，避免某些外部工具卡住时把整个应用拖死。

这类保护尤其重要，因为本项目大量依赖外部环境：

- Shell
- 系统 PATH
- 外部终端
- 编辑器可执行文件
- 第三方 Node 管理器

## 12. 前后端通信为什么比较稳定

项目里前后端的数据交换是统一格式：

```ts
{ success, data, error }
```

Rust 侧由 `IpcResponse` 统一构造返回值，前端侧由 `src/lib/tauri.ts` 统一封装调用入口。

这种做法有几个好处：

- 每个命令的成功和失败结构一致
- 前端 Hook 更容易做错误处理
- 新功能增加时不容易把通信格式写散

这也是为什么项目里很多 Hook 看起来都类似：

1. 调用 `tauriAPI`
2. 判断 `success`
3. 把 `data` 放进状态
4. 把 `error` 用 message 或状态展示出来

## 13. 软件真正依赖的核心边界

理解这款软件时，可以把它看成四层：

### 13.1 展示层

位于 `src/components/`，负责：

- 页面布局
- 卡片展示
- 抽屉和按钮
- 用户反馈

### 13.2 状态与交互层

位于 `src/hooks/` 和 `src/lib/tauri.ts`，负责：

- 页面初始化
- 数据请求
- 调用封装
- 本地状态更新

### 13.3 IPC 与业务层

位于 `src-tauri/src/commands.rs` 和各业务模块，负责：

- 前后端通信
- 参数校验
- 路径校验
- 配置管理
- 命令调度

### 13.4 系统资源层

最终真正被调用的是：

- 本地文件系统
- `package.json`
- shell / PATH
- Terminal / cmd / Linux terminal
- VS Code / Cursor / JetBrains 等编辑器
- Node 官网或镜像源

应用的价值，其实就在于把这些分散的系统能力统一收口到一个桌面 UI 里。

## 14. 可以这样理解整个软件

如果从运行角度给 `devFleet` 下一个定义，它其实是：

一个基于 React 的桌面控制台前端，加上一组由 Rust 实现的本地开发环境操作器。

它自己不直接替代 npm、pnpm、VS Code、nvm 或 Terminal，而是把这些工具和环境能力组织起来，用更低摩擦的方式给开发者使用。

## 15. 阅读源码的推荐顺序

如果想继续顺着“它是怎么运行的”往下读源码，建议按这个顺序：

1. `src-tauri/src/main.rs`
2. `src-tauri/src/lib.rs`
3. `src/renderer.tsx`
4. `src/App.tsx`
5. `src/components/ProjectManager.tsx`
6. `src/lib/tauri.ts`
7. `src-tauri/src/commands.rs`
8. `src-tauri/src/config.rs`
9. `src-tauri/src/project.rs`
10. `src-tauri/src/detector.rs`
11. `src-tauri/src/node_manager.rs`

按这个顺序看，会更容易先建立主链路，再深入具体实现。
