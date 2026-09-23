import { useState, useCallback, useRef } from "react";
import { IpcResponse, Project } from "../types/project";
import { tauriAPI, type ScriptRunResult } from "../lib/tauri";

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [availability, setAvailability] = useState<Record<string, boolean>>({});
  const [pinnedProjectIds, setPinnedProjectIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const snapshotRequest = useRef(0);
  const pinnedRequestQueue = useRef(Promise.resolve());

  const loadProjects = useCallback(async () => {
    const request = ++snapshotRequest.current;
    setLoading(true);
    setError(null);
    try {
      const result = await tauriAPI.loadProjectConfig();
      if (request !== snapshotRequest.current) return result;
      if (result.success && result.data) {
        setProjects(result.data.projects);
        setAvailability(result.data.availability);
        setPinnedProjectIds(result.data.pinnedProjectIds || []);
      } else {
        const msg = result.error || "加载项目配置失败";
        setError(msg);
        console.error("loadProjects failed:", msg);
      }
      return result;
    } catch (e) {
      const msg = e instanceof Error ? e.message : "加载项目配置异常";
      setError(msg);
      console.error("loadProjects error:", e);
      return { success: false, error: msg };
    } finally {
      setLoading(false);
    }
  }, []);

  const refreshProjects = useCallback(async () => {
    const request = ++snapshotRequest.current;
    setLoading(true);
    try {
      const result = await tauriAPI.refreshProjectConfig();
      if (request !== snapshotRequest.current) return result;
      if (result.success && result.data) {
        setProjects(result.data.projects);
        setAvailability(result.data.availability);
        setPinnedProjectIds(result.data.pinnedProjectIds || []);
      }
      return result;
    } catch (e) {
      return { success: false, error: e instanceof Error ? e.message : "刷新项目失败" };
    } finally {
      setLoading(false);
    }
  }, []);

  const addProject = useCallback(async () => {
    setLoading(true);
    try {
      const path = await tauriAPI.selectFolder();
      if (!path) return null;

      const result = await tauriAPI.addProjectToConfig(path);
      if (result.success && result.data) {
        const project = result.data;
        snapshotRequest.current++;
        setAvailability((prev) => ({ ...prev, [project.id]: true }));
        setProjects((prev) =>
          prev.some((p) => p.id === project.id)
            ? prev
            : [...prev, project],
        );
      }
      return result;
    } finally {
      setLoading(false);
    }
  }, []);

  const removeProject = useCallback(async (projectId: string) => {
    const result = await tauriAPI.removeProjectFromConfig(projectId);
    if (result.success) {
      snapshotRequest.current++;
      setProjects((prev) => prev.filter((p) => p.id !== projectId));
    }
    return result;
  }, []);

  const updateScriptSelection = useCallback(
    async (projectId: string, scriptName: string) => {
      snapshotRequest.current++;
      try {
        const result = await tauriAPI.setProjectScript(projectId, scriptName);
        if (result.success && result.data) {
          snapshotRequest.current++;
          const selectedScript = result.data.selectedScript;
          setProjects((prev) =>
            prev.map((p) => p.id === projectId ? { ...p, selectedScript } : p),
          );
        }
        return result;
      } catch {
        return { success: false, error: "保存配置失败" };
      }
    },
    [],
  );

  const updateNote = useCallback(async (projectId: string, note: string) => {
    snapshotRequest.current++;
    try {
      const result = await tauriAPI.setProjectNote(projectId, note);
      if (result.success && result.data) {
        snapshotRequest.current++;
        const savedNote = result.data.note;
        setProjects((prev) => prev.map((p) => p.id === projectId ? { ...p, note: savedNote } : p));
      }
      return result;
    } catch (e) {
      return { success: false, error: e instanceof Error ? e.message : "保存备注失败" };
    }
  }, []);

  const relocateProject = useCallback(async (projectId: string): Promise<IpcResponse<Project> | null> => {
    try {
      const path = await tauriAPI.selectFolder();
      if (!path) return null;
      snapshotRequest.current++;
      const result = await tauriAPI.relocateProject(projectId, path);
      if (result.success && result.data) {
        const updated = result.data;
        snapshotRequest.current++;
        setProjects((prev) => prev.map((p) => p.id === projectId ? updated : p));
        setAvailability((prev) => ({ ...prev, [projectId]: true }));
      }
      return result;
    } catch (e) {
      return { success: false, error: e instanceof Error ? e.message : "重新定位失败" };
    }
  }, []);

  const togglePinned = useCallback((projectId: string, pinned: boolean) => {
    const request = pinnedRequestQueue.current.then(async () => {
      snapshotRequest.current++;
      try {
        const result = await tauriAPI.setProjectPinned(projectId, pinned);
        if (result.success && result.data) {
          snapshotRequest.current++;
          setPinnedProjectIds(result.data.projectIds);
        }
        return result;
      } catch (e) {
        return {
          success: false,
          error: e instanceof Error ? e.message : "保存置顶状态失败",
        };
      }
    });
    pinnedRequestQueue.current = request.then(() => undefined, () => undefined);
    return request;
  }, []);

  const runScript = useCallback(async (project: Project): Promise<IpcResponse<ScriptRunResult>> => {
    if (!project.selectedScript) {
      return { success: false, error: "请先选择要运行的脚本" };
    }
    setLoading(true);
    try {
      return await tauriAPI.runScript({
        projectPath: project.path,
        scriptName: project.selectedScript,
        projectId: project.id,
        nodeVersion: project.nodeVersion,
      });
    } catch (e) {
      return { success: false, error: e instanceof Error ? e.message : "启动脚本失败" };
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    projects,
    availability,
    pinnedProjectIds,
    setProjects,
    loading,
    error,
    loadProjects,
    refreshProjects,
    addProject,
    removeProject,
    updateScriptSelection,
    updateNote,
    runScript,
    relocateProject,
    togglePinned,
  };
}
