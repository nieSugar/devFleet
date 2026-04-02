import React from "react";
import { Select, Typography, Tooltip } from "antd";
import { DeleteOutlined } from "@ant-design/icons";
import { Cursor, Trae, Windsurf, Antigravity } from "@lobehub/icons";
import { Project, NvmInfo, EditorStatus } from "../types/project";
import EditorButton from "./EditorButton";
import NodeVersionSelect from "./NodeVersionSelect";
import vscodeSvg from "../img/vscode.svg";
import vscodeInsidersSvg from "../img/vscode-insiders.svg";
import webstormSvg from "../img/webstorm.svg";
import ideaSvg from "../img/idea.svg";
import zedSvg from "../img/zed.svg";
import kiroSvg from "../img/kiro.svg";
import "./ProjectCard.css";

const EDITOR_ICONS: Record<string, React.ReactNode> = {
  vscode: <img alt="VSCode" src={vscodeSvg} draggable={false} />,
  "vscode-insiders": <img alt="VSCode Insiders" src={vscodeInsidersSvg} draggable={false} />,
  cursor: <Cursor size={16} />,
  windsurf: <Windsurf size={16} />,
  trae: <Trae.Color size={16} />,
  webstorm: <img alt="WebStorm" src={webstormSvg} draggable={false} />,
  idea: <img alt="IntelliJ IDEA" src={ideaSvg} draggable={false} />,
  zed: <img alt="Zed" src={zedSvg} draggable={false} />,
  kiro: <img alt="Kiro" src={kiroSvg} draggable={false} />,
  antigravity: <Antigravity.Color size={16} />,
};

interface ProjectCardProps {
  project: Project;
  editors: EditorStatus | null;
  nvmInfo: NvmInfo | null;
  index: number;
  onScriptChange: (id: string, script: string) => void;
  onNodeVersionChange: (id: string, v: string | null | undefined) => void;
  onRun: (p: Project) => void;
  onRemove: (id: string, name: string) => void;
  onNoteChange: (id: string, note: string) => void;
  onOpenEditor: (
    editor: string,
    path: string,
  ) => Promise<{ success: boolean; error?: string }>;
  showMsg: (type: "success" | "error", text: string) => void;
}

const PM_COLORS: Record<string, string> = {
  npm: "#c4887e",
  yarn: "#7ea8c4",
  pnpm: "#c4a57e",
  bun: "#c47eac",
};

const ProjectCard: React.FC<ProjectCardProps> = React.memo(({
  project,
  editors,
  nvmInfo,
  index,
  onScriptChange,
  onNodeVersionChange,
  onRun,
  onRemove,
  onNoteChange,
  onOpenEditor,
  showMsg,
}) => {
  const pm = project.packageManager || "npm";
  const accent = PM_COLORS[pm] || PM_COLORS.npm;

  const editorAction = async (editor: string) => {
    const r = await onOpenEditor(editor, project.path);
    if (!r.success) showMsg("error", r.error || "打开编辑器失败");
  };

  return (
    <div
      className="project-card"
      style={{ animationDelay: `${index * 50}ms` }}
    >
      <div className="card-accent-bar" style={{ background: accent }} />

      {/* Header */}
      <div className="card-header">
        <div className="card-title-row">
          <h3 className="card-name">{project.name}</h3>
          <span className="card-pm-badge" style={{ color: accent }}>
            {pm}
          </span>
        </div>
        <Tooltip title="删除项目">
          <button
            className="card-delete"
            onClick={() => onRemove(project.id, project.name)}
          >
            <DeleteOutlined />
          </button>
        </Tooltip>
      </div>

      {/* Path */}
      <div className="card-path">
        <Typography.Text
          copyable={{ tooltips: ["复制路径", "已复制"] }}
          className="card-path-text"
          ellipsis={{ tooltip: project.path }}
        >
          {project.path}
        </Typography.Text>
      </div>

      {/* Note */}
      <div className="card-note">
        <Typography.Paragraph
          editable={{
            onChange: (v) => onNoteChange(project.id, v),
            tooltip: "编辑备注",
          }}
          className="card-note-text"
          ellipsis={{ rows: 1, tooltip: true }}
        >
          {project.note || ""}
        </Typography.Paragraph>
      </div>

      <div className="card-divider" />

      {/* Editors + Node */}
      <div className="card-meta">
        <div className="card-editors">
          {editors && Object.entries(editors)
            .filter(([, info]) => info.installed)
            .map(([id, info]) => (
              <EditorButton
                key={id}
                icon={EDITOR_ICONS[id]}
                alt={info.name}
                title={info.name}
                onClick={() => editorAction(id)}
              />
            ))}
        </div>
        {nvmInfo?.isInstalled && (
          <div className="card-node">
            <NodeVersionSelect
              record={project}
              nvmInfo={nvmInfo}
              onChange={onNodeVersionChange}
            />
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="card-actions">
        <Select
          value={project.selectedScript}
          className="card-script-select"
          onChange={(v) => onScriptChange(project.id, v)}
          options={project.scripts.map((s) => ({
            label: s.name,
            value: s.name,
          }))}
          size="small"
          showSearch
          optionFilterProp="label"
          popupMatchSelectWidth={false}
        />
        <button
          className="run-btn"
          disabled={!project.selectedScript}
          onClick={() => onRun(project)}
        >
          <svg
            className="run-btn-icon"
            width="10"
            height="12"
            viewBox="0 0 10 12"
            fill="currentColor"
          >
            <path d="M1 0.5a.5.5 0 01.77-.42l8 5a.5.5 0 010 .84l-8 5A.5.5 0 011 11.5v-11z" />
          </svg>
          <span>运行</span>
        </button>
      </div>
    </div>
  );
});

export default ProjectCard;
