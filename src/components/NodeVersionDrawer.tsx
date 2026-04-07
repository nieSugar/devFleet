import React, { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Drawer, Input, Spin, Tooltip, message } from "antd";
import {
  SearchOutlined,
  DownOutlined,
  CheckCircleFilled,
  LoadingOutlined,
  ReloadOutlined,
  CloudDownloadOutlined,
  SafetyCertificateFilled,
  SwapOutlined,
  DeleteOutlined,
  GlobalOutlined,
  CheckOutlined,
  EditOutlined,
  FolderOutlined,
  FolderOpenOutlined,
  ApiOutlined,
  CheckCircleOutlined,
} from "@ant-design/icons";
import { tauriAPI } from "../lib/tauri";
import type {
  NvmInfo,
  RemoteNodeVersion,
  NodeVersionManager,
} from "../types/project";
import "./NodeVersionDrawer.css";

interface NodeVersionDrawerProps {
  open: boolean;
  onClose: () => void;
  onVersionChange?: () => void;
}

type FilterMode = "all" | "lts" | "installed";

interface VersionGroup {
  major: number;
  ltsName?: string;
  hasCurrent: boolean;
  count: number;
  versions: EnrichedVersion[];
}

interface EnrichedVersion extends RemoteNodeVersion {
  isInstalled: boolean;
  isCurrent: boolean;
}

const MANAGER_KEYS: Record<NodeVersionManager, string> = {
  builtin: "nodeDrawer.manager.builtin",
  nvmd: "nodeDrawer.manager.nvmd",
  nvs: "nodeDrawer.manager.nvs",
  nvm: "nodeDrawer.manager.nvm",
  "nvm-windows": "nodeDrawer.manager.nvm-windows",
  none: "nodeDrawer.manager.none",
};

const MIRROR_PRESETS: { labelKey: string; url: string }[] = [
  { labelKey: "nodeDrawer.mirror.official", url: "" },
  { labelKey: "npmmirror", url: "https://npmmirror.com/mirrors/node" },
];

interface MirrorSelectorProps {
  mirror: string;
  onMirrorChange: (url: string) => void;
}

