# de  

这是一个使用 React、TypeScript、Vite 和 Electron 构建的现代桌面应用程序。
devFleet 是一个用于开发和调试的桌面应用程序，支持 VSCode、Cursor、WebStorm 等编辑器。

## 🚀 功能特性

- ⚛️ **React 18** - 现代 React 开发
- 📘 **TypeScript** - 类型安全的开发体验
- ⚡ **Vite** - 快速的构建工具和热重载
- 🖥️ **Electron** - 跨平台桌面应用
- 🎨 **现代 UI** - 美观的毛玻璃效果界面
- 📝 **待办事项** - 完整的 CRUD 功能示例
- 💾 **本地存储** - 数据持久化
- 🌐 **中文界面** - 完全中文化的用户界面

## 📦 项目结构

```
src/
├── components/          # React 组件
│   ├── TodoList.tsx    # 待办事项组件
│   └── TodoList.css    # 待办事项样式
├── App.tsx             # 主应用组件
├── App.css             # 主应用样式
├── renderer.tsx        # 渲染器进程入口
├── main.ts             # 主进程（Electron）
├── preload.ts          # 预加载脚本
└── index.css           # 全局样式
```

## 🛠️ 开发环境设置

### 前置要求

- Node.js (>= 16.4.0)
- npm 或 yarn

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

- `npm start` - 启动开发模式
- `npm run package` - 打包应用程序
- `npm run make` - 创建安装包
- `npm run publish` - 发布应用程序
- `npm run lint` - 运行 ESLint 检查

## 🎯 应用功能

## 🎨 界面设计

- **现代毛玻璃效果** - 使用 backdrop-filter 实现
- **渐变背景** - 美观的紫色渐变
- **响应式设计** - 适配不同屏幕尺寸
- **平滑动画** - CSS 过渡效果
- **中文字体优化** - 支持微软雅黑等中文字体

## 🔧 技术栈详解

### 前端技术
- **React 18** - 使用最新的 React 特性
- **TypeScript** - 提供类型安全
- **CSS3** - 现代 CSS 特性（Grid、Flexbox、backdrop-filter）

### 构建工具
- **Vite** - 快速的开发服务器和构建工具
- **Electron Forge** - Electron 应用的构建和打包

### 开发工具
- **ESLint** - 代码质量检查
- **Hot Reload** - 开发时的热重载

## 📱 如何添加新功能

### 1. 创建新的 React 组件

```tsx
// src/components/NewComponent.tsx
import React from 'react';
import './NewComponent.css';

const NewComponent: React.FC = () => {
  return (
    <div className="new-component">
      <h2>新组件</h2>
    </div>
  );
};

export default NewComponent;
```

### 2. 在主应用中使用

```tsx
// src/App.tsx
import NewComponent from './components/NewComponent';

// 在 JSX 中使用
<NewComponent />
```

### 3. 添加样式

```css
/* src/components/NewComponent.css */
.new-component {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  border-radius: 15px;
  padding: 2rem;
}
```

## 🚀 部署和分发

### 打包应用程序

```bash
npm run package
```

### 创建安装包

```bash
npm run make
```

生成的文件将在 `out/` 目录中。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT License
