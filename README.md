<div align="center">

# devFleet

**轻量、快速的跨平台开发项目管理工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-2.1.1-green.svg)](https://github.com/nieSugar/devFleet/releases)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=white)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)]()

基于 **Tauri 2 + React 19 + TypeScript 5 + Rust** 构建，帮助开发者快速管理和启动多个 Node.js 项目。

[下载安装](#-下载安装) · [功能特性](#-功能特性) · [快速开始](#-快速开始) · [参与贡献](#-参与贡献)

<!-- ![devFleet Screenshot](docs/screenshot.png) -->

</div>

---

## ✨ 功能特性

<table>
<tr>
<td width="50%">

### 📦 项目管理
- 选择包含 `package.json` 的文件夹即可添加项目
- 自动识别 npm scripts 与包管理器（npm / yarn / pnpm / bun）
- 项目配置自动持久化，支持按名称或路径快速搜索

</td>
<td width="50%">

### 🟢 Node 版本管理
- 支持 nvmd、nvs、nvm、nvm-windows
- 为每个项目指定独立的 Node 版本
- 自动生成 `.nvmdrc` / `.node-version` / `.nvmrc` 配置文件
- 远程获取 Node.js 版本列表，一键安装 / 切换 / 卸载

</td>
</tr>
<tr>
<td width="50%">

### 🚀 脚本快速启动
- 外部终端运行 & 应用内托管模式
- 跨平台支持：Windows (PowerShell)、macOS (Terminal)、Linux
- 根据包管理器自动生成运行命令
- 脚本名校验，防止命令注入

</td>
<td width="50%">

### 🖥️ 编辑器集成
- 一键在 VSCode / Cursor / WebStorm 中打开项目
- 自动检测系统已安装的编辑器
- 支持设置默认编辑器偏好

</td>
</tr>
<tr>
<td width="50%">

### 🎨 界面与体验
- 自定义无边框标题栏，原生窗口控制
- 浅色 / 深色主题自由切换
- 键盘快捷键操作
- 错误边界保护，防止组件崩溃

</td>
<td width="50%">

### 🔄 自动更新
- 应用内检查新版本
- 下载与安装一键完成
- 基于 Tauri Updater 插件，签名验证确保安全

</td>
</tr>
</table>

## 📥 下载安装

前往 [GitHub Releases](https://github.com/nieSugar/devFleet/releases) 下载最新版本：

| 平台 | 安装包 |
|------|--------|
| Windows | `.msi` / `.exe` (NSIS) |
| macOS (Apple Silicon) | `.dmg` (aarch64) |
| macOS (Intel) | `.dmg` (x86_64) |
| Linux | `.deb` / `.AppImage` |

## 🛠️ 快速开始

### 前置要求

- [Node.js](https://nodejs.org/) >= 18
- [pnpm](https://pnpm.io/) >= 9
- [Rust](https://www.rust-lang.org/tools/install) (stable)

### 克隆并安装

```bash
git clone https://github.com/nieSugar/devFleet.git
cd devFleet
pnpm install
```

### 启动开发模式

```bash
pnpm tauri dev
```

同时启动 Vite 前端开发服务器和 Tauri Rust 后端，支持热重载。

### 构建生产版本

```bash
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 📜 可用脚本

| 命令 | 说明 |
|------|------|
| `pnpm dev` | 启动 Vite 前端开发服务器 |
| `pnpm build` | TypeScript 编译 + Vite 构建 |
| `pnpm tauri dev` | Tauri 开发模式（前后端联调） |
| `pnpm tauri build` | Tauri 生产构建 |
| `pnpm lint` | ESLint 代码检查 |
| `pnpm lint:fix` | ESLint 自动修复 |
| `pnpm format` | Prettier 代码格式化 |

## 🏗️ 技术栈

| 层 | 技术 |
|----|------|
| 框架 | [Tauri 2](https://v2.tauri.app) — 轻量级跨平台桌面框架 |
| 前端 | [React 19](https://react.dev) + [TypeScript 5.9](https://www.typescriptlang.org/) + [Vite 7](https://vite.dev) |
| UI | [Ant Design 5](https://ant.design/) + [@ant-design/icons 6](https://ant.design/components/icon) |
| 字体 | [Plus Jakarta Sans](https://fonts.google.com/specimen/Plus+Jakarta+Sans) + [JetBrains Mono](https://www.jetbrains.com/lp/mono/) |
| 后端 | [Rust](https://www.rust-lang.org/) (serde · ureq · chrono · regex-lite) |
| 插件 | Tauri Dialog · Updater · Process |
| 代码质量 | [ESLint 9](https://eslint.org/) (flat config) + [Prettier](https://prettier.io/) + Clippy + rustfmt |
| CI/CD | GitHub Actions — 前后端 CI 检查 + 多平台自动构建与发布 |

## 🗂️ 项目结构

<details>
<summary>点击展开</summary>

```
src/                              # 前端源码 (React + TypeScript)
├── renderer.tsx                  # 应用入口
├── App.tsx                       # 根组件（主题 / 布局）
├── index.css                     # 全局样式与 CSS 变量
├── components/
│   ├── TitleBar.tsx              # 自定义标题栏（窗口控制 / 主题切换）
│   ├── ProjectManager.tsx        # 项目列表与搜索
│   ├── ProjectCard.tsx           # 项目卡片
│   ├── ProjectHeader.tsx         # 项目头部信息
│   ├── EditorButton.tsx          # 编辑器快捷按钮
│   ├── NodeVersionDrawer.tsx     # Node 版本管理抽屉
│   ├── NodeVersionSelect.tsx     # Node 版本选择器
│   ├── UpdateChecker.tsx         # 应用更新检查
│   └── ErrorBoundary.tsx         # 错误边界
├── contexts/
│   └── ThemeContext.tsx           # 主题上下文
├── hooks/
│   ├── useProjects.ts            # 项目数据管理
│   ├── useEditors.ts             # 编辑器检测
│   ├── useNvmInfo.ts             # NVM 信息
│   └── useKeyboardShortcuts.ts   # 快捷键绑定
├── lib/
│   └── tauri.ts                  # Tauri IPC 命令封装
├── types/
│   └── project.ts                # 类型定义
└── img/                          # 编辑器 SVG 图标

src-tauri/                        # 后端源码 (Rust + Tauri)
├── src/
│   ├── lib.rs                    # Tauri 启动与命令注册
│   ├── commands.rs               # IPC 命令实现
│   ├── config.rs                 # 配置文件读写
│   ├── detector.rs               # 包管理器 / 编辑器 / NVM 检测
│   ├── models.rs                 # 数据模型
│   └── project.rs                # 项目逻辑
├── capabilities/                 # Tauri 权限声明
├── icons/                        # 应用图标（多平台多尺寸）
├── Cargo.toml
└── tauri.conf.json
```

</details>

## 📁 配置文件位置

| 操作系统 | 路径 |
|---------|------|
| Windows | `%APPDATA%/devfleet/devfleet-config.json` |
| macOS | `~/Library/Application Support/devfleet/` |
| Linux | `~/.local/share/devfleet/` |

## 🤝 参与贡献

欢迎任何形式的贡献！

1. **Fork** 本仓库
2. 创建特性分支：`git checkout -b feature/your-feature`
3. 提交更改：`git commit -m "feat: add your feature"`
4. 推送分支：`git push origin feature/your-feature`
5. 发起 **Pull Request**

> 提交信息建议遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

## 📄 License

本项目基于 [MIT License](LICENSE) 开源。

---

<div align="center">

**Made with ❤️ by [nieSugar](https://github.com/nieSugar)**

如果觉得有用，欢迎 ⭐ Star 支持一下！

</div>
