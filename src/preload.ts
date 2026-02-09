// 有关如何使用预加载脚本的详细信息，请参阅 Electron 文档：
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts

import { contextBridge, ipcRenderer } from 'electron';
import { IpcResponse } from './types/project';

// 暴露安全的 API 给渲染进程
contextBridge.exposeInMainWorld('electronAPI', {
  // 选择文件夹
  selectFolder: (): Promise<IpcResponse> =>
    ipcRenderer.invoke('select-folder'),

  // 获取项目的 npm 脚本
  getPackageScripts: (projectPath: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('get-package-scripts', projectPath),

  // 运行 npm 脚本（支持外部终端）
  runScript: (params: { projectPath: string; scriptName: string; projectId: string;}): Promise<IpcResponse> =>
    ipcRenderer.invoke('run-script', params),

  // 停止脚本
  stopScript: (projectId: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('stop-script', projectId),

  // 检查脚本运行状态
  checkScriptStatus: (projectId: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('check-script-status', projectId),

  // 检测编辑器安装情况
  detectEditors: (): Promise<IpcResponse> =>
    ipcRenderer.invoke('detect-editors'),

  // 用编辑器打开项目
  openInEditor: (params: { editor: 'vscode' | 'cursor' | 'webstorm'; projectPath: string }): Promise<IpcResponse> =>
    ipcRenderer.invoke('open-in-editor', params),

  // 加载项目配置
  loadProjectConfig: (): Promise<IpcResponse> =>
    ipcRenderer.invoke('load-project-config'),

  // 保存项目配置
  saveProjectConfig: (config: any): Promise<IpcResponse> =>
    ipcRenderer.invoke('save-project-config', config),

  // 添加项目到配置
  addProjectToConfig: (projectPath: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('add-project-to-config', projectPath),

  // 从配置中移除项目
  removeProjectFromConfig: (projectId: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('remove-project-from-config', projectId),

  // ============= NVM 相关 API =============

  // 获取 NVM 信息
  getNvmInfo: (): Promise<IpcResponse> =>
    ipcRenderer.invoke('get-nvm-info'),

  // 检测项目推荐的 Node 版本
  detectProjectNodeVersion: (projectPath: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('detect-project-node-version', projectPath),

  // 设置项目的 Node 版本
  setProjectNodeVersion: (params: { projectId: string; nodeVersion: string }): Promise<IpcResponse> =>
    ipcRenderer.invoke('set-project-node-version', params),
});

// 声明全局类型，以便 TypeScript 识别
declare global {
  interface Window {
    electronAPI: {
      selectFolder: () => Promise<IpcResponse>;
      getPackageScripts: (projectPath: string) => Promise<IpcResponse>;
      runScript: (params: { projectPath: string; scriptName: string; projectId: string; nodeVersion?: string }) => Promise<IpcResponse>;
      stopScript: (projectId: string) => Promise<IpcResponse>;
      checkScriptStatus: (projectId: string) => Promise<IpcResponse>;
      detectEditors: () => Promise<IpcResponse>;
      openInEditor: (params: { editor: 'vscode' | 'cursor' | 'webstorm'; projectPath: string }) => Promise<IpcResponse>;
      loadProjectConfig: () => Promise<IpcResponse>;
      saveProjectConfig: (config: any) => Promise<IpcResponse>;
      addProjectToConfig: (projectPath: string) => Promise<IpcResponse>;
      removeProjectFromConfig: (projectId: string) => Promise<IpcResponse>;
      getNvmInfo: () => Promise<IpcResponse>;
      detectProjectNodeVersion: (projectPath: string) => Promise<IpcResponse>;
      setProjectNodeVersion: (params: { projectId: string; nodeVersion: string }) => Promise<IpcResponse>;
    };
  }
}
