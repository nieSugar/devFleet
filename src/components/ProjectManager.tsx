import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { NodeProcessInfo, Project } from "../types/project";
import type { NodeProcessesState } from "../hooks/useNodeProcesses";
import { listenForMacOSAddProject } from "../lib/macosNative";
import { listenForProjectsChanged } from "../lib/projectEvents";
import { useProjects } from "../hooks/useProjects";
import { useEditors } from "../hooks/useEditors";
import { useNvmInfo } from "../hooks/useNvmInfo";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import ProjectHeader from "./ProjectHeader";
import ProjectImportModal from "./ProjectImportModal";
import ProjectCard from "./ProjectCard";
import { PlusOutlined, FolderOpenOutlined } from "@ant-design/icons";
import { App, Button } from "antd";
import "./ProjectManager.css";

interface ProjectManagerProps {
  nvmRefreshKey?: number;
  processState: NodeProcessesState;
  onOpenNodeManager: () => void;
  onViewProcesses: (project: Project) => void;
}

const ProjectManager: React.FC<ProjectManagerProps> = ({ nvmRefreshKey, processState, onOpenNodeManager, onViewProcesses }) => {
  const {
    projects,
    availability,
    pinnedProjectIds,
    setProjects,
    loading,
    loadProjects,
    refreshProjects,
    addProject,
    removeProject,
    updateScriptSelection,
    updateNote,
    runScript,
    relocateProject,
    togglePinned,
  } = useProjects();
  const { editors, defaultEditorId, openInEditor, refreshEditors } = useEditors();
  const { nvmInfo, changeNodeVersion, refreshNvmInfo } = useNvmInfo();
  const { modal, message: messageApi } = App.useApp();
  const { t } = useTranslation();
  const [searchText, setSearchText] = useState("");
  const [importOpen, setImportOpen] = useState(false);
  const [relocatingId, setRelocatingId] = useState<string | null>(null);
  const [runningIds, setRunningIds] = useState<Set<string>>(new Set());
  const pendingRuns = useRef(new Set<string>());
  const pendingPins = useRef(new Set<string>());
  const [pinningIds, setPinningIds] = useState<Set<string>>(new Set());
  const refreshProcesses = processState.refresh;

  const processesByProject = useMemo(() => {
    const grouped = new Map<string, NodeProcessInfo[]>();
    for (const process of processState.processes) {
      if (!process.matchedProjectId) continue;
      const group = grouped.get(process.matchedProjectId) || [];
      group.push(process);
      grouped.set(process.matchedProjectId, group);
    }
    return grouped;
  }, [processState.processes]);

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

  const handleImported = useCallback(() => {
    refreshProjects().then((r) => {
      if (r && !r.success) messageApi.error(r.error || t("project.refreshFailed"));
    });
  }, [refreshProjects, messageApi, t]);

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

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void listenForProjectsChanged(() => {
      if (cancelled) return;
      loadProjects().then((r) => {
        if (!cancelled && !r.success)
          messageApi.error(r.error || t("project.loadFailed"));
      });
    }).then((cleanup) => {
      if (cancelled) cleanup();
      else unlisten = cleanup;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [loadProjects, messageApi, t]);

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
    if (pendingRuns.current.has(project.id)) return;
    pendingRuns.current.add(project.id);
    setRunningIds(new Set(pendingRuns.current));
    try {
      const result = await runScript(project);
      if (result.success) {
        const environment = result.data?.nodeVersion
          ? t("project.specifiedEnvironment", { version: result.data.nodeVersion })
          : t("project.defaultEnvironment");
        showMsg("success", t("project.scriptStarted", { script: project.selectedScript, environment }));
        void refreshProcesses(true);
      } else if (result.code?.startsWith("NODE_VERSION_")) {
        modal.confirm({
          title: t("project.nodeCheckTitle"),
          content: <><p>{t(`project.nodeErrors.${result.code}`, { version: project.nodeVersion, defaultValue: result.error })}</p><p>{t("project.nodeCheckHelp")}</p></>,
          okText: t("project.manageNode"),
          cancelText: t("common.close"),
          onOk: onOpenNodeManager,
        });
      } else {
        showMsg("error", result.error || t("project.scriptFailed"));
        if (result.code === "PROJECT_UNAVAILABLE") void loadProjects();
      }
    } finally {
      pendingRuns.current.delete(project.id);
      setRunningIds(new Set(pendingRuns.current));
    }
  }, [runScript, showMsg, t, refreshProcesses, modal, onOpenNodeManager, loadProjects]);

  const handleRelocate = useCallback(async (project: Project) => {
    if (relocatingId !== null) return;
    setRelocatingId(project.id);
    try {
      const result = await relocateProject(project.id);
      if (!result) return;
      if (result.success && result.data) {
        showMsg("success", t("project.relocateSuccess"));
        if (project.selectedScript && !result.data.selectedScript) messageApi.warning(t("project.scriptRemoved"));
        void refreshProcesses(true);
      } else {
        showMsg("error", result.error || t("project.relocateFailed"));
      }
    } finally {
      setRelocatingId(null);
    }
  }, [relocatingId, relocateProject, showMsg, t, messageApi, refreshProcesses]);

  const handleTogglePinned = useCallback(async (project: Project, pinned: boolean) => {
    if (pendingPins.current.has(project.id)) return;
    pendingPins.current.add(project.id);
    setPinningIds(new Set(pendingPins.current));
    try {
      const result = await togglePinned(project.id, pinned);
      if (!result.success) messageApi.error(result.error || t("project.pinFailed"));
    } finally {
      pendingPins.current.delete(project.id);
      setPinningIds(new Set(pendingPins.current));
    }
  }, [messageApi, t, togglePinned]);

  const handleNoteChange = useCallback(async (projectId: string, note: string) => {
    const result = await updateNote(projectId, note);
    if (!result.success) showMsg("error", result.error || t("project.noteSaveFailed"));
  }, [updateNote, showMsg, t]);

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

  const orderedProjects = useMemo(() => {
    const pinned = new Set(pinnedProjectIds);
    return filteredProjects
      .map((project, index) => ({ project, index }))
      .sort((a, b) =>
        Number(pinned.has(b.project.id)) - Number(pinned.has(a.project.id)) ||
        a.index - b.index,
      )
      .map(({ project }) => project);
  }, [filteredProjects, pinnedProjectIds]);

  return (
    <div className="project-manager">

      <ProjectHeader
        loading={loading}
        projectCount={filteredProjects.length}
        totalCount={projects.length}
        searchText={searchText}
        onAdd={handleAdd}
        onImport={() => setImportOpen(true)}
        onRefresh={handleRefresh}
        onSearch={setSearchText}
      />

      {filteredProjects.length > 0 ? (
        <div className="projects-grid">
          {orderedProjects.map((project, i) => (
            <ProjectCard
              key={project.id}
              project={project}
              editors={editors}
              defaultEditorId={defaultEditorId}
              isPinned={pinnedProjectIds.includes(project.id)}
              pinSaving={pinningIds.has(project.id)}
              onTogglePinned={handleTogglePinned}
              nvmInfo={nvmInfo}
              index={i}
              onScriptChange={handleScriptChange}
              onNodeVersionChange={handleNodeVersionChange}
              onRun={handleRun}
              onRemove={handleRemove}
              onNoteChange={handleNoteChange}
              onOpenEditor={openInEditor}
              showMsg={showMsg}
              available={availability[project.id] === true}
              relocating={relocatingId === project.id}
              running={runningIds.has(project.id)}
              onRelocate={handleRelocate}
              processes={processesByProject.get(project.id) || []}
              processStatus={processState.status}
              onViewProcesses={onViewProcesses}
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

      <ProjectImportModal
        open={importOpen}
        onClose={() => setImportOpen(false)}
        onImported={handleImported}
      />

    </div>
  );
};

export default ProjectManager;
