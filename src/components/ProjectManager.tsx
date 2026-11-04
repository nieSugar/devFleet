import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { NodeProcessInfo, Project } from "../types/project";
import type { NodeProcessesState } from "../hooks/useNodeProcesses";
import { listenForMacOSAddProject } from "../lib/macosNative";
import { listenForProjectsChanged } from "../lib/projectEvents";
import { tauriAPI } from "../lib/tauri";
import { useProjects } from "../hooks/useProjects";
import { useEditors } from "../hooks/useEditors";
import { useNvmInfo } from "../hooks/useNvmInfo";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import ProjectHeader from "./ProjectHeader";
import ProjectImportModal from "./ProjectImportModal";
import ProjectCard from "./ProjectCard";
import { PlusOutlined, FolderOpenOutlined } from "@ant-design/icons";
import { App, Button, Typography, type InputRef } from "antd";
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
  const [projectFilter, setProjectFilter] = useState<"all" | "processes" | "unavailable">("all");
  const [compact, setCompact] = useState(false);
  const [searchFocused, setSearchFocused] = useState(false);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const searchRef = useRef<InputRef>(null);
  const openingEditor = useRef(false);
  const [importOpen, setImportOpen] = useState(false);
  const [relocatingId, setRelocatingId] = useState<string | null>(null);
  const [runningIds, setRunningIds] = useState<Set<string>>(new Set());
  const pendingRuns = useRef(new Set<string>());
  const pendingPins = useRef(new Set<string>());
  const [pinningIds, setPinningIds] = useState<Set<string>>(new Set());
  const pendingNodeChanges = useRef(new Set<string>());
  const [nodeSavingIds, setNodeSavingIds] = useState<Set<string>>(new Set());
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
    onFocusSearch: () => searchRef.current?.focus({ cursor: "all" }),
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
    if (pendingNodeChanges.current.has(projectId) || pendingRuns.current.has(projectId)) return;
    pendingNodeChanges.current.add(projectId);
    setNodeSavingIds(new Set(pendingNodeChanges.current));
    try {
      const preview = await tauriAPI.getProjectNodeVersionFile(projectId);
      if (!preview.success || !preview.data) {
        showMsg("error", preview.error || t("project.nodeVersionFailed"));
        return;
      }
      const confirmed = await modal.confirm({
        title: t(nodeVersion ? "project.nodeVersionWriteTitle" : "project.nodeVersionRemoveTitle"),
        content: <>
          <p>{t(nodeVersion ? "project.nodeVersionWriteContent" : "project.nodeVersionRemoveContent", { version: nodeVersion })}</p>
          <Typography.Text code copyable style={{ overflowWrap: "anywhere" }}>{preview.data.filePath}</Typography.Text>
        </>,
        okText: t("common.confirm"),
        okType: nodeVersion ? "primary" : "danger",
        cancelText: t("common.cancel"),
      });
      if (!confirmed) return;
      const result = await changeNodeVersion(projectId, nodeVersion, preview.data.filePath);
      if (result.success && result.data) {
        const savedVersion = result.data.project.nodeVersion;
        setProjects((prev) =>
          prev.map((p) =>
            p.id === projectId
              ? { ...p, nodeVersion: savedVersion }
              : p,
          ),
        );
        showMsg("success", result.data.message || t("project.nodeVersionUpdated"));
      } else {
        showMsg("error", result.error || t("project.nodeVersionFailed"));
      }
    } catch {
      showMsg("error", t("project.nodeVersionError"));
    } finally {
      pendingNodeChanges.current.delete(projectId);
      setNodeSavingIds(new Set(pendingNodeChanges.current));
    }
  }, [changeNodeVersion, modal, setProjects, showMsg, t]);

  const handleRun = useCallback(async (project: Project) => {
    if (pendingRuns.current.has(project.id) || pendingNodeChanges.current.has(project.id)) return;
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
    const q = searchText.trim().toLowerCase();
    return projects.filter((p) => {
      if (projectFilter === "processes" && !processesByProject.has(p.id)) return false;
      if (projectFilter === "unavailable" && availability[p.id] !== false) return false;
      return !q || p.name.toLowerCase().includes(q) ||
        p.path.toLowerCase().includes(q) ||
        Boolean(p.note && p.note.toLowerCase().includes(q));
    });
  }, [projects, searchText, projectFilter, processesByProject, availability]);

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

  const selectedProject = orderedProjects.find((project) => project.id === selectedProjectId) || orderedProjects[0];
  useEffect(() => {
    if (!searchFocused || !selectedProject) return;
    document.getElementById(`project-card-${selectedProject.id}`)?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [searchFocused, selectedProject]);

  const handleSearchKeyDown = async (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (event.key === "Escape") {
      event.currentTarget.blur();
      return;
    }
    if (!selectedProject) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const index = orderedProjects.indexOf(selectedProject);
      const next = (index + (event.key === "ArrowDown" ? 1 : -1) + orderedProjects.length) % orderedProjects.length;
      setSelectedProjectId(orderedProjects[next].id);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (event.repeat || openingEditor.current || relocatingId === selectedProject.id || runningIds.has(selectedProject.id)) return;
      if (availability[selectedProject.id] !== true) {
        showMsg("error", t("project.pathUnavailable"));
        return;
      }
      if (!defaultEditorId || !editors?.[defaultEditorId]?.installed) {
        const picker = document.getElementById(`project-card-${selectedProject.id}`)?.querySelector<HTMLDetailsElement>(".editor-picker");
        if (picker) {
          picker.open = true;
          picker.querySelector("summary")?.focus();
        }
        return;
      }
      openingEditor.current = true;
      try {
        const result = await openInEditor(defaultEditorId, selectedProject.path);
        if (!result.success) showMsg("error", result.error || t("project.editorFailed"));
      } catch {
        showMsg("error", t("project.editorFailed"));
      } finally {
        openingEditor.current = false;
      }
    }
  };

  return (
    <div className="project-manager">

      <ProjectHeader
        loading={loading}
        projectCount={filteredProjects.length}
        totalCount={projects.length}
        searchText={searchText}
        searchRef={searchRef}
        onSearchKeyDown={handleSearchKeyDown}
        onSearchFocus={() => setSearchFocused(true)}
        onSearchBlur={() => setSearchFocused(false)}
        projectFilter={projectFilter}
        onFilterChange={(filter) => { setProjectFilter(filter); setSelectedProjectId(null); }}
        compact={compact}
        onCompactChange={setCompact}
        onAdd={handleAdd}
        onImport={() => setImportOpen(true)}
        onRefresh={handleRefresh}
        onSearch={(text) => { setSearchText(text); setSelectedProjectId(null); }}
      />

      <span id="project-search-selection" className="project-sr-only" role="status">
        {searchFocused && selectedProject ? t("project.searchSelection", { name: selectedProject.name }) : ""}
      </span>
      {projectFilter === "processes" && processState.status !== "ready" && (
        <div className="project-filter-status" role="status">
          {t(processState.status === "loading" ? "project.processesChecking" : "project.processesUnavailable")}
          {processState.status === "error" && <Button size="small" onClick={() => void refreshProcesses(true)}>{t("common.retry")}</Button>}
        </div>
      )}

      {filteredProjects.length > 0 ? (
        <div id="project-results" className={`projects-grid${compact ? " projects-compact" : ""}`}>
          {orderedProjects.map((project, i) => (
            <ProjectCard
              key={project.id}
              project={project}
              editors={editors}
              defaultEditorId={defaultEditorId}
              isPinned={pinnedProjectIds.includes(project.id)}
              pinSaving={pinningIds.has(project.id)}
              nodeSaving={nodeSavingIds.has(project.id)}
              selected={searchFocused && selectedProject?.id === project.id}
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
              portsError={processState.portsError}
              onRetryProcesses={() => void refreshProcesses(true)}
              onViewProcesses={onViewProcesses}
            />
          ))}
          <button
            type="button"
            className="add-project-card"
            onClick={handleAdd}
            style={{
              animationDelay: `${Math.min(filteredProjects.length * 50, 300)}ms`,
            }}
          >
            <div className="add-icon">
              <PlusOutlined />
            </div>
            <span className="add-label">{t("project.addProject")}</span>
            <span className="add-shortcut">{t("project.addShortcut")}</span>
          </button>
        </div>
      ) : projects.length === 0 ? (
        <div id="project-results" className="empty-state">
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
      ) : projectFilter === "processes" && processState.status !== "ready" ? (
        <div id="project-results" />
      ) : (
        <div id="project-results" className="empty-state">
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
