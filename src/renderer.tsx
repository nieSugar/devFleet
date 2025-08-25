/**
 * 此文件将由 vite 自动加载并在"渲染器"上下文中运行。
 * 要了解更多关于 Electron 中"主进程"和"渲染器"上下文之间差异的信息，请访问：
 *
 * https://electronjs.org/docs/tutorial/process-model
 *
 * 默认情况下，此文件中的 Node.js 集成是禁用的。当在渲染器进程中启用 Node.js 集成时，
 * 请注意潜在的安全影响。您可以在此处阅读更多关于安全风险的信息：
 *
 * https://electronjs.org/docs/tutorial/security
 *
 * 要在此文件中启用 Node.js 集成，请打开 `main.ts` 并启用 `nodeIntegration` 标志：
 *
 * ```
 *  // 创建浏览器窗口
 *  mainWindow = new BrowserWindow({
 *    width: 800,
 *    height: 600,
 *    webPreferences: {
 *      nodeIntegration: true
 *    }
 *  });
 * ```
 */

import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './index.css';

console.log('👋 React + Electron 应用启动中...');

// 获取根元素并创建 React 根
const container = document.getElementById('root');
if (!container) {
  throw new Error('找不到根元素！请确保 HTML 中有 id="root" 的元素。');
}

const root = createRoot(container);
root.render(<App />);
