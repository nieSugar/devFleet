import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { IpcResponse, Project, ProjectConfig } from "../types/project";

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

interface EditorStatus {
  vscode: boolean;
  cursor: boolean;
  webstorm: boolean;
}

interface ScriptRunResult {
  message: string;
  command: string;
  packageManager: string;
  nodeVersion?: string | null;
  mode: "managed" | "external";
  trackable: boolean;
  reason?: string;
}

interface ScriptStatus {
  isRunning: boolean;
  mode: "managed" | "external";
  trackable: boolean;
  reason?: string;
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

export const tauriAPI = {
  selectFolder: async (): Promise<string | null> => {
    return await open({ directory: true, title: "选择项目文件夹" });
  },

  getPackageScripts: (
    projectPath: string
  ): Promise<IpcResponse<{ scripts: Project["scripts"]; packageManager: string }>> =>
    invoke("get_package_scripts", { projectPath }),

  runScript: (params: RunScriptParams): Promise<IpcResponse<ScriptRunResult>> =>
    invoke("run_script", params),

  stopScript: (projectId: string): Promise<IpcResponse<MessageResult>> =>
    invoke("stop_script", { projectId }),

  checkScriptStatus: (projectId: string): Promise<IpcResponse<ScriptStatus>> =>
    invoke("check_script_status", { projectId }),

  getScriptOutput: (
    projectId: string
  ): Promise<IpcResponse<{ output: string }>> =>
    invoke("get_script_output", { projectId }),

  detectEditors: (): Promise<IpcResponse<EditorStatus>> =>
    invoke("detect_editors"),

  openInEditor: (params: {
    editor: string;
    projectPath: string;
  }): Promise<IpcResponse<MessageResult>> => invoke("open_in_editor", params),

  loadProjectConfig: (): Promise<IpcResponse<ProjectConfig>> =>
    invoke("load_project_config"),

  saveProjectConfig: (config: ProjectConfig): Promise<IpcResponse<MessageResult>> =>
    invoke("save_project_config", { config }),

  addProjectToConfig: (projectPath: string): Promise<IpcResponse<Project>> =>
    invoke("add_project_to_config", { projectPath }),

  removeProjectFromConfig: (
    projectId: string
  ): Promise<IpcResponse<MessageResult>> =>
    invoke("remove_project_from_config", { projectId }),

  getNvmInfo: (): Promise<IpcResponse<import("../types/project").NvmInfo>> =>
    invoke("get_nvm_info"),

  detectProjectNodeVersion: (
    projectPath: string
  ): Promise<IpcResponse<DetectedVersion>> =>
    invoke("detect_project_node_version", { projectPath }),

  setProjectNodeVersion: (
    params: SetNodeVersionParams
  ): Promise<IpcResponse<NodeVersionResult>> =>
    invoke("set_project_node_version", params),
};
