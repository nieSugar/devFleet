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
  lastRunTime?: string;
  packageManager?: string;
  nodeVersion?: string;
  note?: string;
}

export interface ProjectConfig {
  projects: Project[];
  lastUpdated: Date | string;
}

export interface ProjectSnapshot extends ProjectConfig {
  availability: Record<string, boolean>;
  pinnedProjectIds: string[];
}

export interface ProcessInfo {
  pid: number;
  projectId: string;
  scriptName: string;
  startTime: Date;
}

export interface NodeProcessPort {
  protocol: string;
  localAddress: string;
  localPort: number;
  state?: string;
}

export interface NodeProcessInfo {
  pid: number;
  parentPid?: number;
  name: string;
  executable?: string;
  commandLine?: string;
  launchCommand?: string;
  startedAt?: string;
  ports?: NodeProcessPort[];
  matchedProjectId?: string;
  matchedProjectName?: string;
  matchedProjectPath?: string;
}

export interface IpcResponse<T = unknown> {
  success: boolean;
  code?: string;
  data?: T;
  error?: string;
}

export type NodeVersionManager =
  | "builtin"
  | "nvm"
  | "nvm-windows"
  | "nvmd"
  | "nvs"
  | "none";

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

export interface RemoteNodeVersion {
  version: string;
  date: string;
  files: string[];
  npm?: string;
  v8?: string;
  lts: string | false;
  security: boolean;
}

export interface EditorInfo {
  name: string;
  installed: boolean;
  source: "auto" | "custom";
  icon?: string;
  path?: string;
  args: string[];
  canEditArgs: boolean;
}

export type EditorStatus = Record<string, EditorInfo>;

export interface DefaultEditorResult {
  editorId: string | null;
}

export type EditorLaunch =
  | {
      kind: "executable";
      path: string;
      args: string[];
      workingDirectory?: string;
    }
  | { kind: "macApp"; path: string }
  | { kind: "desktopEntry"; path: string }
  | { kind: "knownWindowsBatch"; adapterId: string; path: string };

export interface CustomEditor {
  id: string;
  name: string;
  launch: EditorLaunch;
  iconSource?: string;
}

export interface UpsertCustomEditorInput {
  id?: string;
  name?: string;
  path?: string;
  args?: string[];
}

export type EditorCandidateSource =
  | "startMenu"
  | "appPaths"
  | "applications"
  | "spotlight"
  | "desktopEntry";

export interface EditorCandidate {
  id: string;
  name: string;
  path: string;
  source: EditorCandidateSource;
  added: boolean;
  recommended: boolean;
}

export interface EditorCandidateDiscovery {
  candidates: EditorCandidate[];
  warnings: string[];
  truncated: boolean;
}

export interface ImportEditorCandidateInput {
  candidateId: string;
  name?: string;
}

export interface ProjectScanCandidate {
  path: string;
  name: string;
  packageManager?: string;
  added: boolean;
}

export type ProjectScanWarningCode =
  | "ROOT_UNAVAILABLE"
  | "ROOT_INVALID"
  | "CONFIG_READ_FAILED"
  | "DIRECTORY_LIMIT"
  | "CANDIDATE_LIMIT"
  | "DIRECTORY_UNREADABLE";

export interface ProjectScanWarning {
  code: ProjectScanWarningCode;
  path?: string;
  detail?: string;
}

export interface ProjectScanResult {
  candidates: ProjectScanCandidate[];
  warnings: ProjectScanWarning[];
  truncated: boolean;
  cancelled: boolean;
  visitedDirectories: number;
}
