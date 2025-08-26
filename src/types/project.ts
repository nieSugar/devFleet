// 项目管理相关的类型定义

export interface NpmScript {
  name: string;
  command: string;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  scripts: NpmScript[];
  selectedScript?: string;
  isRunning?: boolean;
  lastRunTime?: Date;
}

export interface AppSettings {
  // 是否在外部终端中运行脚本（Windows: PowerShell; macOS: Terminal; Linux: 常见终端）
  runInExternalTerminal: boolean;
}

export interface ProjectConfig {
  projects: Project[];
  lastUpdated: Date;
  settings?: AppSettings;
}

export interface ProcessInfo {
  pid: number;
  projectId: string;
  scriptName: string;
  startTime: Date;
}

// Electron IPC 通信的消息类型
export interface IpcMessage {
  type: 'select-folder' | 'run-script' | 'stop-script' | 'get-package-scripts';
  payload?: any;
}

export interface IpcResponse {
  success: boolean;
  data?: any;
  error?: string;
}
