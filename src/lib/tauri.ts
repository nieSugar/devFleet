import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  IpcResponse,
  Project,
  ProjectConfig,
  ProjectSnapshot,
  NvmInfo,
  RemoteNodeVersion,
  EditorStatus,
  DefaultEditorResult,
  CustomEditor,
  EditorCandidateDiscovery,
  ImportEditorCandidateInput,
  UpsertCustomEditorInput,
  NodeProcessInfo,
  ProjectScanResult,
} from "../types/project";

type RunScriptParams = {
  projectPath: string;
  scriptName: string;
  projectId: string;
  packageManager?: string;
  nodeVersion?: string | null;
};

type SetNodeVersionParams = {
  projectId: string;
  nodeVersion: string | null | undefined;
};

export interface ScriptRunResult {
  message: string;
  command: string;
  packageManager: string;
  nodeVersion?: string | null;
}

interface MessageResult {
  message: string;
}

interface NodeVersionResult {
  message: string;
  project: Project;
}

interface DetectedVersion {
  version: string | null;
}

interface ShellContextMenuState {
  supported: boolean;
  enabled: boolean;
  mode?: "managed" | "packaged" | "unsupported";
}

export const tauriAPI = {
  syncAppLanguage: (language: string): Promise<IpcResponse<MessageResult>> =>
    invoke("sync_app_language", { language }),

  selectFolder: async (): Promise<string | null> => {
    return await open({ directory: true, title: "选择项目文件夹" });
  },

  selectEditor: async (title = "选择编辑器程序"): Promise<string | null> => {
    return await open({
      directory: false,
      multiple: false,
      title,
    });
  },

  getPackageScripts: (
    projectPath: string
  ): Promise<IpcResponse<{ scripts: Project["scripts"]; packageManager: string }>> =>
    invoke("get_package_scripts", { projectPath }),

  runScript: (params: RunScriptParams): Promise<IpcResponse<ScriptRunResult>> =>
    invoke("run_script", params),

  listNodeProcesses: (): Promise<IpcResponse<NodeProcessInfo[]>> =>
    invoke("list_node_processes"),

  killNodeProcess: (process: NodeProcessInfo): Promise<IpcResponse<MessageResult>> =>
    invoke("kill_node_process", {
      pid: process.pid,
      expectedStartedAt: process.startedAt ?? null,
      expectedCommandLine: process.commandLine ?? null,
      expectedExecutable: process.executable ?? null,
    }),

  detectEditors: (force?: boolean): Promise<IpcResponse<EditorStatus>> =>
    invoke("detect_editors", { force }),

  getDefaultEditor: (): Promise<IpcResponse<DefaultEditorResult>> =>
    invoke("get_default_editor"),

  setDefaultEditor: (
    editorId: string | null,
  ): Promise<IpcResponse<DefaultEditorResult>> =>
    invoke("set_default_editor", { editorId }),

  openInEditor: (params: {
    editor: string;
    projectPath: string;
  }): Promise<IpcResponse<MessageResult>> => invoke("open_in_editor", params),

  upsertCustomEditor: (
    request: UpsertCustomEditorInput
  ): Promise<IpcResponse<CustomEditor>> => invoke("upsert_custom_editor", { request }),

  removeCustomEditor: (editorId: string): Promise<IpcResponse<CustomEditor>> =>
    invoke("remove_custom_editor", { editorId }),

  discoverEditorCandidates: (): Promise<IpcResponse<EditorCandidateDiscovery>> =>
    invoke("discover_editor_candidates"),

  importEditorCandidate: (
    request: ImportEditorCandidateInput
  ): Promise<IpcResponse<CustomEditor>> => invoke("import_editor_candidate", { request }),

  loadProjectConfig: (): Promise<IpcResponse<ProjectSnapshot>> =>
    invoke("load_project_config"),

  refreshProjectConfig: (): Promise<IpcResponse<ProjectSnapshot>> =>
    invoke("refresh_project_config"),

  relocateProject: (projectId: string, projectPath: string): Promise<IpcResponse<Project>> =>
    invoke("relocate_project", { projectId, projectPath }),

  setProjectPinned: (
    projectId: string,
    pinned: boolean,
  ): Promise<IpcResponse<{ projectIds: string[] }>> =>
    invoke("set_project_pinned", { projectId, pinned }),

  setProjectScript: (projectId: string, scriptName: string): Promise<IpcResponse<Project>> =>
    invoke("set_project_script", { projectId, scriptName }),

  setProjectNote: (projectId: string, note: string): Promise<IpcResponse<Project>> =>
    invoke("set_project_note", { projectId, note }),

  saveProjectConfig: (config: ProjectConfig): Promise<IpcResponse<MessageResult>> =>
    invoke("save_project_config", { config }),

  addProjectToConfig: (projectPath: string): Promise<IpcResponse<Project>> =>
    invoke("add_project_to_config", { projectPath }),

  scanProjectCandidates: (rootPath: string): Promise<IpcResponse<ProjectScanResult>> =>
    invoke("scan_project_candidates", { rootPath }),

  cancelProjectScan: (): Promise<IpcResponse<MessageResult>> =>
    invoke("cancel_project_scan"),

  removeProjectFromConfig: (projectId: string): Promise<IpcResponse<MessageResult>> =>
    invoke("remove_project_from_config", { projectId }),

  getShellContextMenuState: (): Promise<IpcResponse<ShellContextMenuState>> =>
    invoke("get_shell_context_menu_state"),

  setShellContextMenuEnabled: (
    enabled: boolean
  ): Promise<IpcResponse<ShellContextMenuState>> =>
    invoke("set_shell_context_menu_enabled", { enabled }),

  getNvmInfo: (): Promise<IpcResponse<NvmInfo>> => invoke("get_nvm_info"),

  detectProjectNodeVersion: (
    projectPath: string
  ): Promise<IpcResponse<DetectedVersion>> =>
    invoke("detect_project_node_version", { projectPath }),

  setProjectNodeVersion: (
    params: SetNodeVersionParams
  ): Promise<IpcResponse<NodeVersionResult>> =>
    invoke("set_project_node_version", params),

  fetchRemoteNodeVersions: (): Promise<IpcResponse<RemoteNodeVersion[]>> =>
    invoke("fetch_remote_node_versions"),

  installNodeVersion: (params: {
    version: string;
    manager?: string;
  }): Promise<IpcResponse<{ message: string; output: string }>> =>
    invoke("install_node_version", params),

  switchNodeVersion: (params: {
    version: string;
    manager?: string;
  }): Promise<IpcResponse<{ message: string; output: string }>> =>
    invoke("switch_node_version", params),

  uninstallNodeVersion: (params: {
    version: string;
    manager?: string;
  }): Promise<IpcResponse<{ message: string; output: string }>> =>
    invoke("uninstall_node_version", params),

  getNodeMirror: (): Promise<IpcResponse<{ mirror: string }>> =>
    invoke("get_node_mirror"),

  setNodeMirror: (mirror: string): Promise<IpcResponse<{ message: string }>> =>
    invoke("set_node_mirror", { mirror }),

  getNodeInstallDir: (): Promise<IpcResponse<{ dir: string; custom: string }>> =>
    invoke("get_node_install_dir"),

  setNodeInstallDir: (dir: string): Promise<IpcResponse<{ message: string }>> =>
    invoke("set_node_install_dir", { dir }),

  setupNodeGlobalPath: (): Promise<IpcResponse<{ message: string }>> =>
    invoke("setup_node_global_path"),

  checkNodeInPath: (): Promise<
    IpcResponse<{
      inPath: boolean;
      binPath: string | null;
      nodeAvailable: boolean;
      powerShellPolicyReady: boolean;
    }>
  > => invoke("check_node_in_path"),
};