const MirrorSelector: React.FC<MirrorSelectorProps> = ({
  mirror,
  onMirrorChange,
}) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [customMode, setCustomMode] = useState(false);
  const [customUrl, setCustomUrl] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  const isPreset = MIRROR_PRESETS.some((p) => p.url === mirror);
  const matchedPreset = MIRROR_PRESETS.find((p) => p.url === mirror);
  const activeLabel = matchedPreset
    ? (matchedPreset.labelKey === "npmmirror" ? "npmmirror" : t(matchedPreset.labelKey))
    : t("nodeDrawer.mirror.custom");

  const toggleOpen = useCallback(() => {
    setOpen((prev) => {
      if (prev) {
        setCustomMode(false);
        setCustomUrl(mirror);
      }
      return !prev;
    });
  }, [mirror]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    if (open) document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const selectPreset = (url: string) => {
    onMirrorChange(url);
    setOpen(false);
  };

  const submitCustom = () => {
    const trimmed = customUrl.trim().replace(/\/+$/, "");
    onMirrorChange(trimmed);
    setOpen(false);
  };

  return (
    <div className="nd-mirror" ref={ref}>
      <button
        className="nd-mirror-trigger"
        onClick={toggleOpen}
        title={t("nodeDrawer.mirror.title")}
      >
        <GlobalOutlined />
        <span>{activeLabel}</span>
        <DownOutlined className={`nd-mirror-arrow ${open ? "open" : ""}`} />
      </button>

      {open && (
        <div className="nd-mirror-dropdown">
          {MIRROR_PRESETS.map((p) => (
            <button
              key={p.url}
              className={`nd-mirror-option ${mirror === p.url ? "active" : ""}`}
              onClick={() => selectPreset(p.url)}
            >
              <span className="nd-mirror-option-label">
                {p.labelKey === "npmmirror" ? "npmmirror" : t(p.labelKey)}
              </span>
              {p.url && (
                <span className="nd-mirror-option-url">{p.url}</span>
              )}
              {mirror === p.url && <CheckOutlined className="nd-mirror-check" />}
            </button>
          ))}

          <div className="nd-mirror-divider" />

          {customMode ? (
            <div className="nd-mirror-custom">
              <input
                className="nd-mirror-input"
                placeholder="https://example.com/mirrors/node"
                value={customUrl}
                onChange={(e) => setCustomUrl(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && submitCustom()}
                autoFocus
              />
              <button className="nd-mirror-save" onClick={submitCustom}>
                {t("common.confirm")}
              </button>
            </div>
          ) : (
            <button
              className={`nd-mirror-option ${!isPreset && mirror ? "active" : ""}`}
              onClick={() => {
                setCustomMode(true);
                setCustomUrl(mirror);
              }}
            >
              <EditOutlined />
              <span className="nd-mirror-option-label">{t("nodeDrawer.mirror.customMirror")}</span>
              {!isPreset && mirror && (
                <CheckOutlined className="nd-mirror-check" />
              )}
            </button>
          )}
        </div>
      )}
    </div>
  );
};

interface InstallDirSelectorProps {
  dir: string;
  resolvedDir: string;
  onDirChange: (dir: string) => void;
}

const InstallDirSelector: React.FC<InstallDirSelectorProps> = ({
  dir,
  resolvedDir,
  onDirChange,
}) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [inputVal, setInputVal] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  const displayDir = resolvedDir || dir || t("nodeDrawer.installDir.default");
  const truncated =
    displayDir.length > 35
      ? "..." + displayDir.slice(-32)
      : displayDir;

  const toggleOpen = useCallback(() => {
    setOpen((prev) => {
      if (!prev) setInputVal(dir);
      return !prev;
    });
  }, [dir]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    if (open) document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const submit = () => {
    onDirChange(inputVal.trim());
    setOpen(false);
  };

  const resetDefault = () => {
    onDirChange("");
    setOpen(false);
  };

  return (
    <div className="nd-mirror" ref={ref}>
      <button
        className="nd-mirror-trigger"
        onClick={toggleOpen}
        title={t("nodeDrawer.installDir.tooltip", { dir: dir || t("nodeDrawer.installDir.default") })}
      >
        <FolderOutlined />
        <span>{truncated}</span>
        <DownOutlined className={`nd-mirror-arrow ${open ? "open" : ""}`} />
      </button>

      {open && (
        <div className="nd-mirror-dropdown">
          <div className="nd-mirror-custom nd-install-dir-panel">
            <div className="nd-install-dir-label">
              <FolderOpenOutlined />
              <span>{t("nodeDrawer.installDir.title")}</span>
            </div>
            <input
              className="nd-mirror-input"
              placeholder={t("nodeDrawer.installDir.placeholder")}
              value={inputVal}
              onChange={(e) => setInputVal(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submit()}
              autoFocus
            />
            <div className="nd-install-dir-actions">
              <button className="nd-mirror-save" onClick={submit}>
                {t("common.confirm")}
              </button>
              {dir && (
                <button
                  className="nd-mirror-save"
                  onClick={resetDefault}
                  style={{ opacity: 0.7 }}
                >
                  {t("nodeDrawer.installDir.resetDefault")}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

type BusyAction = { version: string; type: "install" | "switch" | "uninstall" };

const NodeVersionDrawer: React.FC<NodeVersionDrawerProps> = ({
  open,
  onClose,
  onVersionChange,
}) => {
  const { t } = useTranslation();
  const [remoteVersions, setRemoteVersions] = useState<RemoteNodeVersion[]>(
    [],
  );
  const [nvmInfo, setNvmInfo] = useState<NvmInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<BusyAction | null>(null);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<FilterMode>("all");
  const [expandedGroups, setExpandedGroups] = useState<Set<number>>(new Set());
  const [messageApi, contextHolder] = message.useMessage();
  const [hasFetched, setHasFetched] = useState(false);
  const [mirror, setMirror] = useState("");
  const [installDir, setInstallDir] = useState("");
  const [installDirDisplay, setInstallDirDisplay] = useState("");
  const [nodeInPath, setNodeInPath] = useState(true);
  const [nodeAvailable, setNodeAvailable] = useState(true);
  const [pathSetupBusy, setPathSetupBusy] = useState(false);

  const checkPathStatus = useCallback(() => {
    tauriAPI.checkNodeInPath().then((res) => {
      if (res.success && res.data) {
        setNodeInPath(res.data.inPath);
        setNodeAvailable(res.data.nodeAvailable);
      }
    });
  }, []);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    tauriAPI.getNodeMirror().then((res) => {
      if (!cancelled && res.success && res.data) setMirror(res.data.mirror);
    });
    tauriAPI.getNodeInstallDir().then((res) => {
      if (!cancelled && res.success && res.data) {
        setInstallDir(res.data.custom);
        setInstallDirDisplay(res.data.dir);
      }
    });
    checkPathStatus();
    return () => { cancelled = true; };
  }, [open, checkPathStatus]);

  const handleMirrorChange = useCallback(
    async (url: string) => {
      const prev = mirror;
      setMirror(url);
      const res = await tauriAPI.setNodeMirror(url);
      if (res.success) {
        messageApi.success(t("nodeDrawer.mirrorSwitched"));
        setHasFetched(false);
      } else {
        setMirror(prev);
        messageApi.error(res.error || t("nodeDrawer.mirrorFailed"));
      }
    },
    [mirror, messageApi, t],
  );

  const handleSetupPath = useCallback(async () => {
    setPathSetupBusy(true);
    try {
      const res = await tauriAPI.setupNodeGlobalPath();
      if (res.success && res.data) {
        messageApi.success(res.data.message);
        setNodeInPath(true);
      } else {
        messageApi.error(res.error || "设置失败");
      }
    } catch {
      messageApi.error(t("nodeDrawer.pathSetupFailed"));
    } finally {
      setPathSetupBusy(false);
    }
  }, [messageApi, t]);

  const handleInstallDirChange = useCallback(
    async (dir: string) => {
      const prev = installDir;
      setInstallDir(dir);
      const res = await tauriAPI.setNodeInstallDir(dir);
      if (res.success) {
        messageApi.success(t("nodeDrawer.installDirUpdated"));
        const dirRes = await tauriAPI.getNodeInstallDir();
        if (dirRes.success && dirRes.data) {
          setInstallDirDisplay(dirRes.data.dir);
        }
        setHasFetched(false);
      } else {
        setInstallDir(prev);
        messageApi.error(res.error || t("nodeDrawer.installDirFailed"));
      }
    },
    [installDir, messageApi, t],
  );

  const cancelledRef = useRef(false);

  const fetchData = useCallback(async () => {
    setLoading(true);
    cancelledRef.current = false;
    try {
      const [remoteResult, nvmResult] = await Promise.all([
        tauriAPI.fetchRemoteNodeVersions(),
        tauriAPI.getNvmInfo(),
      ]);
      if (cancelledRef.current) return;
      if (remoteResult.success && remoteResult.data) {
        setRemoteVersions(remoteResult.data);
      } else {
        messageApi.error(remoteResult.error || t("nodeDrawer.fetchFailed"));
      }
      if (nvmResult.success && nvmResult.data) {
        setNvmInfo(nvmResult.data);
      }
      setHasFetched(true);
    } catch {
      if (!cancelledRef.current)
        messageApi.error(t("nodeDrawer.networkError"));
    } finally {
      if (!cancelledRef.current) setLoading(false);
    }
  }, [messageApi, t]);

  useEffect(() => {
    if (open && !hasFetched) {
      fetchData();
    }
    return () => { cancelledRef.current = true; };
  }, [open, hasFetched, fetchData]);

  const refreshNvmInfo = useCallback(async () => {
    const nvmResult = await tauriAPI.getNvmInfo();
    if (nvmResult.success && nvmResult.data) setNvmInfo(nvmResult.data);
    checkPathStatus();
  }, [checkPathStatus]);

  const installedSet = useMemo(() => {
    const set = new Set<string>();
    nvmInfo?.availableVersions?.forEach((v) => set.add(v.version));
    return set;
  }, [nvmInfo]);

  const groups = useMemo(() => {
    const groupMap = new Map<number, VersionGroup>();

    for (const rv of remoteVersions) {
      const ver = rv.version.replace(/^v/, "");
      const major = parseInt(ver.split(".")[0], 10);
      if (isNaN(major)) continue;

      const isInstalled = installedSet.has(ver);
      const isCurrent = nvmInfo?.currentVersion === ver;

      if (filter === "lts" && rv.lts === false) continue;
      if (filter === "installed" && !isInstalled) continue;
      if (search) {
        const q = search.toLowerCase();
        const matchVer = ver.toLowerCase().includes(q);
        const matchLts =
          typeof rv.lts === "string" && rv.lts.toLowerCase().includes(q);
        if (!matchVer && !matchLts) continue;
      }

      if (!groupMap.has(major)) {
        groupMap.set(major, {
          major,
          ltsName: typeof rv.lts === "string" ? rv.lts : undefined,
          hasCurrent: false,
          count: 0,
          versions: [],
        });
      }

      const group = groupMap.get(major)!;
      group.count++;
      if (isCurrent) group.hasCurrent = true;
      if (!group.ltsName && typeof rv.lts === "string") {
        group.ltsName = rv.lts;
      }
      group.versions.push({ ...rv, isInstalled, isCurrent });
    }

    return Array.from(groupMap.values()).sort((a, b) => b.major - a.major);
  }, [remoteVersions, installedSet, nvmInfo, filter, search]);

  const toggleGroup = (major: number) => {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(major)) next.delete(major);
      else next.add(major);
      return next;
    });
  };

  const handleVersionAction = useCallback(
    async (
      version: string,
      type: BusyAction["type"],
      apiFn: (params: {
        version: string;
        manager?: string;
      }) => Promise<
        import("../types/project").IpcResponse<{ message: string; output: string }>
      >,
      errorMsg: string,
    ) => {
      const mgr = nvmInfo?.isInstalled ? nvmInfo.manager : "builtin";
      setBusy({ version, type });
      try {
        const result = await apiFn({ version, manager: mgr });
        if (result.success && result.data) {
          messageApi.success(result.data.message);
          await refreshNvmInfo();
          onVersionChange?.();
        } else {
          messageApi.error(result.error || errorMsg);
        }
      } catch {
        messageApi.error(errorMsg);
      } finally {
        setBusy(null);
      }
    },
    [nvmInfo, messageApi, refreshNvmInfo, onVersionChange],
  );

  const handleInstall = useCallback(
    (version: string) =>
      handleVersionAction(version, "install", tauriAPI.installNodeVersion, t("nodeDrawer.installFailed")),
    [handleVersionAction, t],
  );

  const handleSwitch = useCallback(
    (version: string) =>
      handleVersionAction(version, "switch", tauriAPI.switchNodeVersion, t("nodeDrawer.switchFailed")),
    [handleVersionAction, t],
  );

  const handleUninstall = useCallback(
    (version: string) =>
      handleVersionAction(version, "uninstall", tauriAPI.uninstallNodeVersion, t("nodeDrawer.uninstallFailed")),
    [handleVersionAction, t],
  );

  const handleRefresh = () => {
    setHasFetched(false);
    setExpandedGroups(new Set());
    fetchData();
  };

  const filterButtons: { key: FilterMode; label: string }[] = [
    { key: "all", label: t("nodeDrawer.filterAll") },
    { key: "lts", label: t("nodeDrawer.filterLts") },
    { key: "installed", label: t("nodeDrawer.filterInstalled") },
  ];

  const isBusy = busy !== null;

  return (
    <Drawer
      open={open}
      onClose={onClose}
      width={520}
      closable={false}
      className="node-drawer"
      styles={{ body: { padding: 0 }, header: { display: "none" } }}
    >
      {contextHolder}

      {/* ── Header ── */}
      <div className="nd-header">
        <div className="nd-header-top">
          <div className="nd-title">
            <div className="nd-title-icon">
              <NodeIcon />
            </div>
            <span className="nd-title-text">{t("nodeDrawer.title")}</span>
          </div>
          <div className="nd-header-actions">
            <Tooltip title={t("common.refresh")}>
              <button
                className="nd-icon-btn"
                onClick={handleRefresh}
                disabled={loading}
              >
                <ReloadOutlined spin={loading} />
              </button>
            </Tooltip>
            <button className="nd-icon-btn nd-close-btn" onClick={onClose}>
              <svg
                width="10"
                height="10"
                viewBox="0 0 10 10"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              >
                <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" />
              </svg>
            </button>
          </div>
        </div>

        {nvmInfo && (
          <div className="nd-meta">
            <div className="nd-manager-badge">
              <span
                className={`nd-manager-dot ${nvmInfo.isInstalled ? "active" : ""}`}
              />
              <span>{t(MANAGER_KEYS[nvmInfo.manager])}</span>
            </div>
            {nvmInfo.currentVersion && (
              <div className="nd-current-ver">
                <span className="nd-current-label">{t("common.current")}</span>
                <span className="nd-current-num">
                  v{nvmInfo.currentVersion}
                </span>
              </div>
            )}
            {remoteVersions.length > 0 && (
              <span className="nd-total-count">
                {t("common.versions", { count: remoteVersions.length })}
              </span>
            )}
          </div>
        )}

        <div className="nd-meta" style={{ marginTop: nvmInfo ? 8 : 14, gap: 8, flexWrap: "wrap" }}>
          <MirrorSelector mirror={mirror} onMirrorChange={handleMirrorChange} />
          {nvmInfo?.manager === "builtin" && (
            <InstallDirSelector
              dir={installDir}
              resolvedDir={installDirDisplay}
              onDirChange={handleInstallDirChange}
            />
          )}
        </div>

        {nvmInfo?.manager === "builtin" && nvmInfo.currentVersion && !nodeInPath && !nodeAvailable && (
          <div className="nd-path-banner">
            <div className="nd-path-banner-text">
              <ApiOutlined />
              <span>{t("nodeDrawer.pathBanner")}</span>
            </div>
            <button
              className="nd-path-btn"
              onClick={handleSetupPath}
              disabled={pathSetupBusy}
            >
              {pathSetupBusy ? <LoadingOutlined /> : <CheckCircleOutlined />}
              <span>{t("nodeDrawer.addToPath")}</span>
            </button>
          </div>
        )}
      </div>

      {/* ── Toolbar ── */}
      <div className="nd-toolbar">
        <Input
          placeholder={t("nodeDrawer.searchPlaceholder")}
          prefix={<SearchOutlined style={{ color: "var(--text-muted)" }} />}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          allowClear
          className="nd-search"
        />
        <div className="nd-filters">
          {filterButtons.map((f) => (
            <button
              key={f.key}
              className={`nd-filter-btn ${filter === f.key ? "active" : ""}`}
              onClick={() => setFilter(f.key)}
            >
              {f.label}
            </button>
          ))}
        </div>
      </div>

      {/* ── Content ── */}
      <div className="nd-content">
        {loading && !hasFetched ? (
          <div className="nd-loading">
            <Spin indicator={<LoadingOutlined style={{ fontSize: 28 }} />} />
            <span className="nd-loading-text">{t("nodeDrawer.loading")}</span>
          </div>
        ) : groups.length === 0 ? (
          <div className="nd-empty">
            <CloudDownloadOutlined className="nd-empty-icon" />
            <span className="nd-empty-text">
              {search || filter !== "all"
                ? t("nodeDrawer.noMatch")
                : t("nodeDrawer.noData")}
            </span>
            {!search && filter === "all" && (
              <button className="nd-retry-btn" onClick={handleRefresh}>
                {t("nodeDrawer.reload")}
              </button>
            )}
          </div>
        ) : (
          groups.map((group, gi) => {
            const isExpanded = expandedGroups.has(group.major);

            return (
              <div
                key={group.major}
                className={`nd-group ${isExpanded ? "expanded" : ""} ${group.hasCurrent ? "has-current" : ""}`}
                style={{ animationDelay: `${gi * 40}ms` }}
              >
                <div
                  className="nd-group-header"
                  onClick={() => toggleGroup(group.major)}
                >
                  <div className="nd-group-left">
                    <DownOutlined className="nd-group-arrow" />
                    <span className="nd-group-major">
                      Node.js {group.major}
                    </span>
                    {group.ltsName && (
                      <span className="nd-lts-badge">{group.ltsName}</span>
                    )}
                    {group.hasCurrent && (
                      <span className="nd-current-tag">{t("common.current")}</span>
                    )}
                  </div>
                  <div className="nd-group-right">
                    <span className="nd-group-latest">
                      {group.versions[0]?.version}
                    </span>
                    <span className="nd-group-count">
                      {t("common.versions", { count: group.count })}
                    </span>
                  </div>
                </div>

                {isExpanded && (
                  <div className="nd-version-list">
                    {group.versions.map((v, vi) => (
                      <VersionRow
                        key={v.version}
                        v={v}
                        vi={vi}
                        busy={busy}
                        isBusy={isBusy}
                        onInstall={handleInstall}
                        onSwitch={handleSwitch}
                        onUninstall={handleUninstall}
                      />
                    ))}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </Drawer>
  );
};

interface VersionRowProps {
  v: EnrichedVersion;
  vi: number;
  busy: BusyAction | null;
  isBusy: boolean;
  onInstall: (version: string) => void;
  onSwitch: (version: string) => void;
  onUninstall: (version: string) => void;
}

const VersionRow: React.FC<VersionRowProps> = ({
  v,
  vi,
  busy,
  isBusy,
  onInstall,
  onSwitch,
  onUninstall,
}) => {
  const { t } = useTranslation();
  const ver = v.version.replace(/^v/, "");
  const isThisBusy = busy?.version === v.version;

  return (
    <div
      className={`nd-version-row ${v.isCurrent ? "current" : ""} ${v.isInstalled ? "installed" : ""}`}
      style={{ animationDelay: `${vi * 25}ms` }}
    >
      <div className="nd-version-info">
        <span className="nd-version-num">{v.version}</span>
        <span className="nd-version-date">{v.date}</span>
        {typeof v.lts === "string" && (
          <span className="nd-version-lts">{v.lts}</span>
        )}
        {v.security && (
          <Tooltip title={t("nodeDrawer.securityUpdate")}>
            <SafetyCertificateFilled className="nd-security-icon" />
          </Tooltip>
        )}
        {v.npm && <span className="nd-version-npm">npm {v.npm}</span>}
      </div>
      <div className="nd-version-actions">
        {v.isCurrent ? (
          <span className="nd-current-badge">
            <CheckCircleFilled />
            {t("nodeDrawer.currentUse")}
          </span>
        ) : v.isInstalled ? (
          <>
            <button
              className={`nd-switch-btn ${isThisBusy && busy?.type === "switch" ? "switching" : ""}`}
              onClick={() => onSwitch(ver)}
              disabled={isBusy}
            >
              {isThisBusy && busy?.type === "switch" ? (
                <>
                  <LoadingOutlined />
                  <span>{t("common.switching")}</span>
                </>
              ) : (
                <>
                  <SwapOutlined />
                  <span>{t("common.switch")}</span>
                </>
              )}
            </button>
            <Tooltip title={t("nodeDrawer.uninstallVersion")}>
              <button
                className={`nd-uninstall-btn ${isThisBusy && busy?.type === "uninstall" ? "uninstalling" : ""}`}
                onClick={() => onUninstall(ver)}
                disabled={isBusy}
              >
                {isThisBusy && busy?.type === "uninstall" ? (
                  <LoadingOutlined />
                ) : (
                  <DeleteOutlined />
                )}
              </button>
            </Tooltip>
          </>
        ) : (
          <button
            className={`nd-install-btn ${isThisBusy && busy?.type === "install" ? "installing" : ""}`}
            onClick={() => onInstall(v.version)}
            disabled={isBusy}
          >
            {isThisBusy && busy?.type === "install" ? (
              <>
                <LoadingOutlined />
                <span>{t("common.installing")}</span>
              </>
            ) : (
              <span>{t("common.install")}</span>
            )}
          </button>
        )}
      </div>
    </div>
  );
};

const NodeIcon: React.FC = () => (
  <svg width="16" height="16" viewBox="0 0 256 289" fill="currentColor">
    <path d="M128 288.464c-3.975 0-7.685-1.06-11.13-2.915l-35.247-20.936c-5.3-2.915-2.65-3.975-1.06-4.505 7.155-2.385 8.48-2.915 15.9-7.155.796-.53 1.856-.265 2.65.265l27.032 16.166c1.06.53 2.385.53 3.18 0l105.74-61.217c1.06-.53 1.59-1.59 1.59-2.915V83.08c0-1.325-.53-2.385-1.59-2.915L129.06 19.213c-1.06-.53-2.385-.53-3.18 0L20.14 80.43c-1.06.53-1.59 1.855-1.59 2.915v122.17c0 1.06.53 2.385 1.59 2.915l28.887 16.695c15.636 7.95 25.44-1.325 25.44-10.6V93.68c0-1.59 1.325-3.18 3.18-3.18h13.25c1.59 0 3.18 1.325 3.18 3.18v120.58c0 20.936-11.396 33.126-31.272 33.126-6.095 0-10.865 0-24.38-6.625L10.865 224.33C4.24 220.62 0 213.465 0 205.78V83.08c0-7.685 4.24-14.84 10.865-18.55L116.605 3.05c6.36-3.445 14.84-3.445 21.2 0L243.545 64.53c6.625 3.71 10.865 10.865 10.865 18.55v122.7c0 7.685-4.24 14.84-10.865 18.55L137.805 285.55c-3.18 1.855-7.155 2.915-9.805 2.915z" />
  </svg>
);

export default NodeVersionDrawer;
