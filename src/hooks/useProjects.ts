import { useState, useCallback } from "react";
import { Project, ProjectConfig } from "../types/project";
import { tauriAPI } from "../lib/tauri";

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadProjects = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await tauriAPI.loadProjectConfig();
      if (result.success && result.data) {
        setProjects(result.data.projects);
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
    setLoading(true);
    try {
      const result = await tauriAPI.refreshProjectConfig();
      if (result.success && result.data) {
        setProjects(result.data.projects);
      }
      return result;
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
      setProjects((prev) => prev.filter((p) => p.id !== projectId));
    }
    return result;
  }, []);

  const updateScriptSelection = useCallback(
    async (projectId: string, scriptName: string) => {
      setProjects((prev) =>
        prev.map((p) =>
          p.id === projectId ? { ...p, selectedScript: scriptName } : p,
        ),
      );

      try {
        const cfgResult = await tauriAPI.loadProjectConfig();
        if (!cfgResult.success || !cfgResult.data) {
          return { success: false, error: "加载配置失败" };
        }
        const updatedProjects = cfgResult.data.projects.map((p) =>
          p.id === projectId ? { ...p, selectedScript: scriptName } : p,
        );
        const config: ProjectConfig = {
          ...cfgResult.data,
          projects: updatedProjects,
          lastUpdated: new Date().toISOString(),
        };
        const saveResult = await tauriAPI.saveProjectConfig(config);
        if (!saveResult.success) {
          setProjects((prev) =>
            prev.map((p) =>
              p.id === projectId ? { ...p, selectedScript: undefined } : p,
            ),
          );
        }
        return saveResult;
      } catch {
        return { success: false, error: "保存配置失败" };
      }
    },
    [],
  );

  const runScript = useCallback(async (project: Project) => {
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
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    projects,
    setProjects,
    loading,
    error,
    loadProjects,
    refreshProjects,
    addProject,
    removeProject,
    updateScriptSelection,
    runScript,
  };
}
