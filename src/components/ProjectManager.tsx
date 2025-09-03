import React, { useState, useEffect } from "react";
import { Project } from "../types/project";
import "./ProjectManager.css";
import { Table, Button, Space, Select, Typography, message } from "antd";
import {
  PlusOutlined,
  ReloadOutlined,
  PlayCircleOutlined,
  DeleteOutlined,
} from "@ant-design/icons";
import vscodeSvg from "../img/vscode.svg";
import cursorSvg from "../img/cursor.svg";
import webstormSvg from "../img/webstorm.svg";

const ProjectManager: React.FC = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);
  const [messageApi, contextHolder] = message.useMessage();
  const [availableEditors, setAvailableEditors] = useState<{
    vscode: boolean;
    cursor: boolean;
    webstorm: boolean;
  } | null>(null);

  // 加载项目配置
  useEffect(() => {
    loadProjects();
    // 检测编辑器
    (async () => {
      try {
        const result = await window.electronAPI.detectEditors();
        console.log(result, "result");

        if (result.success) setAvailableEditors(result.data);
      } catch (e) {}
    })();
  }, []);

  const loadProjects = async () => {
    try {
      const result = await window.electronAPI.loadProjectConfig();
      if (result.success && result.data) {
        setProjects(result.data.projects);
      } else {
        showMessage("error", result.error || "加载项目配置失败");
      }
    } catch (error) {
      showMessage("error", "加载项目配置失败");
    }
  };

  const showMessage = (type: "success" | "error", text: string) => {
    if (type === "success") messageApi.success(text);
    else messageApi.error(text);
  };

  // 添加项目
  const handleAddProject = async () => {
    setLoading(true);
    try {
      const result = await window.electronAPI.selectFolder();

      if (result.success && result.data) {
        const addResult = await window.electronAPI.addProjectToConfig(
          result.data.path
        );
        if (addResult.success && addResult.data) {
          setProjects((prev) => [...prev, addResult.data]);
          showMessage("success", `项目 "${addResult.data.name}" 添加成功`);
        } else {
          showMessage("error", addResult.error || "添加项目失败");
        }
      } else {
        if (result.error && !result.error.includes("用户取消")) {
          showMessage("error", result.error);
        }
      }
    } catch (error) {
      showMessage("error", "添加项目时出错");
    } finally {
      setLoading(false);
    }
  };

  // 删除项目
  const handleRemoveProject = async (projectId: string) => {
    if (window.confirm("确定要删除这个项目吗？")) {
      try {
        const result =
          await window.electronAPI.removeProjectFromConfig(projectId);
        if (result.success) {
          setProjects((prev) => prev.filter((p) => p.id !== projectId));
          showMessage("success", "项目删除成功");
        } else {
          showMessage("error", result.error || "删除项目失败");
        }
      } catch (error) {
        showMessage("error", "删除项目时出错");
      }
    }
  };

  // 选择脚本
  const handleScriptChange = async (projectId: string, scriptName: string) => {
    // 更新本地状态
    const updatedProjects = projects.map((project) =>
      project.id === projectId
        ? { ...project, selectedScript: scriptName }
        : project
    );
    setProjects(updatedProjects);

    // 保存配置到主进程
    try {
      const config = {
        projects: updatedProjects,
        lastUpdated: new Date(),
      };
      await window.electronAPI.saveProjectConfig(config);
    } catch (error) {
      console.error("保存配置失败:", error);
    }
  };

  // 运行脚本
  const handleRunScript = async (project: Project) => {
    if (!project.selectedScript) {
      showMessage("error", "请先选择要运行的脚本");
      return;
    }

    setLoading(true);
    try {
      const result = await window.electronAPI.runScript({
        projectPath: project.path,
        scriptName: project.selectedScript,
        projectId: project.id,
      });

      if (result.success) {
        showMessage("success", `脚本 "${project.selectedScript}" 启动成功`);
      } else {
        showMessage("error", result.error || "启动脚本失败");
      }
    } catch (error) {
      showMessage("error", "启动脚本时出错");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="project-manager">
      {contextHolder}
      <div className="project-manager-header">
        <h2>项目管理</h2>
        <Space>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={handleAddProject}
            loading={loading}
          >
            添加项目
          </Button>
          <Button
            icon={<ReloadOutlined />}
            onClick={loadProjects}
            disabled={loading}
          >
            刷新
          </Button>
        </Space>
      </div>

      <div className="projects-container">
        <Table
          rowKey="id"
          dataSource={projects}
          pagination={false}
          columns={[
            {
              title: "项目名称",
              dataIndex: "name",
              width: 150,
            },
            {
              title: "项目路径",
              dataIndex: "path",
              render: (text: string, record: Project) => {
                const showVSCode = availableEditors?.vscode;
                const showCursor = availableEditors?.cursor;
                const showWebStorm = availableEditors?.webstorm;
                return (
                  <div
                    style={{ display: "flex", alignItems: "center", gap: 8 }}
                  >
                    <Typography.Text copyable ellipsis={{ tooltip: text }}>
                      {text}
                    </Typography.Text>
                    <div
                      style={{ display: "flex", alignItems: "center", gap: 6 }}
                    >
                      {showVSCode && (
                        <Button
                          size="small"
                          type="text"
                          title="使用 VS Code 打开"
                          onClick={async () => {
                            const res = await window.electronAPI.openInEditor({
                              editor: "vscode",
                              projectPath: record.path,
                            });
                            if (!res.success)
                              showMessage(
                                "error",
                                res.error || "打开 VS Code 失败"
                              );
                          }}
                        >
                          <img
                            alt="VS Code"
                            src={vscodeSvg}
                            style={{ width: 16, height: 16 }}
                          />
                        </Button>
                      )}
                      {showCursor && (
                        <Button
                          size="small"
                          type="text"
                          title="使用 Cursor 打开"
                          onClick={async () => {
                            const res = await window.electronAPI.openInEditor({
                              editor: "cursor",
                              projectPath: record.path,
                            });
                            if (!res.success)
                              showMessage(
                                "error",
                                res.error || "打开 Cursor 失败"
                              );
                          }}
                        >
                          <img
                            alt="Cursor"
                            src={cursorSvg}
                            style={{ width: 16, height: 16 }}
                          />
                        </Button>
                      )}
                      {showWebStorm && (
                        <Button
                          size="small"
                          type="text"
                          title="使用 WebStorm 打开"
                          onClick={async () => {
                            const res = await window.electronAPI.openInEditor({
                              editor: "webstorm",
                              projectPath: record.path,
                            });
                            if (!res.success)
                              showMessage(
                                "error",
                                res.error || "打开 WebStorm 失败"
                              );
                          }}
                        >
                          <img
                            alt="WebStorm"
                            src={webstormSvg}
                            style={{ width: 16, height: 16 }}
                          />
                        </Button>
                      )}
                    </div>
                  </div>
                );
              },
            },
            {
              title: "npm 脚本",
              dataIndex: "scripts",
              width: 150,
              render: (_: any, record: Project) => (
                <Select
                  value={record.selectedScript}
                  style={{ width: 150 }}
                  onChange={(v) => handleScriptChange(record.id, v)}
                  options={record.scripts.map((s) => ({
                    label: s.name,
                    value: s.name,
                  }))}
                />
              ),
            },
            {
              title: "操作",
              width: 150,
              render: (_: any, record: Project) => (
                <Space>
                  <Button
                    type="primary"
                    icon={<PlayCircleOutlined />}
                    disabled={!record.selectedScript}
                    onClick={() => handleRunScript(record)}
                  >
                    运行
                  </Button>
                  <Button
                    danger
                    icon={<DeleteOutlined />}
                    onClick={() => handleRemoveProject(record.id)}
                  >
                    删除
                  </Button>
                </Space>
              ),
            },
          ]}
        />
      </div>
    </div>
  );
};

export default ProjectManager;
