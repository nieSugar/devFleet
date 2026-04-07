import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Project, ProjectConfig } from "../types/project";
import { tauriAPI } from "../lib/tauri";
import { listenForMacOSAddProject } from "../lib/macosNative";
import { useProjects } from "../hooks/useProjects";
import { useEditors } from "../hooks/useEditors";
import { useNvmInfo } from "../hooks/useNvmInfo";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import ProjectHeader from "./ProjectHeader";
import ProjectCard from "./ProjectCard";
import { PlusOutlined, FolderOpenOutlined } from "@ant-design/icons";
import { App, Button } from "antd";
import "./ProjectManager.css";

interface ProjectManagerProps {
  nvmRefreshKey?: number;
}

const ProjectManager: React.FC<ProjectManagerProps> = ({ nvmRefreshKey }) => {
  const {
    projects,
    setProjects,
    loading,
    loadProjects,
    refreshProjects,
    addProject,
    removeProject,
    updateScriptSelection,
    runScript,
  } = useProjects();
  const { editors, openInEditor, refreshEditors } = useEditors();
  const { nvmInfo, changeNodeVersion, refreshNvmInfo } = useNvmInfo();
  const { modal, message: messageApi } = App.useApp();
  const { t } = useTranslation();
  const [searchText, setSearchText] = useState("");

  useEffect(() => {
    let cancelled = false;
    loadProjects().then((r) => {
      if (!cancelled && r && !r.success)
        messageApi.error(r.error || t("project.loadFailed"));
    });
    return () => { cancelled = true; };
  }, [loadProjects, messageApi, t]);

  const nvmRefreshKeyRef = useRef(nvmRefreshKey);
  useEffect(() => {
    if (nvmRefreshKeyRef.current !== undefined && nvmRefreshKey !== nvmRefreshKeyRef.current) {
      refreshNvmInfo();
      refreshProjects();
    }
    nvmRefreshKeyRef.current = nvmRefreshKey;
  }, [nvmRefreshKey, refreshNvmInfo, refreshProjects]);

  const handleRefresh = useCallback(() => {
    refreshEditors();
    refreshProjects().then((r) => {
      if (r && !r.success) messageApi.error(r.error || t("project.refreshFailed"));
    });
  }, [refreshEditors, refreshProjects, messageApi, t]);

  useKeyboardShortcuts({
    onAddProject: () => handleAdd(),
    onRefresh: handleRefresh,
  });

  const showMsg = useCallback((type: "success" | "error", text: string) => {
    if (type === "success") messageApi.success(text);
    else messageApi.error(text);
  }, [messageApi]);

  const handleAdd = useCallback(async () => {
    const result = await addProject();
    if (!result) return;
    if (result.success && result.data)
      showMsg("success", t("project.addSuccess", { name: result.data.name }));
    else showMsg("error", result.error || t("project.addFailed"));
  }, [addProject, showMsg, t]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    // macOS 原生菜单的“添加项目”最终仍然复用前端现有的 handleAdd 流程。
    void listenForMacOSAddProject(handleAdd).then((cleanup) => {
      if (cancelled) cleanup();
      else unlisten = cleanup;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [handleAdd]);

  const handleRemove = useCallback((projectId: string, projectName: string) => {
    modal.confirm({
      title: t("project.deleteTitle"),
      content: t("project.deleteContent", { name: projectName }),
      okText: t("common.delete"),
      okType: "danger",
      cancelText: t("common.cancel"),
      onOk: async () => {
        try {
          const result = await removeProject(projectId);
          if (result.success) showMsg("success", t("project.deleteSuccess"));
          else showMsg("error", result.error || t("project.deleteFailed"));
        } catch {
          showMsg("error", t("project.deleteError"));
        }
      },
    });
  }, [modal, removeProject, showMsg, t]);

  const handleScriptChange = useCallback(async (projectId: string, scriptName: string) => {
    const result = await updateScriptSelection(projectId, scriptName);
    if (!result.success) showMsg("error", result.error || t("project.saveFailed"));
  }, [updateScriptSelection, showMsg, t]);

  const handleNodeVersionChange = useCallback(async (
    projectId: string,
    nodeVersion: string | null | undefined,
  ) => {
    try {
      const result = await changeNodeVersion(projectId, nodeVersion);
      if (result.success && result.data) {
        setProjects((prev) =>
          prev.map((p) =>
            p.id === projectId
              ? { ...p, nodeVersion: nodeVersion || undefined }
              : p,
          ),
        );
        showMsg("success", result.data.message || t("project.nodeVersionUpdated"));
      } else {
        showMsg("error", result.error || t("project.nodeVersionFailed"));
      }
    } catch {
      showMsg("error", t("project.nodeVersionError"));
    }
  }, [changeNodeVersion, setProjects, showMsg, t]);

  const handleRun = useCallback(async (project: Project) => {
    const result = await runScript(project);
    if (result.success) {
      const vi = project.nodeVersion ? ` (Node ${project.nodeVersion})` : "";
      showMsg("success", t("project.scriptStarted", { script: project.selectedScript, version: vi }));
    } else {
      showMsg("error", result.error || t("project.scriptFailed"));
    }
  }, [runScript, showMsg, t]);

  const handleNoteChange = useCallback(async (projectId: string, note: string) => {
    const trimmed = note.trim() || undefined;
    setProjects((prev) =>
      prev.map((p) => (p.id === projectId ? { ...p, note: trimmed } : p)),
    );
    try {
      const cfgResult = await tauriAPI.loadProjectConfig();
      if (cfgResult.success && cfgResult.data) {
        const updatedProjects = cfgResult.data.projects.map((p: Project) =>
          p.id === projectId ? { ...p, note: trimmed } : p,
        );
        const config: ProjectConfig = {
          ...cfgResult.data,
          projects: updatedProjects,
          lastUpdated: new Date().toISOString(),
        };
        await tauriAPI.saveProjectConfig(config);
      }
    } catch {
      showMsg("error", t("project.noteSaveFailed"));
    }
  }, [setProjects, showMsg, t]);

  const filteredProjects = useMemo(() => {
    if (!searchText.trim()) return projects;
    const q = searchText.toLowerCase();
    return projects.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.path.toLowerCase().includes(q) ||
        (p.note && p.note.toLowerCase().includes(q)),
    );
  }, [projects, searchText]);

  return (
    <div className="project-manager">

      <ProjectHeader
        loading={loading}
        projectCount={filteredProjects.length}
        totalCount={projects.length}
        searchText={searchText}
        onAdd={handleAdd}
        onRefresh={handleRefresh}
        onSearch={setSearchText}
      />

      {filteredProjects.length > 0 ? (
        <div className="projects-grid">
          {filteredProjects.map((project, i) => (
            <ProjectCard
              key={project.id}
              project={project}
              editors={editors}
              nvmInfo={nvmInfo}
              index={i}
              onScriptChange={handleScriptChange}
              onNodeVersionChange={handleNodeVersionChange}
              onRun={handleRun}
              onRemove={handleRemove}
              onNoteChange={handleNoteChange}
              onOpenEditor={openInEditor}
              showMsg={showMsg}
            />
          ))}
          <div
            className="add-project-card"
            onClick={handleAdd}
            style={{
              animationDelay: `${filteredProjects.length * 50}ms`,
            }}
          >
            <div className="add-icon">
              <PlusOutlined />
            </div>
            <span className="add-label">{t("project.addProject")}</span>
            <span className="add-shortcut">{t("project.addShortcut")}</span>
          </div>
        </div>
      ) : projects.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">
            <FolderOpenOutlined />
          </div>
          <div className="empty-title">{t("project.emptyTitle")}</div>
          <div className="empty-desc">
            {t("project.emptyDesc")}
          </div>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            size="large"
            onClick={handleAdd}
          >
            {t("project.addProject")}
          </Button>
          <span className="empty-shortcut">{t("project.emptyShortcut")}</span>
        </div>
      ) : (
        <div className="empty-state">
          <div className="empty-title">{t("project.noMatch")}</div>
          <div className="empty-desc">{t("project.noMatchDesc")}</div>
        </div>
      )}

    </div>
  );
};

export default ProjectManager;
