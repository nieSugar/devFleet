import React, { useState, useEffect, useCallback, useRef } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import "./UpdateChecker.css";

type Status =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

interface UpdateInfo {
  version: string;
  body: string | null;
  date: string | null;
}

const UpdateChecker: React.FC = () => {
  const [status, setStatus] = useState<Status>("idle");
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState(0);
  const [showModal, setShowModal] = useState(false);
  const [error, setError] = useState("");
  const [currentVersion, setCurrentVersion] = useState("");
  const updateRef = useRef<Update | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      updateRef.current = null;
    };
  }, []);

  const checkForUpdate = useCallback(async (manual: boolean) => {
    if (manual) setStatus("checking");
    setError("");
    try {
      const result = await check();
      if (!mountedRef.current) return;
      if (result?.available) {
        updateRef.current = result;
        setInfo({
          version: result.version,
          body: result.body ?? null,
          date: result.date ?? null,
        });
        setStatus("available");
      } else {
        if (manual) setStatus("idle");
      }
    } catch (e) {
      if (!mountedRef.current) return;
      if (manual) {
        setError(String(e));
        setStatus("error");
      }
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    getVersion().then((v) => { if (!cancelled) setCurrentVersion(v); }).catch(() => {});
    const timer = setTimeout(() => checkForUpdate(false), 3000);
    return () => { cancelled = true; clearTimeout(timer); };
  }, [checkForUpdate]);

  const handleDownload = async () => {
    const update = updateRef.current;
    if (!update) return;
    setStatus("downloading");
    setProgress(0);
    try {
      let downloaded = 0;
      let total = 0;
      await update.downloadAndInstall((event) => {
        if (!mountedRef.current) return;
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (total > 0)
              setProgress(Math.round((downloaded / total) * 100));
            break;
          case "Finished":
            break;
        }
      });
      if (mountedRef.current) setStatus("ready");
    } catch (e) {
      if (!mountedRef.current) return;
      setError(String(e));
      setStatus("error");
    }
  };

  const handleButtonClick = () => {
    if (status !== "available" && status !== "downloading" && status !== "ready")
      checkForUpdate(true);
    setShowModal(true);
  };

  const closeModal = () => {
    if (status !== "downloading") setShowModal(false);
  };

  return (
    <>
      <button
        className={`tb-btn update-btn${status === "available" ? " has-update" : ""}${status === "checking" ? " is-checking" : ""}`}
        onClick={handleButtonClick}
        title={
          status === "available"
            ? `新版本 v${info?.version}`
            : status === "checking"
              ? "正在检查..."
              : "检查更新"
        }
      >
        <RefreshIcon />
        {status === "available" && <span className="update-dot" />}
      </button>

      {showModal && (
        <div className="update-overlay" onClick={closeModal}>
          <div className="update-modal" onClick={(e) => e.stopPropagation()}>
            <div className="update-modal-header">
              <h3>软件更新</h3>
              <button
                className="update-close-btn"
                onClick={closeModal}
                disabled={status === "downloading"}
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 12 12"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                >
                  <path d="M2 2l8 8M10 2l-8 8" />
                </svg>
              </button>
            </div>

            <div className="update-modal-body">
              {status === "checking" && (
                <div className="update-status">
                  <div className="update-spinner" />
                  <p>正在检查更新...</p>
                </div>
              )}

              {status === "idle" && (
                <div className="update-status">
                  <div className="update-ok-icon">&#10003;</div>
                  <p>当前已是最新版本</p>
                  <span className="update-version-tag">
                    v{currentVersion}
                  </span>
                </div>
              )}

              {status === "available" && info && (
                <div className="update-info">
                  <div className="update-versions">
                    <span className="update-ver-old">v{currentVersion}</span>
                    <svg
                      width="16"
                      height="16"
                      viewBox="0 0 16 16"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      <path d="M3 8h10M9 4l4 4-4 4" />
                    </svg>
                    <span className="update-ver-new">v{info.version}</span>
                  </div>
                  {info.body && (
                    <div className="update-notes">
                      <p className="update-notes-label">更新说明</p>
                      <div className="update-notes-content">{info.body}</div>
                    </div>
                  )}
                </div>
              )}

              {status === "downloading" && (
                <div className="update-dl">
                  <p>正在下载更新 v{info?.version}</p>
                  <div className="update-bar">
                    <div
                      className="update-bar-fill"
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                  <span className="update-bar-text">{progress}%</span>
                </div>
              )}

              {status === "ready" && (
                <div className="update-status">
                  <div className="update-ok-icon">&#10003;</div>
                  <p>更新已下载完成</p>
                  <span className="update-hint">重启应用以完成安装</span>
                </div>
              )}

              {status === "error" && (
                <div className="update-status update-err">
                  <div className="update-err-icon">!</div>
                  <p>检查更新失败</p>
                  <span className="update-err-msg">{error}</span>
                </div>
              )}
            </div>

            <div className="update-modal-footer">
              {status === "available" && (
                <button
                  className="update-action-btn primary"
                  onClick={handleDownload}
                >
                  下载并安装
                </button>
              )}
              {status === "ready" && (
                <button
                  className="update-action-btn primary"
                  onClick={() => relaunch()}
                >
                  立即重启
                </button>
              )}
              {status === "error" && (
                <button
                  className="update-action-btn"
                  onClick={() => checkForUpdate(true)}
                >
                  重试
                </button>
              )}
              {status === "idle" && (
                <button
                  className="update-action-btn"
                  onClick={() => checkForUpdate(true)}
                >
                  检查更新
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
};

const RefreshIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M1.5 8a6.5 6.5 0 0111.48-4.17" />
    <path d="M14.5 2v4h-4" />
    <path d="M14.5 8a6.5 6.5 0 01-11.48 4.17" />
    <path d="M1.5 14v-4h4" />
  </svg>
);

export default UpdateChecker;
