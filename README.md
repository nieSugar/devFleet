# devFleet

**devFleet** 是一个现代化的开发项目管理工具，基于 Electron + React + TypeScript 构建，旨在帮助开发者快速管理和启动多个 Node.js 项目。

## ✨ 核心功能

### 📁 项目管理
- **快速添加项目** - 选择包含 `package.json` 的项目文件夹即可添加
- **自动检测脚本** - 自动读取项目中的 npm scripts
- **包管理器识别** - 自动识别项目使用的包管理器（npm/yarn/pnpm/bun）
- **持久化配置** - 项目配置自动保存，下次启动自动加载

### 🎯 Node 版本管理
- **多版本管理器支持** - 支持 nvmd、nvs、nvm、nvm-windows
- **自动版本切换** - 为每个项目指定 Node 版本
- **配置文件生成** - 自动创建版本配置文件（`.nvmdrc`/`.node-version`/`.nvmrc`）
- **版本列表展示** - 显示所有已安装的 Node 版本

### 🚀 脚本快速启动
- **外部终端运行** - 在独立终端窗口中运行项目脚本
- **跨平台支持** - Windows (PowerShell)、macOS (Terminal)、Linux (多种终端)
- **智能命令生成** - 根据包管理器类型自动调整运行命令

### 🛠️ 编辑器集成
- **VSCode** - 一键在 VSCode 中打开项目
- **Cursor** - 一键在 Cursor 中打开项目
- **WebStorm** - 一键在 WebStorm 中打开项目
- **自动检测** - 自动检测系统已安装的编辑器

### 🎨 用户体验
- **现代化 UI** - 基于 Ant Design 的美观界面
- **中文界面** - 完全中文化
- **操作简便** - 直观的操作流程

## 📦 项目结构

```
src/
├── components/              # React 组件
│   ├── ProjectManager.tsx   # 项目管理组件
│   └── ProjectManager.css   # 项目管理样式
├── utils/                   # 工具函数
│   └── projectManager.ts    # 项目管理逻辑
├── types/                   # TypeScript 类型定义
│   └── project.ts           # 项目相关类型
├── img/                     # 图标资源
│   ├── vscode.svg          # VSCode 图标
│   ├── cursor.svg          # Cursor 图标
│   └── webstorm.svg        # WebStorm 图标
├── renderer.tsx            # 渲染器进程入口
├── main.ts                 # 主进程（Electron）
├── preload.ts              # 预加载脚本
└── index.css               # 全局样式
```

## 🛠️ 开发环境设置

### 前置要求

- **Node.js** (>= 16.4.0)
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

### 启动开发服务器

```bash
npm start
```

这将启动 Electron 应用程序，并启用热重载功能。

## 📋 可用脚本

- `npm start` - 启动开发模式（带调试工具）
- `npm run start:debug` - 启动调试模式
- `npm run package` - 打包应用程序
- `npm run make` - 创建安装包
- `npm run publish` - 发布到 GitHub Releases
- `npm run lint` - 运行 ESLint 检查

## 🎯 使用指南

### 1. 添加项目

1. 点击 **"添加项目"** 按钮
2. 选择包含 `package.json` 的项目文件夹
3. 应用会自动读取项目信息和 npm scripts

### 2. 配置 Node 版本

1. 在 **Node 版本** 列下拉框中选择版本
2. 应用会自动在项目根目录创建版本配置文件：
   - nvmd → `.nvmdrc`
   - nvs → `.node-version`
   - nvm/nvm-windows → `.nvmrc`

### 3. 运行项目

1. 在 **npm 脚本** 列选择要运行的脚本（如 `dev`、`start`）
2. 点击 **"运行"** 按钮
3. 项目将在新的终端窗口中启动

### 4. 使用编辑器打开

- 点击项目路径旁边的编辑器图标
- 支持 VSCode、Cursor、WebStorm

## 🎨 技术特性

### 界面设计
- **Ant Design** - 专业的 React UI 组件库
- **响应式布局** - 适配不同屏幕尺寸
- **图标库** - Ant Design Icons
- **现代化交互** - 流畅的用户体验

## 🔧 技术栈

### 核心框架
- **Electron 37** - 跨平台桌面应用框架
- **React 19** - 声明式 UI 框架
- **TypeScript 5** - 类型安全的 JavaScript 超集
- **Vite 7** - 快速的构建工具

### UI 组件
- **Ant Design 5** - 企业级 UI 设计语言和 React 组件库
- **@ant-design/icons** - Ant Design 图标库

### 构建和打包
- **Electron Forge** - Electron 应用的完整工具链
- **@electron-forge/plugin-vite** - Vite 插件集成
- **@electron-forge/publisher-github** - GitHub Releases 发布

### 开发工具
- **ESLint** - 代码质量检查
- **Hot Module Replacement** - 开发时的热重载

## 📦 打包和发布

### 本地打包

```bash
# 打包应用（不创建安装包）
npm run package

# 创建安装包（Windows: Squirrel, macOS: DMG/ZIP, Linux: DEB/RPM）
npm run make
```

打包后的文件位于 `out/` 目录。

### 发布到 GitHub Releases

1. 在 `forge.config.ts` 中配置 GitHub 信息：
```typescript
{
  name: '@electron-forge/publisher-github',
  config: {
    repository: {
      owner: 'your-username',
      name: 'devFleet'
    },
    authToken: process.env.GITHUB_TOKEN
  }
}
```

2. 设置环境变量 `GITHUB_TOKEN`
3. 运行发布命令：
```bash
npm run publish
```

## 🌟 项目亮点

### 版本配置文件自动管理
当你为项目指定 Node 版本时，devFleet 会自动在项目根目录创建版本配置文件，让版本管理器自动识别版本：

| 版本管理器 | 配置文件 | 说明 |
|----------|---------|------|
| nvmd | `.nvmdrc` | nvmd 专用配置文件 |
| nvs | `.node-version` | nvs 首选配置文件 |
| nvm | `.nvmrc` | nvm 标准配置文件 |
| nvm-windows | `.nvmrc` | 与 nvm 兼容 |

### 智能包管理器检测
自动检测项目使用的包管理器，并生成正确的运行命令：

| 包管理器 | 检测依据 | 运行命令示例 |
|---------|---------|------------|
| npm | `package-lock.json` | `npm run dev` |
| yarn | `yarn.lock` | `yarn dev` |
| pnpm | `pnpm-lock.yaml` | `pnpm dev` |
| bun | `bun.lockb` | `bun dev` |

### 跨平台终端支持
在不同操作系统上使用最合适的终端运行项目：

| 操作系统 | 默认终端 | 备选方案 |
|---------|---------|---------|
| Windows | PowerShell | - |
| macOS | Terminal.app | - |
| Linux | gnome-terminal | konsole, xterm, alacritty |

## 🤝 贡献指南

欢迎贡献代码、提出建议或报告问题！

### 如何贡献

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 问题反馈

如遇到问题，请在 [Issues](https://github.com/nieSugar/devFleet/issues) 页面提交，并提供：
- 操作系统和版本
- Node.js 版本
- 使用的版本管理器
- 详细的错误信息或截图

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [Electron](https://www.electronjs.org/) - 跨平台桌面应用框架
- [React](https://react.dev/) - 用户界面库
- [Ant Design](https://ant.design/) - UI 组件库
- [nvmd](https://github.com/1111mp/nvmd) - Node 版本管理器
- [nvs](https://github.com/jasongin/nvs) - Node Version Switcher

---

**Made with ❤️ by [nieSugar](https://github.com/nieSugar)**
