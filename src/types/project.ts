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
  nodeVersion?: string;
  note?: string;
}

export interface ProjectConfig {
  projects: Project[];
  lastUpdated: Date | string;
}

export interface ProcessInfo {
  pid: number;
  projectId: string;
  scriptName: string;
  startTime: Date;
}

export interface IpcResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}

export type NodeVersionManager = 'nvm' | 'nvm-windows' | 'nvmd' | 'nvs' | 'none';

export interface NodeVersion {
  version: string;
  fullVersion: string;
  path?: string;
  isCurrent?: boolean;
}

export interface NvmInfo {
  isInstalled: boolean;
  manager: NodeVersionManager;
  currentVersion?: string;
  availableVersions: NodeVersion[];
}
