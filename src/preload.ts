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

  // 运行 npm 脚本
  runScript: (params: { projectPath: string; scriptName: string; projectId: string }): Promise<IpcResponse> =>
    ipcRenderer.invoke('run-script', params),

  // 停止脚本
  stopScript: (projectId: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('stop-script', projectId),

  // 检查脚本运行状态
  checkScriptStatus: (projectId: string): Promise<IpcResponse> =>
    ipcRenderer.invoke('check-script-status', projectId),

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
});

// 声明全局类型，以便 TypeScript 识别
declare global {
  interface Window {
    electronAPI: {
      selectFolder: () => Promise<IpcResponse>;
      getPackageScripts: (projectPath: string) => Promise<IpcResponse>;
      runScript: (params: { projectPath: string; scriptName: string; projectId: string }) => Promise<IpcResponse>;
      stopScript: (projectId: string) => Promise<IpcResponse>;
      checkScriptStatus: (projectId: string) => Promise<IpcResponse>;
      loadProjectConfig: () => Promise<IpcResponse>;
      saveProjectConfig: (config: any) => Promise<IpcResponse>;
      addProjectToConfig: (projectPath: string) => Promise<IpcResponse>;
      removeProjectFromConfig: (projectId: string) => Promise<IpcResponse>;
    };
  }
}
