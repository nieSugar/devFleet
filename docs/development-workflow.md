# devFleet 开发流程

本文档面向本仓库的日常开发与维护，目标是帮助开发者快速理解项目结构、启动方式、改动路径和发布步骤。

## 1. 项目概览

`devFleet` 是一个基于 `Tauri 2 + React 19 + TypeScript 5 + Rust` 构建的跨平台桌面应用，主要用于管理本地 Node.js 项目、快速运行脚本、切换 Node 版本和打开编辑器。

项目由两部分组成：

- 前端界面：位于 `src/`，负责界面渲染、用户交互、状态管理和调用 Tauri IPC。
- 后端能力：位于 `src-tauri/`，负责读取本地配置、检测环境、运行脚本、操作 Node 版本和打包桌面应用。

## 2. 开发前准备

### 2.1 依赖要求

- Node.js `>= 18`
- pnpm `>= 9`
- Rust stable
- 各平台对应的 Tauri 构建依赖

建议先确认本机工具链：

```bash
node -v
pnpm -v
rustc -V
cargo -V
```

### 2.2 安装依赖

```bash
pnpm install
```

如果是首次在新机器上开发 Tauri，应优先确认 Rust 和系统依赖已安装完成，否则前端能启动，但桌面壳层无法编译运行。

## 3. 仓库结构与职责

### 3.1 前端目录

- `src/renderer.tsx`
  React 入口，挂载 `App`。
- `src/App.tsx`
  全局主题、Ant Design 配置、整体布局和核心页面装配。
- `src/components/`
  页面组件与 UI 交互逻辑。
- `src/hooks/`
  项目、编辑器、Node 版本、快捷键等状态逻辑。
- `src/lib/tauri.ts`
  前端调用 Rust 命令的统一入口。
- `src/types/`
  前端类型定义。

### 3.2 后端目录

- `src-tauri/src/lib.rs`
  Tauri 应用启动入口，注册全部 IPC 命令。
- `src-tauri/src/commands.rs`
  前端可调用的命令层，负责参数接收和响应返回。
- `src-tauri/src/config.rs`
  配置文件读写。
- `src-tauri/src/detector.rs`
  包管理器、编辑器、Node 管理器等环境检测逻辑。
- `src-tauri/src/node_manager.rs`
  Node 版本下载、安装、切换、卸载等逻辑。
- `src-tauri/src/project.rs`
  项目扫描、脚本读取等项目相关逻辑。
- `src-tauri/src/models.rs`
  前后端共享的数据模型。

## 4. 本地开发主流程

### 4.1 启动开发模式

推荐直接使用：

```bash
pnpm tauri dev
```

该命令会按以下顺序工作：

1. 执行 `beforeDevCommand`，即 `pnpm dev`，启动 Vite 前端开发服务器。
2. 打开 `http://localhost:1420` 供 Tauri 桌面窗口加载。
3. 编译并运行 `src-tauri/` 下的 Rust 应用。
4. 监听前后端文件变化并自动重建。

如果只想调试前端页面，可单独运行：

```bash
pnpm dev
```

但需要注意，依赖 Tauri IPC 的功能在纯浏览器模式下无法完整验证。

### 4.2 日常开发建议节奏

推荐按照下面的顺序开展改动：

1. 明确需求属于界面层、状态层还是 Rust 能力层。
2. 先确定数据结构和接口返回格式。
3. 再补前端组件和交互逻辑。
4. 本地自测通过后再整理代码风格和提交。

对于跨前后端的功能，不建议只改 UI 或只改 Rust，应始终从“数据怎么流动”去看整条链路是否闭合。

## 5. 功能开发如何落地

### 5.1 只改前端时

适用场景：

- 调整布局、文案、样式
- 优化交互体验
- 重构前端状态逻辑

常见改动路径：

1. 从 `src/App.tsx` 或具体组件入口定位页面。
2. 在 `src/components/` 中调整组件结构。
3. 在 `src/hooks/` 中整理状态管理与副作用。
4. 若涉及类型变化，同步更新 `src/types/project.ts`。
5. 若涉及调用参数变化，同步检查 `src/lib/tauri.ts`。

### 5.2 改前后端联动功能时

这是本项目最常见的开发路径。推荐按下面顺序处理：

1. 在 Rust 侧确定数据模型和业务逻辑。
2. 在 `src-tauri/src/commands.rs` 暴露新命令或扩展现有命令。
3. 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册命令。
4. 在 `src/lib/tauri.ts` 中增加对应的前端调用封装。
5. 在 `src/hooks/` 或 `src/components/` 中接入实际界面逻辑。
6. 在前端类型文件中补齐响应类型。

可参考本仓库现有链路：

- 组件或 Hook 发起动作
- `src/lib/tauri.ts` 使用 `invoke(...)` 调用命令
- `src-tauri/src/commands.rs` 接收请求
- `config.rs`、`detector.rs`、`project.rs`、`node_manager.rs` 等模块执行实际业务
- 结果返回前端后更新界面状态

