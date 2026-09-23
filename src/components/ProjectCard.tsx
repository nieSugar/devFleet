import React from "react";
import { Select, Typography, Tooltip } from "antd";
import { DeleteOutlined, PushpinOutlined } from "@ant-design/icons";
import { useTranslation } from "react-i18next";
import { Project, NvmInfo, EditorStatus, NodeProcessInfo } from "../types/project";
import EditorButton from "./EditorButton";
import NodeVersionSelect from "./NodeVersionSelect";
import "./ProjectCard.css";


interface ProjectCardProps {
  project: Project;
  editors: EditorStatus | null;
  defaultEditorId: string | null;
  isPinned: boolean;
  pinSaving: boolean;
  onTogglePinned: (project: Project, pinned: boolean) => void;
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
  available: boolean;
  relocating: boolean;
  running: boolean;
  onRelocate: (project: Project) => void;
  processes: NodeProcessInfo[];
  processStatus: "loading" | "ready" | "error";
  onViewProcesses: (project: Project) => void;
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
  defaultEditorId,
  isPinned,
  pinSaving,
  onTogglePinned,
  nvmInfo,
  index,
  onScriptChange,
  onNodeVersionChange,
  onRun,
  onRemove,
  onNoteChange,
  onOpenEditor,
  showMsg,
  available,
  relocating,
  running,
  onRelocate,
  processes,
  processStatus,
  onViewProcesses,
}) => {
  const { t } = useTranslation();
  const pm = project.packageManager || "npm";
  const accent = PM_COLORS[pm] || PM_COLORS.npm;
  const hasValidScript = Boolean(
    project.selectedScript &&
      project.scripts.some((script) => script.name === project.selectedScript),
  );
  const uniquePorts = Array.from(
    new Map(
      processes
        .flatMap((process) => process.ports || [])
        .map((port) => [`${port.protocol}:${port.localPort}`, port]),
    ).values(),
  );
  const portSummary = uniquePorts.slice(0, 4).map((port) => `${port.protocol}:${port.localPort}`);
  const portTooltip = uniquePorts
    .map((port) => `${port.protocol}://${port.localAddress}:${port.localPort}`)
    .join(", ");

  const editorAction = async (editor: string) => {
    const r = await onOpenEditor(editor, project.path);
    if (!r.success) showMsg("error", r.error || t("project.editorFailed"));
  };

  const installedEditors = editors
    ? Object.entries(editors)
        .filter(([, info]) => info.installed)
        .sort(
          ([idA, infoA], [idB, infoB]) =>
            infoA.name.localeCompare(infoB.name) || idA.localeCompare(idB),
        )
    : [];
  const defaultEditor = defaultEditorId
    ? installedEditors.find(([id]) => id === defaultEditorId)
    : undefined;
  const secondaryEditors = installedEditors.filter(([id]) => id !== defaultEditorId);
  const editorControlsDisabled = !available || relocating || running;
  const editorChoiceLabel = defaultEditorId
    ? t("project.defaultEditorUnavailable")
    : t("project.chooseEditor");

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
          <button
            type="button"
            className={`card-pin${isPinned ? " is-pinned" : ""}`}
            aria-label={t(isPinned ? "project.unpin" : "project.pin")}
            aria-pressed={isPinned}
            disabled={pinSaving}
            onClick={() => onTogglePinned(project, !isPinned)}
          >
            <PushpinOutlined />
          </button>
        </div>
        <Tooltip title={t("project.deleteProject")}>
          <button
            className="card-delete"
            aria-label={t("project.deleteProject")}
            onClick={() => onRemove(project.id, project.name)}
          >
            <DeleteOutlined />
          </button>
        </Tooltip>
      </div>

      {/* Path */}
      <div className="card-path">
        <Typography.Text
          copyable={{ tooltips: [t("common.copyPath"), t("common.copied")] }}
          className="card-path-text"
          ellipsis={{ tooltip: project.path }}
        >
          {project.path}
        </Typography.Text>
        {!available && (
          <div className="card-path-unavailable">
            <span>{t("project.pathUnavailable")}</span>
            <button
              className="card-relocate-btn"
              type="button"
              disabled={relocating}
              aria-label={t(relocating ? "project.relocating" : "project.relocate")}
              onClick={() => onRelocate(project)}
            >
              {t(relocating ? "project.relocating" : "project.relocate")}
            </button>
          </div>
        )}
      </div>

      {/* Note */}
      <div className="card-note">
        <Typography.Paragraph
          editable={{
            onChange: (v) => onNoteChange(project.id, v),
            tooltip: t("project.editNote"),
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
          {defaultEditor ? (
            <EditorButton
              icon={defaultEditor[1].icon}
              alt={defaultEditor[1].name}
              title={defaultEditor[1].name}
              onClick={() => editorAction(defaultEditor[0])}
              disabled={editorControlsDisabled}
            />
          ) : (
            <details
              className={`editor-picker${editorControlsDisabled ? " is-disabled" : ""}`}
              onClick={(event) => {
                if (editorControlsDisabled) event.preventDefault();
              }}
            >
              <summary
                className="editor-choice-btn"
                aria-label={editorChoiceLabel}
                aria-disabled={editorControlsDisabled}
                title={editorChoiceLabel}
              >
                {t("project.chooseEditor")}
              </summary>
              <div className="editor-menu">
                {defaultEditorId && (
                  <div className="editor-menu-message" role="status">
                    {t("project.defaultEditorUnavailable")}
                  </div>
                )}
                {secondaryEditors.map(([id, info]) => (
                  <EditorButton
                    key={id}
                    icon={info.icon}
                    alt={info.name}
                    title={info.name}
                    onClick={() => editorAction(id)}
                    disabled={editorControlsDisabled}
                  />
                ))}
              </div>
            </details>
          )}
          {defaultEditor && secondaryEditors.length > 0 && (
            <details
              className={`editor-picker${editorControlsDisabled ? " is-disabled" : ""}`}
              onClick={(event) => {
                if (editorControlsDisabled) event.preventDefault();
              }}
            >
              <summary
                className="editor-more-btn"
                aria-label={t("project.moreEditors")}
                aria-disabled={editorControlsDisabled}
              >
                {t("project.moreEditors")}
              </summary>
              <div className="editor-menu">
                {secondaryEditors.map(([id, info]) => (
                  <EditorButton
                    key={id}
                    icon={info.icon}
                    alt={info.name}
                    title={info.name}
                    onClick={() => editorAction(id)}
                    disabled={editorControlsDisabled}
                  />
                ))}
              </div>
            </details>
          )}
        </div>
        {nvmInfo?.isInstalled && (
          <div className="card-node">
            <NodeVersionSelect
              record={project}
              nvmInfo={nvmInfo}
              onChange={onNodeVersionChange}
              disabled={!available || relocating || running}
            />
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="card-actions">
        <Select
          value={project.selectedScript}
          className="card-script-select"
          disabled={!available || relocating || running}
          onChange={(v) => onScriptChange(project.id, v)}
          options={project.scripts.map((s) => ({
            label: s.name,
            value: s.name,
          }))}
          showSearch
          optionFilterProp="label"
          popupMatchSelectWidth={false}
        />
        <button
          className="run-btn"
          disabled={!available || relocating || running || !hasValidScript}
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
          <span>{t("common.run")}</span>
        </button>
      </div>

      <div className="card-processes" aria-live="polite">
        {processStatus === "loading" && <span>{t("project.processesChecking")}</span>}
        {processStatus === "error" && <span>{t("project.processesUnavailable")}</span>}
        {processStatus === "ready" && (
          <>
            <span>
              {processes.length > 0
                ? t("project.processesDetected", { count: processes.length })
                : t("project.noProcessesDetected")}
            </span>
            {portSummary.length > 0 && (
              <Tooltip title={portTooltip}>
                <span className="card-process-ports">
                  {t("project.ports", { ports: portSummary.join(", ") })}
                </span>
              </Tooltip>
            )}
            {processes.length > 0 && (
              <button
                type="button"
                className="card-view-processes"
                aria-label={t("project.viewProcesses")}
                onClick={() => onViewProcesses(project)}
              >
                {t("project.viewProcesses")}
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
});

export default ProjectCard;
