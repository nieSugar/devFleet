import React, { useState, useEffect, useMemo } from "react";
import { Project, ProjectConfig } from "../types/project";
import { tauriAPI } from "../lib/tauri";
import { useProjects } from "../hooks/useProjects";
import { useEditors } from "../hooks/useEditors";
import { useNvmInfo } from "../hooks/useNvmInfo";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import EditorButton from "./EditorButton";
import NodeVersionSelect from "./NodeVersionSelect";
import ProjectHeader from "./ProjectHeader";
import LogPanel from "./LogPanel";
import "./ProjectManager.css";
import {
  Table,
  Button,
  Space,
  Select,
  Typography,
  message,
  Tag,
  Modal,
  Empty,
} from "antd";
import {
  PlayCircleOutlined,
  DeleteOutlined,
  NodeIndexOutlined,
} from "@ant-design/icons";
import vscodeSvg from "../img/vscode.svg";
import cursorSvg from "../img/cursor.svg";
import webstormSvg from "../img/webstorm.svg";

const ProjectManager: React.FC = () => {
  const {
    projects,
    setProjects,
    loading,
    loadProjects,
    addProject,
    removeProject,
    updateScriptSelection,
    runScript,
  } = useProjects();
  const { editors, openInEditor } = useEditors();
  const { nvmInfo, changeNodeVersion } = useNvmInfo();
  const [messageApi, contextHolder] = message.useMessage();
  const [searchText, setSearchText] = useState("");
  const [activeLogProject, setActiveLogProject] = useState<{
    id: string;
    name: string;
  } | null>(null);

  useEffect(() => {
    loadProjects().then((r) => {
      if (r && !r.success) {
        messageApi.error(r.error || "加载项目配置失败");
      }
    });
  }, [loadProjects, messageApi]);

  useKeyboardShortcuts({
    onAddProject: () => handleAdd(),
    onRefresh: () =>
      loadProjects().then((r) => {
        if (r && !r.success) messageApi.error(r.error || "刷新失败");
      }),
  });

  const showMsg = (type: "success" | "error", text: string) => {
    if (type === "success") messageApi.success(text);
    else messageApi.error(text);
  };

  const handleAdd = async () => {
    const result = await addProject();
    if (!result) return;
    if (result.success && result.data) {
      showMsg("success", `项目 "${result.data.name}" 添加成功`);
    } else {
      showMsg("error", result.error || "添加项目失败");
    }
  };

  const handleRemove = (projectId: string, projectName: string) => {
    Modal.confirm({
      title: "确认删除",
      content: `确定要删除项目「${projectName}」吗？此操作不会删除项目文件。`,
      okText: "删除",
      okType: "danger",
      cancelText: "取消",
      onOk: async () => {
        try {
          const result = await removeProject(projectId);
          if (result.success) {
            showMsg("success", "项目删除成功");
          } else {
            showMsg("error", result.error || "删除项目失败");
          }
        } catch {
          showMsg("error", "删除项目时出错");
        }
      },
    });
  };

  const handleScriptChange = async (projectId: string, scriptName: string) => {
    const result = await updateScriptSelection(projectId, scriptName);
    if (!result.success) {
      showMsg("error", result.error || "保存配置失败");
    }
  };

  const handleNodeVersionChange = async (
    projectId: string,
    nodeVersion: string | null | undefined
  ) => {
    try {
      const result = await changeNodeVersion(projectId, nodeVersion);
      if (result.success && result.data) {
        setProjects((prev) =>
          prev.map((p) =>
            p.id === projectId
              ? { ...p, nodeVersion: nodeVersion || undefined }
              : p
          )
        );
        showMsg("success", result.data.message || "Node 版本已更新");
      } else {
        showMsg("error", result.error || "设置 Node 版本失败");
      }
    } catch {
      showMsg("error", "设置 Node 版本时出错");
    }
  };

  const handleRun = async (project: Project) => {
    const result = await runScript(project);
    if (result.success) {
      const vi = project.nodeVersion ? ` (Node ${project.nodeVersion})` : "";
      showMsg("success", `脚本 "${project.selectedScript}"${vi} 启动成功`);
      if (result.data?.mode === "managed") {
        setActiveLogProject({ id: project.id, name: project.name });
      }
    } else {
      showMsg("error", result.error || "启动脚本失败");
    }
  };

  const handleNoteChange = async (projectId: string, note: string) => {
    const trimmed = note.trim() || undefined;
    setProjects((prev) =>
      prev.map((p) => (p.id === projectId ? { ...p, note: trimmed } : p))
    );
    try {
      const cfgResult = await tauriAPI.loadProjectConfig();
      if (cfgResult.success && cfgResult.data) {
        const updatedProjects = cfgResult.data.projects.map((p: Project) =>
          p.id === projectId ? { ...p, note: trimmed } : p
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
  };

  const filteredProjects = useMemo(() => {
    if (!searchText.trim()) return projects;
    const q = searchText.toLowerCase();
    return projects.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.path.toLowerCase().includes(q) ||
        (p.note && p.note.toLowerCase().includes(q))
    );
  }, [projects, searchText]);

  return (
    <div className="project-manager">
      {contextHolder}
      <ProjectHeader
        loading={loading}
        searchText={searchText}
        onAdd={handleAdd}
        onRefresh={() =>
          loadProjects().then((r) => {
            if (r && !r.success) showMsg("error", r.error || "刷新失败");
          })
        }
        onSearch={setSearchText}
      />

      <div className="projects-container">
        <Table
          rowKey="id"
          dataSource={filteredProjects}
          loading={loading}
          pagination={false}
          locale={{
            emptyText: (
              <Empty description="暂无项目，点击上方「添加项目」开始" />
            ),
          }}
          columns={[
            {
              title: "项目名称",
              dataIndex: "name",
              width: 150,
            },
            {
              title: "项目路径",
              dataIndex: "path",
              render: (text: string, record: Project) => (
                <div
                  style={{ display: "flex", alignItems: "center", gap: 8 }}
                >
                  <Typography.Text copyable ellipsis={{ tooltip: text }}>
                    {text}
                  </Typography.Text>
                  <div
                    style={{ display: "flex", alignItems: "center", gap: 6 }}
                  >
                    {editors?.vscode && (
                      <EditorButton
                        icon={vscodeSvg}
                        alt="VS Code"
                        title="使用 VS Code 打开"
                        onClick={async () => {
                          const res = await openInEditor(
                            "vscode",
                            record.path
                          );
                          if (!res.success)
                            showMsg("error", res.error || "打开 VS Code 失败");
                        }}
                      />
                    )}
                    {editors?.cursor && (
                      <EditorButton
                        icon={cursorSvg}
                        alt="Cursor"
                        title="使用 Cursor 打开"
                        onClick={async () => {
                          const res = await openInEditor(
                            "cursor",
                            record.path
                          );
                          if (!res.success)
                            showMsg("error", res.error || "打开 Cursor 失败");
                        }}
                      />
                    )}
                    {editors?.webstorm && (
                      <EditorButton
                        icon={webstormSvg}
                        alt="WebStorm"
                        title="使用 WebStorm 打开"
                        onClick={async () => {
                          const res = await openInEditor(
                            "webstorm",
                            record.path
                          );
                          if (!res.success)
                            showMsg(
                              "error",
                              res.error || "打开 WebStorm 失败"
                            );
                        }}
                      />
                    )}
                  </div>
                </div>
              ),
            },
            {
              title: "备注",
              dataIndex: "note",
              width: 160,
              render: (_: string | undefined, record: Project) => (
                <Typography.Paragraph
                  editable={{
                    onChange: (val) => handleNoteChange(record.id, val),
                    tooltip: "点击编辑",
                  }}
                  style={{ marginBottom: 0 }}
                  ellipsis={{ rows: 1, tooltip: true }}
                >
                  {record.note || ""}
                </Typography.Paragraph>
              ),
            },
            {
              title: "npm 脚本",
              dataIndex: "scripts",
              width: 130,
              render: (_: Project["scripts"], record: Project) => (
                <Select<string>
                  value={record.selectedScript}
                  style={{ width: 130 }}
                  onChange={(v) => handleScriptChange(record.id, v)}
                  options={record.scripts.map((s) => ({
                    label: s.name,
                    value: s.name,
                  }))}
                />
              ),
            },
            {
              title: () => (
                <Space>
                  <NodeIndexOutlined />
                  Node 版本
                  {nvmInfo?.isInstalled && nvmInfo.manager !== "none" && (
                    <Tag
                      color="blue"
                      style={{ fontSize: 10, marginLeft: 4 }}
                    >
                      {nvmInfo.manager === "nvm-windows"
                        ? "nvm-win"
                        : nvmInfo.manager}
                    </Tag>
                  )}
                </Space>
              ),
              dataIndex: "nodeVersion",
              width: 220,
              render: (_: Project["nodeVersion"], record: Project) => (
                <NodeVersionSelect
                  record={record}
                  nvmInfo={nvmInfo}
                  onChange={handleNodeVersionChange}
                />
              ),
            },
            {
              title: "操作",
              width: 150,
              render: (_: unknown, record: Project) => (
                <Space>
                  <Button
                    type="primary"
                    icon={<PlayCircleOutlined />}
                    disabled={!record.selectedScript}
                    onClick={() => handleRun(record)}
                  >
                    运行
                  </Button>
                  <Button
                    danger
                    icon={<DeleteOutlined />}
                    onClick={() => handleRemove(record.id, record.name)}
                  >
                    删除
                  </Button>
                </Space>
              ),
            },
          ]}
        />
      </div>

      <LogPanel
        projectId={activeLogProject?.id ?? null}
        projectName={activeLogProject?.name ?? ""}
      />
    </div>
  );
};

export default ProjectManager;