### 5.3 新增一个完整能力的推荐步骤

如果要新增“检测某类开发工具”或“新增项目操作”一类能力，建议使用这套顺序：

1. 先在 `models.rs` 设计好输入输出结构。
2. 在对应业务模块中实现核心逻辑。
3. 在 `commands.rs` 添加对外命令，统一错误返回格式。
4. 在 `lib.rs` 注册命令。
5. 在 `src/lib/tauri.ts` 添加 API 方法。
6. 在 Hook 中封装状态和调用。
7. 最后由组件接入按钮、表单或反馈提示。

这样做的好处是职责清晰，后续排查问题时能快速定位是 UI、IPC 还是业务层出了问题。

## 6. 配置与数据流

本项目的项目列表、Node 版本选择等信息会落到本地配置文件中。开发时需要特别关注以下两点：

- 前端修改了项目信息后，通常需要同步调用保存配置的命令，而不仅仅是更新内存状态。
- Rust 侧对本地环境的检测结果可能受平台、Shell、已安装工具和权限影响，调试时要优先确认真实运行环境。

与配置持久化相关的核心入口：

- `src/hooks/useProjects.ts`
- `src-tauri/src/config.rs`
- `src-tauri/src/project.rs`

## 7. 自测流程

每次提交前，至少完成以下检查：

```bash
pnpm lint
pnpm exec tsc --noEmit
cd src-tauri && cargo fmt --check
cd src-tauri && cargo clippy -- -D warnings
cd src-tauri && cargo build
```

如果正在改跨端功能，建议额外执行：

```bash
pnpm tauri dev
```

手动验证以下场景：

- 能否正常加载项目列表
- 添加项目是否成功
- 包管理器和脚本识别是否正确
- 编辑器检测和打开是否正常
- Node 版本检测、设置、切换是否符合预期
- macOS、Windows、Linux 平台相关逻辑是否有条件分支遗漏

## 8. 构建流程

### 8.1 本地构建

```bash
pnpm tauri build
```

执行过程如下：

1. 运行 `pnpm build`，生成前端静态资源到 `dist/`。
2. Tauri 读取 `src-tauri/tauri.conf.json` 中的 `frontendDist`。
3. Rust 编译 release 版本并生成各平台安装包。

产物默认位于：

```text
src-tauri/target/release/bundle/
```

### 8.2 构建前应确认

- `package.json` 与 `src-tauri/tauri.conf.json`、`Cargo.toml` 的版本号是否一致
- 图标资源是否齐全
- 自动更新配置、公钥和发布地址是否正确

## 9. GitHub CI 与发布流程

### 9.1 CI 检查

仓库包含 `.github/workflows/ci.yml`，会在 `push` 或 `pull_request` 到 `master` 时执行：

- 前端检查：`pnpm lint`、`pnpm exec tsc --noEmit`
- Rust 检查：`cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo build`

因此本地开发时最好先跑一遍同样的命令，避免 CI 才发现问题。

### 9.2 Release 发布

仓库包含 `.github/workflows/release.yml`，通过手动触发发布流程，支持：

- Windows
- macOS Apple Silicon
- macOS Intel
- Linux

发布流程会：

1. 安装前端与 Rust 依赖。
2. 在需要时覆盖版本号。
3. 调用 `tauri-action` 构建各平台安装包。
4. 生成 GitHub Release 草稿。
5. 在 Windows 额外生成并上传 Portable 免安装版 zip，解压后可直接运行 `devFleet.exe`。

发布前需确认 GitHub Secrets 已配置完整，尤其是：

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- Apple notarization 相关密钥和账号信息

## 10. 推荐协作规范

- 提交前保持前端与 Rust 检查通过。
- 提交信息建议遵循 Conventional Commits。
- 改动 IPC 接口时，前后端类型要一起更新。
- 涉及平台差异的改动，代码中要明确条件分支，不要默认所有系统行为一致。
- 优先复用现有 Hook、命令返回结构和配置读写方式，避免出现第二套实现。

## 11. 新人上手建议

如果是第一次接手本项目，推荐按下面顺序熟悉代码：

1. 先看 `README.md`，理解产品目标和核心功能。
2. 看 `src/renderer.tsx`、`src/App.tsx`，理解前端入口。
3. 看 `src/components/ProjectManager.tsx`，理解主页面如何组织功能。
4. 看 `src/lib/tauri.ts`，理解前端如何调用后端。
5. 看 `src-tauri/src/lib.rs` 和 `src-tauri/src/commands.rs`，理解 Rust 命令注册与分发。
6. 最后再深入 `config.rs`、`detector.rs`、`node_manager.rs` 等业务模块。

按照这个顺序阅读，能更快建立“界面 -> IPC -> Rust 逻辑 -> 本地系统能力”的完整认知。
