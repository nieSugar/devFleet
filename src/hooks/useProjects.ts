import { useState, useCallback } from "react";
import { Project, ProjectConfig } from "../types/project";
import { tauriAPI } from "../lib/tauri";

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);

  const loadProjects = useCallback(async () => {
    setLoading(true);
    try {
      const result = await tauriAPI.loadProjectConfig();
      if (result.success && result.data) {
        setProjects(result.data.projects);
      }
      return result;
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
        setProjects((prev) => [...prev, result.data!]);
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
      const prev = [...projects];
      const updated = projects.map((p) =>
        p.id === projectId ? { ...p, selectedScript: scriptName } : p
      );
      setProjects(updated);

      try {
        const currentResult = await tauriAPI.loadProjectConfig();
        const currentConfig =
          currentResult.success && currentResult.data
            ? currentResult.data
            : ({ projects: updated, lastUpdated: new Date().toISOString() } as ProjectConfig);

        const config: ProjectConfig = {
          ...currentConfig,
          projects: updated,
          lastUpdated: new Date().toISOString(),
        };
        const saveResult = await tauriAPI.saveProjectConfig(config);
        if (!saveResult.success) {
          setProjects(prev);
        }
        return saveResult;
      } catch {
        setProjects(prev);
        return { success: false, error: "保存配置失败" };
      }
    },
    [projects]
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
    loadProjects,
    refreshProjects,
    addProject,
    removeProject,
    updateScriptSelection,
    runScript,
  };
}
