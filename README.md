# devFleet

**devFleet** 是一个现代化的开发项目管理工具，基于 **Tauri 2 + React 19 + TypeScript + Rust** 构建，旨在帮助开发者快速管理和启动多个 Node.js 项目。

## 核心功能

### 项目管理
- **快速添加项目** - 选择包含 `package.json` 的项目文件夹即可添加
- **自动检测脚本** - 自动读取项目中的 npm scripts
- **包管理器识别** - 自动识别项目使用的包管理器（npm/yarn/pnpm/bun）
- **持久化配置** - 项目配置自动保存，下次启动自动加载
- **项目搜索** - 按名称或路径快速过滤项目

### Node 版本管理
- **多版本管理器支持** - 支持 nvmd、nvs、nvm、nvm-windows
- **自动版本切换** - 为每个项目指定 Node 版本
- **配置文件生成** - 自动创建版本配置文件（`.nvmdrc`/`.node-version`/`.nvmrc`）
- **版本列表展示** - 显示所有已安装的 Node 版本

### 脚本快速启动
- **外部终端运行** - 在独立终端窗口中运行项目脚本
- **托管模式** - 在应用内管理脚本进程
- **跨平台支持** - Windows (PowerShell)、macOS (Terminal)、Linux (多种终端)
- **智能命令生成** - 根据包管理器类型自动调整运行命令

### 编辑器集成
- **VSCode** - 一键在 VSCode 中打开项目
- **Cursor** - 一键在 Cursor 中打开项目
- **WebStorm** - 一键在 WebStorm 中打开项目
- **自动检测** - 自动检测系统已安装的编辑器

### 应用设置
- **终端模式切换** - 外部终端/托管模式
- **默认编辑器** - 设置偏好编辑器
- **主题切换** - 浅色/深色主题
- **快捷键** - 常用操作键盘快捷键

## 项目结构

```
├── src/                        # 前端源码 (React + TypeScript)
│   ├── renderer.tsx            # 入口文件
│   ├── App.tsx                 # 根组件（ConfigProvider + ErrorBoundary）
│   ├── components/
│   │   ├── ProjectManager.tsx  # 项目管理主组件
│   │   ├── ProjectHeader.tsx   # 页头（搜索、添加、刷新）
│   │   ├── EditorButton.tsx    # 编辑器打开按钮
│   │   ├── NodeVersionSelect.tsx # Node 版本选择器
│   │   ├── ErrorBoundary.tsx   # React 错误边界
│   │   ├── SettingsDrawer.tsx  # 设置抽屉
│   │   └── ProjectManager.css
│   ├── hooks/
│   │   ├── useProjects.ts     # 项目 CRUD + 脚本操作
│   │   ├── useEditors.ts      # 编辑器检测与打开
│   │   └── useNvmInfo.ts      # NVM 信息与版本切换
│   ├── lib/
│   │   └── tauri.ts           # Tauri IPC 封装（强类型）
│   ├── types/
│   │   ├── project.ts         # 业务类型定义
│   │   └── assets.d.ts        # 资源模块声明
│   └── img/                   # 编辑器图标
│
├── src-tauri/                  # 后端源码 (Rust + Tauri)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs            # 入口
│       ├── lib.rs             # Tauri 启动与命令注册
│       ├── commands.rs        # Tauri 命令（含输入校验）
│       ├── config.rs          # 配置读写
│       ├── detector.rs        # 包管理器/编辑器/NVM 检测
│       ├── models.rs          # 数据模型
│       └── project.rs         # 项目逻辑（路径规范化）
│
├── eslint.config.js           # ESLint 扁平配置
├── .prettierrc                # Prettier 配置
├── vite.config.ts             # Vite 构建配置
└── tsconfig.json              # TypeScript 配置
```

## 开发环境设置

### 前置要求

- **Node.js** (>= 18)
- **Rust** (latest stable)
- **包管理器**: npm、yarn、pnpm 或 bun
- **可选 - Node 版本管理器**:
  - [nvmd](https://github.com/1111mp/nvmd) - 跨平台，推荐
  - [nvs](https://github.com/jasongin/nvs) - 跨平台
  - [nvm](https://github.com/nvm-sh/nvm) - macOS/Linux
  - [nvm-windows](https://github.com/coreybutler/nvm-windows) - Windows

### 安装依赖

```bash
npm install
```

### 启动开发模式

```bash
npm run tauri dev
```

这将同时启动 Vite 前端开发服务器和 Tauri 后端，支持热重载。

## 可用脚本

| 脚本 | 说明 |
|------|------|
| `npm run dev` | 启动 Vite 前端开发服务器 |
| `npm run build` | TypeScript 编译 + Vite 构建 |
| `npm run tauri dev` | Tauri 开发模式（前后端联调） |
| `npm run tauri build` | Tauri 生产构建 |
| `npm run lint` | ESLint 检查 |
| `npm run lint:fix` | ESLint 自动修复 |
| `npm run format` | Prettier 格式化 |

## 技术栈

### 核心框架
- **Tauri 2** - 轻量级跨平台桌面框架（Rust 后端）
- **React 19** - 声明式 UI 框架
- **TypeScript 5** - 类型安全
- **Vite 7** - 快速构建工具

### UI 组件
- **Ant Design 5** - 企业级 React 组件库
- **@ant-design/icons** - 图标库

### 后端
- **Rust** - 高性能系统级语言
- **serde** / **serde_json** - 序列化
- **regex-lite** - 轻量正则

### 开发工具
- **ESLint** - 代码质量检查
- **Prettier** - 代码格式化
- **HMR** - 开发时热重载

## 打包和发布

### 本地构建

```bash
npm run tauri build
```

构建产物在 `src-tauri/target/release/bundle/` 目录。

### GitHub Actions 发布

项目配置了 `.github/workflows/release.yml`，支持：
- 手动触发（workflow_dispatch）
- 多平台构建（Windows、macOS arm64/x64、Ubuntu）
- 自动发布到 GitHub Releases

## 配置文件位置

| 操作系统 | 路径 |
|---------|------|
| Windows | `%APPDATA%/devfleet/devfleet-config.json` |
| macOS | `~/Library/Application Support/devfleet/` |
| Linux | `~/.local/share/devfleet/` |

## 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

---

**Made with Tauri + React + Rust by [nieSugar](https://github.com/nieSugar)**
