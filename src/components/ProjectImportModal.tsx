import React, { useCallback, useEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, List, Modal, Spin, Tag, Typography } from "antd";
import { FolderOpenOutlined } from "@ant-design/icons";
import { useTranslation } from "react-i18next";
import { tauriAPI } from "../lib/tauri";
import type { ProjectScanCandidate, ProjectScanResult } from "../types/project";
import "./ProjectImportModal.css";

interface ProjectImportModalProps {
  open: boolean;
  onClose: () => void;
  onImported: () => void;
}

type ImportState = "success" | "error";
type ImportOutcome = { state: ImportState; message: string };

const ProjectImportModal: React.FC<ProjectImportModalProps> = ({ open, onClose, onImported }) => {
  const { t } = useTranslation();
  const [rootPath, setRootPath] = useState("");
  const [result, setResult] = useState<ProjectScanResult | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [outcomes, setOutcomes] = useState<Record<string, ImportOutcome>>({});
  const [scanning, setScanning] = useState(false);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState("");
  const [scanCancelled, setScanCancelled] = useState(false);
  const generation = useRef(0);

  const reset = useCallback(() => {
    setRootPath("");
    setResult(null);
    setSelected(new Set());
    setOutcomes({});
    setError("");
    setScanCancelled(false);
  }, []);

  useEffect(() => {
    if (!open) {
      generation.current += 1;
      if (scanning) void tauriAPI.cancelProjectScan();
      reset();
    }
  }, [open, reset, scanning]);

  const chooseFolder = useCallback(async () => {
    try {
      const path = await tauriAPI.selectFolder();
      if (!path) return;
      setRootPath(path);
      setResult(null);
      setSelected(new Set());
      setOutcomes({});
      setError("");
      setScanCancelled(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : t("project.import.chooseFolderFailed"));
    }
  }, [t]);

  const scan = useCallback(async () => {
    if (!rootPath || scanning || importing) return;
    const requestId = ++generation.current;
    setScanning(true);
    setResult(null);
    setSelected(new Set());
    setOutcomes({});
    setError("");
    setScanCancelled(false);
    try {
      const response = await tauriAPI.scanProjectCandidates(rootPath);
      if (requestId !== generation.current) return;
      if (response.success && response.data) setResult(response.data);
      else setError(response.error || t("project.import.scanFailed"));
    } catch (cause) {
      if (requestId === generation.current)
        setError(cause instanceof Error ? cause.message : t("project.import.scanFailed"));
    } finally {
      if (requestId === generation.current) setScanning(false);
    }
  }, [importing, rootPath, scanning, t]);

  const cancelScan = useCallback(() => {
    generation.current += 1;
    setScanning(false);
    setScanCancelled(true);
    void tauriAPI.cancelProjectScan();
  }, []);

  const close = useCallback(() => {
    if (importing) return;
    if (scanning) cancelScan();
    onClose();
  }, [cancelScan, importing, onClose, scanning]);

  const toggle = useCallback((candidate: ProjectScanCandidate, checked: boolean) => {
    setSelected((previous) => {
      const next = new Set(previous);
      if (checked) next.add(candidate.path);
      else next.delete(candidate.path);
      return next;
    });
  }, []);

  const importSelected = useCallback(async () => {
    if (!result || importing || selected.size === 0) return;
    setImporting(true);
    setError("");
    let successCount = 0;
    const nextOutcomes: Record<string, ImportOutcome> = {};
    for (const candidate of result.candidates) {
      if (!selected.has(candidate.path) || candidate.added) continue;
      try {
        const response = await tauriAPI.addProjectToConfig(candidate.path);
        if (response.success) {
          successCount += 1;
          nextOutcomes[candidate.path] = { state: "success", message: t("project.import.itemSuccess") };
          setSelected((previous) => {
            const next = new Set(previous);
            next.delete(candidate.path);
            return next;
          });
          setResult((previous) => previous
            ? { ...previous, candidates: previous.candidates.map((item) => item.path === candidate.path ? { ...item, added: true } : item) }
            : previous);
        } else {
          nextOutcomes[candidate.path] = { state: "error", message: response.error || t("project.import.itemFailed") };
        }
      } catch (cause) {
        nextOutcomes[candidate.path] = {
          state: "error",
          message: cause instanceof Error ? cause.message : t("project.import.itemFailed"),
        };
      }
      setOutcomes({ ...nextOutcomes });
    }
    setImporting(false);
    if (successCount > 0) onImported();
  }, [importing, onImported, result, selected, t]);

  const candidates = result?.candidates ?? [];
  const selectedCount = selected.size;

  return (
    <Modal
      open={open}
      title={t("project.import.title")}
      onCancel={close}
      closable={!importing}
      maskClosable={!importing}
      keyboard={!importing}
      destroyOnHidden
      width={720}
      footer={[
        <Button key="cancel" onClick={close} disabled={importing}>{t("common.cancel")}</Button>,
        <Button
          key="import"
          type="primary"
          loading={importing}
          disabled={!result || selectedCount === 0 || scanning}
          onClick={importSelected}
        >
          {t("project.import.confirm", { count: selectedCount })}
        </Button>,
      ]}
    >
      <div className="project-import-modal">
        <div className="project-import-picker">
          <Typography.Text ellipsis={{ tooltip: rootPath }} className="project-import-path">
            {rootPath || t("project.import.noFolder")}
          </Typography.Text>
          <Button icon={<FolderOpenOutlined />} onClick={chooseFolder} disabled={scanning || importing}>
            {t("project.import.chooseFolder")}
          </Button>
          <Button type="primary" onClick={scan} disabled={!rootPath || scanning || importing}>
            {scanning ? t("project.import.scanning") : t("project.import.scan")}
          </Button>
          {scanning && <Button onClick={cancelScan}>{t("project.import.cancelScan")}</Button>}
        </div>

        {error && <Alert type="error" showIcon message={error} className="project-import-alert" />}
        {scanning && <div className="project-import-loading"><Spin /> {t("project.import.scanningHint")}</div>}
        {(scanCancelled || result?.cancelled) && <Alert type="warning" showIcon message={t("project.import.cancelled")} className="project-import-alert" />}
        {result && result.warnings.slice(0, 5).map((warning, index) => (
          <Alert
            key={`${warning.code}-${warning.path ?? ""}-${index}`}
            type="warning"
            showIcon
            message={t(`project.import.warnings.${warning.code}`, { path: warning.path ?? "", detail: warning.detail ?? "" })}
            className="project-import-alert"
          />
        ))}
        {result?.truncated && !result.warnings.some(({ code }) => code === "DIRECTORY_LIMIT" || code === "CANDIDATE_LIMIT") && (
          <Alert type="warning" showIcon message={t("project.import.truncated")} className="project-import-alert" />
        )}

        {result && (
          <>
            <div className="project-import-summary">
              {t("project.import.summary", { count: candidates.length, visited: result.visitedDirectories })}
            </div>
            <List
              className="project-import-list"
              bordered
              rowKey="path"
              locale={{ emptyText: t("project.import.empty") }}
              dataSource={candidates}
              renderItem={(candidate) => {
                const outcome = outcomes[candidate.path];
                return (
                  <List.Item>
                    <Checkbox
                      checked={selected.has(candidate.path)}
                      disabled={candidate.added || importing}
                      onChange={(event) => toggle(candidate, event.target.checked)}
                    >
                      <div className="project-import-candidate">
                        <Typography.Text strong>{candidate.name}</Typography.Text>
                        <Typography.Text type="secondary" ellipsis={{ tooltip: candidate.path }}>{candidate.path}</Typography.Text>
                      </div>
                    </Checkbox>
                    {candidate.added && <Tag>{t("project.import.alreadyAdded")}</Tag>}
                    {outcome && <Tag color={outcome.state === "success" ? "success" : "error"}>{outcome.message}</Tag>}
                  </List.Item>
                );
              }}
            />
          </>
        )}
      </div>
    </Modal>
  );
};

export default ProjectImportModal;
