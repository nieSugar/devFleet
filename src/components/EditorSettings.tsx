import React, { useMemo, useState } from "react";
import { App, Button, Input, Modal } from "antd";
import {
  AppstoreAddOutlined,
  DeleteOutlined,
  EditOutlined,
  FolderOpenOutlined,
  ImportOutlined,
  PlusOutlined,
  ReloadOutlined,
} from "@ant-design/icons";
import { useTranslation } from "react-i18next";
import { useEditors } from "../hooks/useEditors";
import { tauriAPI } from "../lib/tauri";
import type {
  EditorCandidate,
  EditorCandidateDiscovery,
  EditorInfo,
} from "../types/project";
import "./EditorSettings.css";

interface EditorDraft {
  id?: string;
  name: string;
  path: string;
  pathChanged: boolean;
  argsText: string;
  canEditArgs: boolean;
}

interface CandidateImportDraft {
  id: string;
  name: string;
}

const defaultName = (path: string) => {
  const leaf = path.split(/[\\/]/).filter(Boolean).pop() || "";
  return leaf.replace(/\.(exe|app|desktop)$/i, "");
};

const canEditPathArgs = (path: string) => !/\.(app|desktop)$/i.test(path.trim());

const EditorGlyph: React.FC<Pick<EditorInfo, "icon" | "name">> = ({ icon, name }) => {
  const [failedIcon, setFailedIcon] = useState<string | null>(null);
  const fallback = name.trim().slice(0, 2).toUpperCase() || "IDE";

  return (
    <span className="editor-settings-icon" aria-hidden="true">
      {icon && failedIcon !== icon ? (
        <img src={icon} alt="" draggable={false} onError={() => setFailedIcon(icon)} />
      ) : (
        fallback
      )}
    </span>
  );
};

