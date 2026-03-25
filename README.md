<div align="center">

# devFleet

**轻量、快速的跨平台开发项目管理工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-2.1.0-green.svg)](https://github.com/nieSugar/devFleet/releases)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=white)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)]()

基于 **Tauri 2 + React 19 + TypeScript + Rust** 构建，帮助开发者快速管理和启动多个 Node.js 项目。

[下载安装](#-下载安装) · [功能特性](#-功能特性) · [快速开始](#-快速开始) · [参与贡献](#-参与贡献)

<!-- 👇 替换为你的实际截图 -->
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

</td>
</tr>
<tr>
<td width="50%">

### 🚀 脚本快速启动
- 外部终端运行 & 应用内托管模式
- 跨平台支持：Windows (PowerShell)、macOS (Terminal)、Linux
- 根据包管理器自动生成运行命令

</td>
<td width="50%">

### 🖥️ 编辑器集成
- 一键在 VSCode / Cursor / WebStorm 中打开项目
- 自动检测系统已安装的编辑器
- 支持设置默认编辑器偏好

</td>
</tr>
</table>

> 还有更多：浅色/深色主题切换、终端模式切换、快捷键操作……

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
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- 包管理器：npm / yarn / pnpm / bun 任选其一

### 克隆并安装

```bash
git clone https://github.com/nieSugar/devFleet.git
cd devFleet
npm install
```

### 启动开发模式

```bash
npm run tauri dev
```

同时启动 Vite 前端开发服务器和 Tauri Rust 后端，支持热重载。

### 构建生产版本

```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 📜 可用脚本

| 命令 | 说明 |
|------|------|
| `npm run dev` | 启动 Vite 前端开发服务器 |
| `npm run build` | TypeScript 编译 + Vite 构建 |
| `npm run tauri dev` | Tauri 开发模式（前后端联调） |
| `npm run tauri build` | Tauri 生产构建 |
| `npm run lint` | ESLint 代码检查 |
| `npm run lint:fix` | ESLint 自动修复 |
| `npm run format` | Prettier 代码格式化 |

## 🏗️ 技术栈

| 层 | 技术 |
|----|------|
| 框架 | [Tauri 2](https://v2.tauri.app) — 轻量级跨平台桌面框架 |
| 前端 | [React 19](https://react.dev) + [TypeScript 5](https://www.typescriptlang.org/) + [Vite 7](https://vite.dev) |
| UI | [Ant Design 5](https://ant.design/) + [@ant-design/icons](https://ant.design/components/icon) |
| 后端 | [Rust](https://www.rust-lang.org/) + serde + regex-lite |
| CI/CD | GitHub Actions — 多平台自动构建与发布 |

## 🗂️ 项目结构

<details>
<summary>点击展开</summary>

```
src/                          # 前端源码 (React + TypeScript)
├── renderer.tsx              # 入口文件
├── App.tsx                   # 根组件
├── components/               # UI 组件
├── hooks/                    # 自定义 Hooks
├── lib/                      # Tauri IPC 封装
├── types/                    # 类型定义
└── img/                      # 编辑器图标

src-tauri/                    # 后端源码 (Rust + Tauri)
├── src/
│   ├── lib.rs                # Tauri 启动与命令注册
│   ├── commands.rs           # Tauri 命令
│   ├── config.rs             # 配置读写
│   ├── detector.rs           # 包管理器/编辑器/NVM 检测
│   ├── models.rs             # 数据模型
│   └── project.rs            # 项目逻辑
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
