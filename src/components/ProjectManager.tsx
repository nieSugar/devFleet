import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { Project, ProjectConfig } from "../types/project";
import { tauriAPI } from "../lib/tauri";
import { useProjects } from "../hooks/useProjects";
import { useEditors } from "../hooks/useEditors";
import { useNvmInfo } from "../hooks/useNvmInfo";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import ProjectHeader from "./ProjectHeader";
import ProjectCard from "./ProjectCard";
import { PlusOutlined, FolderOpenOutlined } from "@ant-design/icons";
import { message, Modal, Button } from "antd";
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
  const [messageApi, contextHolder] = message.useMessage();
  const [searchText, setSearchText] = useState("");

  useEffect(() => {
    loadProjects().then((r) => {
      if (r && !r.success) messageApi.error(r.error || "加载项目配置失败");
    });
  }, [loadProjects, messageApi]);

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
      if (r && !r.success) messageApi.error(r.error || "刷新失败");
    });
  }, [refreshEditors, refreshProjects, messageApi]);

  useKeyboardShortcuts({
    onAddProject: () => handleAdd(),
    onRefresh: handleRefresh,
  });

  const showMsg = useCallback((type: "success" | "error", text: string) => {
    if (type === "success") messageApi.success(text);
    else messageApi.error(text);
  }, [messageApi]);

  const handleAdd = async () => {
    const result = await addProject();
    if (!result) return;
    if (result.success && result.data)
      showMsg("success", `项目 "${result.data.name}" 添加成功`);
    else showMsg("error", result.error || "添加项目失败");
  };

  const handleRemove = useCallback((projectId: string, projectName: string) => {
    Modal.confirm({
      title: "确认删除",
      content: `确定要删除项目「${projectName}」吗？此操作不会删除项目文件。`,
      okText: "删除",
      okType: "danger",
      cancelText: "取消",
      onOk: async () => {
        try {
          const result = await removeProject(projectId);
          if (result.success) showMsg("success", "项目删除成功");
          else showMsg("error", result.error || "删除项目失败");
        } catch {
          showMsg("error", "删除项目时出错");
        }
      },
    });
  }, [removeProject, showMsg]);

  const handleScriptChange = useCallback(async (projectId: string, scriptName: string) => {
    const result = await updateScriptSelection(projectId, scriptName);
    if (!result.success) showMsg("error", result.error || "保存配置失败");
  }, [updateScriptSelection, showMsg]);

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
        showMsg("success", result.data.message || "Node 版本已更新");
      } else {
        showMsg("error", result.error || "设置 Node 版本失败");
      }
    } catch {
      showMsg("error", "设置 Node 版本时出错");
    }
  }, [changeNodeVersion, setProjects, showMsg]);

  const handleRun = useCallback(async (project: Project) => {
    const result = await runScript(project);
    if (result.success) {
      const vi = project.nodeVersion ? ` (Node ${project.nodeVersion})` : "";
      showMsg("success", `脚本 "${project.selectedScript}"${vi} 启动成功`);
    } else {
      showMsg("error", result.error || "启动脚本失败");
    }
  }, [runScript, showMsg]);

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
      showMsg("error", "保存备注失败");
    }
  }, [setProjects, showMsg]);

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
      {contextHolder}

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
            <span className="add-label">添加项目</span>
            <span className="add-shortcut">Ctrl + N</span>
          </div>
        </div>
      ) : projects.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">
            <FolderOpenOutlined />
          </div>
          <div className="empty-title">还没有项目</div>
          <div className="empty-desc">
            添加你的第一个项目，开始高效管理开发工作流
          </div>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            size="large"
            onClick={handleAdd}
          >
            添加项目
          </Button>
          <span className="empty-shortcut">或按 Ctrl + N</span>
        </div>
      ) : (
        <div className="empty-state">
          <div className="empty-title">没有匹配的项目</div>
          <div className="empty-desc">尝试其他搜索关键词</div>
        </div>
      )}

    </div>
  );
};

export default ProjectManager;