const EditorSettings: React.FC = () => {
  const { t } = useTranslation();
  const { message, modal } = App.useApp();
  const {
    editors,
    error,
    loading,
    refreshEditors,
    upsertCustomEditor,
    removeCustomEditor,
  } = useEditors();
  const [draft, setDraft] = useState<EditorDraft | null>(null);
  const [saving, setSaving] = useState(false);
  const [candidateDiscovery, setCandidateDiscovery] =
    useState<EditorCandidateDiscovery | null>(null);
  const [candidateMode, setCandidateMode] = useState<"recommended" | "all">(
    "recommended",
  );
  const [candidateQuery, setCandidateQuery] = useState("");
  const [candidateScanning, setCandidateScanning] = useState(false);
  const [importDraft, setImportDraft] = useState<CandidateImportDraft | null>(null);
  const [candidateImporting, setCandidateImporting] = useState<string | null>(null);

  const entries = useMemo(
    () =>
      Object.entries(editors || {}).sort(
        ([idA, infoA], [idB, infoB]) =>
          infoA.name.localeCompare(infoB.name) || idA.localeCompare(idB)
      ),
    [editors]
  );

  const filteredCandidates = useMemo(() => {
    const query = candidateQuery.trim().toLocaleLowerCase();
    return (candidateDiscovery?.candidates || []).filter((candidate) => {
      if (candidateMode === "recommended" && !candidate.recommended) return false;
      return (
        !query ||
        candidate.name.toLocaleLowerCase().includes(query) ||
        candidate.path.toLocaleLowerCase().includes(query)
      );
    });
  }, [candidateDiscovery, candidateMode, candidateQuery]);

  // ponytail: search all 1000 records, render 200; add virtualization only if profiling needs it.
  const visibleCandidates = filteredCandidates.slice(0, 200);

  const chooseProgram = async () => {
    const path = await tauriAPI.selectEditor(
      t("settings.editors.chooseProgramTitle"),
    );
    if (!path) return null;
    return path;
  };

  const handleAdd = async () => {
    const path = await chooseProgram();
    if (!path) return;
    setDraft({
      name: defaultName(path),
      path,
      pathChanged: true,
      argsText: "",
      canEditArgs: canEditPathArgs(path),
    });
  };

  const handleEdit = (id: string, editor: EditorInfo) => {
    setDraft({
      id,
      name: editor.name,
      path: editor.path || "",
      pathChanged: false,
      argsText: editor.args.join("\n"),
      canEditArgs: editor.canEditArgs,
    });
  };

  const handleChangeProgram = async () => {
    const path = await chooseProgram();
    if (!path) return;
    setDraft((current) =>
      current
        ? {
            ...current,
            path,
            pathChanged: true,
            canEditArgs: canEditPathArgs(path),
            argsText: canEditPathArgs(path) ? current.argsText : "",
          }
        : current
    );
  };

  const handleSave = async () => {
    if (!draft || !draft.path.trim()) {
      message.error(t("settings.editors.pathRequired"));
      return;
    }
    setSaving(true);
    try {
      const result = await upsertCustomEditor({
        id: draft.id,
        name: draft.name,
        path: draft.id && !draft.pathChanged ? undefined : draft.path,
        args: draft.canEditArgs
          ? draft.argsText.split(/\r?\n/).filter((line) => line.length > 0)
          : [],
      });
      if (!result.success) throw new Error(result.error || "Save failed");
      message.success(t("settings.editors.saveSuccess"));
      setDraft(null);
    } catch (saveError) {
      console.warn("[settings] failed to save editor", saveError);
      message.error(
        saveError instanceof Error ? saveError.message : t("settings.editors.saveFailed")
      );
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = (id: string, name: string) => {
    modal.confirm({
      title: t("settings.editors.deleteTitle", { name }),
      content: t("settings.editors.deleteDesc"),
      okText: t("common.delete"),
      okButtonProps: { danger: true },
      cancelText: t("common.cancel"),
      onOk: async () => {
        try {
          const result = await removeCustomEditor(id);
          if (!result.success) {
            throw new Error(result.error || t("settings.editors.deleteFailed"));
          }
          message.success(t("settings.editors.deleteSuccess"));
        } catch (deleteError) {
          message.error(
            deleteError instanceof Error
              ? deleteError.message
              : t("settings.editors.deleteFailed")
          );
          throw deleteError;
        }
      },
    });
  };

  const handleDiscoverCandidates = async () => {
    setCandidateScanning(true);
    try {
      const result = await tauriAPI.discoverEditorCandidates();
      if (!result.success || !result.data) {
        throw new Error(result.error || t("settings.editors.candidates.scanFailed"));
      }
      setCandidateDiscovery(result.data);
      setCandidateMode("recommended");
      setCandidateQuery("");
    } catch (scanError) {
      console.warn("[settings] failed to discover editor candidates", scanError);
      message.error(
        scanError instanceof Error
          ? scanError.message
          : t("settings.editors.candidates.scanFailed"),
      );
    } finally {
      setCandidateScanning(false);
    }
  };

  const handleImportCandidate = async () => {
    if (!importDraft) return;
    setCandidateImporting(importDraft.id);
    try {
      const result = await tauriAPI.importEditorCandidate({
        candidateId: importDraft.id,
        name: importDraft.name,
      });
      if (!result.success) {
        throw new Error(result.error || t("settings.editors.candidates.importFailed"));
      }
      setCandidateDiscovery((current) =>
        current
          ? {
              ...current,
              candidates: current.candidates.map((candidate) =>
                candidate.id === importDraft.id
                  ? { ...candidate, added: true }
                  : candidate,
              ),
            }
          : current,
      );
      await refreshEditors();
      message.success(t("settings.editors.candidates.importSuccess"));
      setImportDraft(null);
    } catch (importError) {
      console.warn("[settings] failed to import editor candidate", importError);
      message.error(
        importError instanceof Error
          ? importError.message
          : t("settings.editors.candidates.importFailed"),
      );
    } finally {
      setCandidateImporting(null);
    }
  };

  return (
    <section className="settings-section editor-settings-section">
      <div className="settings-section-head editor-settings-heading">
        <div>
          <h2 className="settings-section-title">{t("settings.editors.title")}</h2>
          <p className="settings-section-desc">{t("settings.editors.description")}</p>
        </div>
        <div className="editor-settings-actions">
          <Button
            icon={<ReloadOutlined />}
            loading={loading}
            onClick={() => void refreshEditors()}
          >
            {t("settings.editors.rescan")}
          </Button>
          <Button
            icon={<AppstoreAddOutlined />}
            loading={candidateScanning}
            onClick={() => void handleDiscoverCandidates()}
          >
            {t("settings.editors.candidates.scan")}
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => void handleAdd()}>
            {t("settings.editors.add")}
          </Button>
        </div>
      </div>

      {error && <div className="editor-settings-error">{error}</div>}
      <div className="editor-settings-list" aria-busy={loading}>
        {!loading && entries.length === 0 && (
          <div className="editor-settings-empty">{t("settings.editors.empty")}</div>
        )}
        {entries.map(([id, editor]) => (
          <article className="editor-settings-item" key={id}>
            <EditorGlyph icon={editor.icon} name={editor.name} />
            <div className="editor-settings-copy">
              <div className="editor-settings-name-row">
                <strong>{editor.name}</strong>
                <span className="editor-settings-source">
                  {t(`settings.editors.source.${editor.source}`)}
                </span>
                <span
                  className="editor-settings-status"
                  data-available={editor.installed}
                >
                  {editor.installed
                    ? t("settings.editors.available")
                    : t("settings.editors.unavailable")}
                </span>
              </div>
              <span className="editor-settings-path" title={editor.path}>
                {editor.path || t("settings.editors.noPath")}
              </span>
            </div>
            {editor.source === "custom" && (
              <div className="editor-settings-item-actions">
                <Button
                  aria-label={t("settings.editors.edit")}
                  icon={<EditOutlined />}
                  onClick={() => handleEdit(id, editor)}
                />
                <Button
                  danger
                  aria-label={t("common.delete")}
                  icon={<DeleteOutlined />}
                  onClick={() => handleDelete(id, editor.name)}
                />
              </div>
            )}
          </article>
        ))}
      </div>

      {candidateDiscovery && (
        <div className="editor-candidates">
          <div className="editor-candidates-head">
            <div>
              <h3>{t("settings.editors.candidates.title")}</h3>
              <p>{t("settings.editors.candidates.disclaimer")}</p>
            </div>
            <div className="editor-candidates-modes" role="group">
              <button
                type="button"
                data-active={candidateMode === "recommended"}
                onClick={() => setCandidateMode("recommended")}
              >
                {t("settings.editors.candidates.recommended")}
              </button>
              <button
                type="button"
                data-active={candidateMode === "all"}
                onClick={() => setCandidateMode("all")}
              >
                {t("settings.editors.candidates.all")}
              </button>
            </div>
          </div>
          <Input
            allowClear
            value={candidateQuery}
            placeholder={t("settings.editors.candidates.search")}
            onChange={(event) => setCandidateQuery(event.target.value)}
          />
          {candidateDiscovery.warnings.length > 0 && (
            <div className="editor-candidates-warning">
              {candidateDiscovery.warnings.join(" · ")}
            </div>
          )}
          <div className="editor-candidates-list">
            {visibleCandidates.length === 0 && (
              <div className="editor-settings-empty">
                {t("settings.editors.candidates.empty")}
              </div>
            )}
            {visibleCandidates.map((candidate: EditorCandidate) => (
              <article className="editor-candidate-item" key={candidate.id}>
                <span className="editor-candidate-icon" aria-hidden="true">
                  APP
                </span>
                <div className="editor-settings-copy">
                  <div className="editor-settings-name-row">
                    <strong>{candidate.name}</strong>
                    {candidate.recommended && (
                      <span className="editor-candidate-recommended">
                        {t("settings.editors.candidates.suggested")}
                      </span>
                    )}
                    <span className="editor-settings-source">
                      {t(
                        `settings.editors.candidates.source.${candidate.source}`,
                      )}
                    </span>
                  </div>
                  <span className="editor-settings-path" title={candidate.path}>
                    {candidate.path}
                  </span>
                </div>
                <Button
                  icon={<ImportOutlined />}
                  disabled={candidate.added}
                  loading={candidateImporting === candidate.id}
                  onClick={() =>
                    setImportDraft({ id: candidate.id, name: candidate.name })
                  }
                >
                  {candidate.added
                    ? t("settings.editors.candidates.added")
                    : t("settings.editors.candidates.import")}
                </Button>
              </article>
            ))}
          </div>
          {filteredCandidates.length > visibleCandidates.length && (
            <p className="editor-candidates-limit">
              {t("settings.editors.candidates.renderLimit", {
                shown: visibleCandidates.length,
                total: filteredCandidates.length,
              })}
            </p>
          )}
        </div>
      )}

      <Modal
        open={draft !== null}
        title={
          draft?.id ? t("settings.editors.editTitle") : t("settings.editors.addTitle")
        }
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
        confirmLoading={saving}
        maskClosable={!saving}
        keyboard={!saving}
        onOk={() => void handleSave()}
        onCancel={() => !saving && setDraft(null)}
      >
        {draft && (
          <div className="editor-settings-form">
            <label>
              <span>{t("settings.editors.name")}</span>
              <Input
                value={draft.name}
                maxLength={200}
                onChange={(event) => setDraft({ ...draft, name: event.target.value })}
              />
            </label>
            <label>
              <span>{t("settings.editors.program")}</span>
              <div className="editor-settings-path-input">
                <Input value={draft.path} readOnly />
                <Button
                  icon={<FolderOpenOutlined />}
                  onClick={() => void handleChangeProgram()}
                >
                  {t("settings.editors.choose")}
                </Button>
              </div>
            </label>
            {draft.canEditArgs && (
              <label>
                <span>{t("settings.editors.args")}</span>
                <Input.TextArea
                  value={draft.argsText}
                  autoSize={{ minRows: 3, maxRows: 7 }}
                  placeholder={t("settings.editors.argsPlaceholder")}
                  onChange={(event) =>
                    setDraft({ ...draft, argsText: event.target.value })
                  }
                />
                <small>{t("settings.editors.argsHint")}</small>
              </label>
            )}
          </div>
        )}
      </Modal>

      <Modal
        open={importDraft !== null}
        title={t("settings.editors.candidates.importTitle")}
        okText={t("settings.editors.candidates.import")}
        cancelText={t("common.cancel")}
        confirmLoading={candidateImporting !== null}
        onOk={() => void handleImportCandidate()}
        onCancel={() => !candidateImporting && setImportDraft(null)}
      >
        {importDraft && (
          <label className="editor-candidate-import-name">
            <span>{t("settings.editors.name")}</span>
            <Input
              value={importDraft.name}
              maxLength={200}
              onChange={(event) =>
                setImportDraft({ ...importDraft, name: event.target.value })
              }
            />
          </label>
        )}
      </Modal>
    </section>
  );
};

export default EditorSettings;
