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
  packageManager?: string;
  nodeVersion?: string; // 项目配置的 Node 版本
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

// Node 版本管理器类型
export type NodeVersionManager = 'nvm' | 'nvm-windows' | 'nvmd' | 'nvs' | 'none';

// NVM 相关类型定义
export interface NodeVersion {
  version: string; // 版本号，如 "18.18.0"
  fullVersion: string; // 完整版本号，如 "v18.18.0"
  path?: string; // 版本安装路径
  isCurrent?: boolean; // 是否为当前使用的版本
}

export interface NvmInfo {
  isInstalled: boolean; // 版本管理器是否已安装
  manager: NodeVersionManager; // 版本管理器类型
  currentVersion?: string; // 当前系统使用的 Node 版本
  availableVersions: NodeVersion[]; // 所有可用的 Node 版本
}
