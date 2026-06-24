import React, { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox, Drawer, Input, Modal, Spin, Switch, Tooltip, message } from "antd";
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
  ClockCircleOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
} from "@ant-design/icons";
import { tauriAPI } from "../lib/tauri";
import type {
  NvmInfo,
  NodeProcessInfo,
  NodeProcessPort,
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
type DrawerTab = "versions" | "processes";

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

const KILL_CONFIRM_STORAGE_KEY = "devfleet.nodeProcess.skipKillConfirm";

function loadSkipKillConfirm() {
  try {
    return window.localStorage.getItem(KILL_CONFIRM_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

function saveSkipKillConfirm(skip: boolean) {
  try {
    if (skip) {
      window.localStorage.setItem(KILL_CONFIRM_STORAGE_KEY, "true");
    } else {
      window.localStorage.removeItem(KILL_CONFIRM_STORAGE_KEY);
    }
  } catch {
    // localStorage may be unavailable in restricted webviews.
  }
}

function shortenMiddle(value: string, max = 86) {
  if (value.length <= max) return value;
  const edge = Math.floor((max - 3) / 2);
  return `${value.slice(0, edge)}...${value.slice(-edge)}`;
}

function formatProcessPort(port: NodeProcessPort) {
  const protocol = port.protocol.toUpperCase();
  const address =
    port.localAddress === "::" ||
    port.localAddress === "0.0.0.0" ||
    port.localAddress === "*"
      ? "*"
      : port.localAddress;
  return `${protocol} ${address}:${port.localPort}`;
}

function normalizeCommandPath(value: string) {
  return value.replace(/\//g, "\\").replace(/\\+/g, "\\");
}

function extractProjectPathFromCommand(command: string, matchedProjectPath?: string) {
  const normalizedCommand = normalizeCommandPath(command).toLowerCase();
  if (
    matchedProjectPath &&
    normalizedCommand.includes(normalizeCommandPath(matchedProjectPath).toLowerCase())
  ) {
    return matchedProjectPath;
  }

  const nodeModulesPath = command.match(
    /(?:"([^"]*node_modules[^"]*)"|'([^']*node_modules[^']*)'|(\S*node_modules\S*))/i
  );
  const rawPath = nodeModulesPath?.[1] || nodeModulesPath?.[2] || nodeModulesPath?.[3];
  if (!rawPath) return null;

  const normalizedPath = normalizeCommandPath(rawPath);
  const markerIndex = normalizedPath.toLowerCase().indexOf("\\node_modules");
  return markerIndex > 0 ? normalizedPath.slice(0, markerIndex) : null;
}

function formatNodeCommandDisplay(command: string, matchedProjectPath?: string) {
  return extractProjectPathFromCommand(command, matchedProjectPath) || command;
}

interface MirrorSelectorProps {
  mirror: string;
  onMirrorChange: (url: string) => void;
}

const MirrorSelector: React.FC<MirrorSelectorProps> = ({ mirror, onMirrorChange }) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [customMode, setCustomMode] = useState(false);
  const [customUrl, setCustomUrl] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  const isPreset = MIRROR_PRESETS.some((p) => p.url === mirror);
  const matchedPreset = MIRROR_PRESETS.find((p) => p.url === mirror);
  const activeLabel = matchedPreset
    ? matchedPreset.labelKey === "npmmirror"
      ? "npmmirror"
      : t(matchedPreset.labelKey)
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
              {p.url && <span className="nd-mirror-option-url">{p.url}</span>}
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
              <span className="nd-mirror-option-label">
                {t("nodeDrawer.mirror.customMirror")}
              </span>
              {!isPreset && mirror && <CheckOutlined className="nd-mirror-check" />}
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
  const truncated = displayDir.length > 35 ? "..." + displayDir.slice(-32) : displayDir;

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
        title={t("nodeDrawer.installDir.tooltip", {
          dir: dir || t("nodeDrawer.installDir.default"),
        })}
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
  const [remoteVersions, setRemoteVersions] = useState<RemoteNodeVersion[]>([]);
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
  const [powerShellPolicyReady, setPowerShellPolicyReady] = useState(true);
  const [pathSetupBusy, setPathSetupBusy] = useState(false);
  const [activeTab, setActiveTab] = useState<DrawerTab>("versions");
  const [nodeProcesses, setNodeProcesses] = useState<NodeProcessInfo[]>([]);
  const [processLoading, setProcessLoading] = useState(false);
  const [processSearch, setProcessSearch] = useState("");
  const [onlyPortProcesses, setOnlyPortProcesses] = useState(false);
  const [killingPid, setKillingPid] = useState<number | null>(null);
  const [pendingKillProcess, setPendingKillProcess] = useState<NodeProcessInfo | null>(
    null
  );
  const [rememberKillChoice, setRememberKillChoice] = useState(false);
  const [skipKillConfirm, setSkipKillConfirm] = useState(loadSkipKillConfirm);
  const versionFetchInFlightRef = useRef(false);
  const versionFetchRequestIdRef = useRef(0);
  const processLoadInFlightRef = useRef(false);
  const processLoadRequestIdRef = useRef(0);

  const checkPathStatus = useCallback(() => {
    tauriAPI.checkNodeInPath().then((res) => {
      if (res.success && res.data) {
        setNodeInPath(res.data.inPath);
        setNodeAvailable(res.data.nodeAvailable);
        setPowerShellPolicyReady(res.data.powerShellPolicyReady);
      }
    });
  }, []);

  useEffect(() => {
    if (!open) {
      setPendingKillProcess(null);
      setRememberKillChoice(false);
    }
  }, [open]);

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
    return () => {
      cancelled = true;
    };
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
    [mirror, messageApi, t]
  );

  const handleSetupPath = useCallback(async () => {
    setPathSetupBusy(true);
    try {
      const res = await tauriAPI.setupNodeGlobalPath();
      if (res.success && res.data) {
        messageApi.success(res.data.message);
        setNodeInPath(true);
        setPowerShellPolicyReady(true);
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
    [installDir, messageApi, t]
  );

  const fetchData = useCallback(async () => {
    if (versionFetchInFlightRef.current) return;

    const requestId = versionFetchRequestIdRef.current + 1;
    versionFetchRequestIdRef.current = requestId;
    versionFetchInFlightRef.current = true;
    setLoading(true);
    try {
      const [remoteResult, nvmResult] = await Promise.all([
        tauriAPI.fetchRemoteNodeVersions(),
        tauriAPI.getNvmInfo(),
      ]);
      if (versionFetchRequestIdRef.current !== requestId) return;
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
      if (versionFetchRequestIdRef.current === requestId) {
        messageApi.error(t("nodeDrawer.networkError"));
      }
    } finally {
      if (versionFetchRequestIdRef.current === requestId) {
        versionFetchInFlightRef.current = false;
        setLoading(false);
      }
    }
  }, [messageApi, t]);

  useEffect(() => {
    if (!open) {
      versionFetchRequestIdRef.current += 1;
      versionFetchInFlightRef.current = false;
      setLoading(false);
      return;
    }

    if (!hasFetched) {
      void fetchData();
    }
  }, [open, hasFetched, fetchData]);

  const refreshNvmInfo = useCallback(async () => {
    const nvmResult = await tauriAPI.getNvmInfo();
    if (nvmResult.success && nvmResult.data) setNvmInfo(nvmResult.data);
    checkPathStatus();
  }, [checkPathStatus]);

  const loadNodeProcesses = useCallback(
    async (notifyError = true) => {
      if (processLoadInFlightRef.current) return;

      const requestId = processLoadRequestIdRef.current + 1;
      processLoadRequestIdRef.current = requestId;
      processLoadInFlightRef.current = true;
      if (notifyError) setProcessLoading(true);
      try {
        const result = await tauriAPI.listNodeProcesses();
        if (processLoadRequestIdRef.current !== requestId) return;
        if (result.success && result.data) {
          setNodeProcesses(result.data);
        } else if (notifyError) {
          messageApi.error(result.error || t("nodeDrawer.processes.loadFailed"));
        }
      } catch {
        if (notifyError && processLoadRequestIdRef.current === requestId) {
          messageApi.error(t("nodeDrawer.processes.loadFailed"));
        }
      } finally {
        if (processLoadRequestIdRef.current === requestId) {
          processLoadInFlightRef.current = false;
          if (notifyError) setProcessLoading(false);
        }
      }
    },
    [messageApi, t]
  );

  useEffect(() => {
    if (open && activeTab === "processes") return;

    processLoadRequestIdRef.current += 1;
    processLoadInFlightRef.current = false;
    setProcessLoading(false);
  }, [open, activeTab]);

  useEffect(() => {
    if (!open || activeTab !== "processes") return;

    void loadNodeProcesses();
    const timer = window.setInterval(() => {
      void loadNodeProcesses(false);
    }, 5000);

    return () => window.clearInterval(timer);
  }, [open, activeTab, loadNodeProcesses]);

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
        const matchLts = typeof rv.lts === "string" && rv.lts.toLowerCase().includes(q);
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

  const filteredNodeProcesses = useMemo(() => {
    const q = processSearch.trim().toLowerCase();
    const visibleProcesses = onlyPortProcesses
      ? nodeProcesses.filter((process) => (process.ports || []).length > 0)
      : nodeProcesses;
    if (!q) return visibleProcesses;

    return visibleProcesses.filter((process) => {
      const command = process.commandLine || process.executable || process.name;
      return [
        process.pid.toString(),
        process.parentPid?.toString() || "",
        process.name,
        process.executable || "",
        process.commandLine || "",
        process.launchCommand || "",
        formatNodeCommandDisplay(command, process.matchedProjectPath),
        ...(process.ports || []).map((port) => formatProcessPort(port)),
        process.matchedProjectName || "",
        process.matchedProjectPath || "",
      ]
        .join(" ")
        .toLowerCase()
        .includes(q);
    });
  }, [nodeProcesses, onlyPortProcesses, processSearch]);

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
      errorMsg: string
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
    [nvmInfo, messageApi, refreshNvmInfo, onVersionChange]
  );

  const handleInstall = useCallback(
    (version: string) =>
      handleVersionAction(
        version,
        "install",
        tauriAPI.installNodeVersion,
        t("nodeDrawer.installFailed")
      ),
    [handleVersionAction, t]
  );

  const handleSwitch = useCallback(
    (version: string) =>
      handleVersionAction(
        version,
        "switch",
        tauriAPI.switchNodeVersion,
        t("nodeDrawer.switchFailed")
      ),
    [handleVersionAction, t]
  );

  const handleUninstall = useCallback(
    (version: string) =>
      handleVersionAction(
        version,
        "uninstall",
        tauriAPI.uninstallNodeVersion,
        t("nodeDrawer.uninstallFailed")
      ),
    [handleVersionAction, t]
  );

  const executeKillNodeProcess = useCallback(
    async (process: NodeProcessInfo, rememberSkipConfirm = false) => {
      setKillingPid(process.pid);
      try {
        const result = await tauriAPI.killNodeProcess(process.pid);
        if (result.success) {
          if (rememberSkipConfirm) {
            saveSkipKillConfirm(true);
            setSkipKillConfirm(true);
          }
          setPendingKillProcess(null);
          setRememberKillChoice(false);
          setNodeProcesses((prev) =>
            prev.filter((item) => item.pid !== process.pid)
          );
          messageApi.success(
            result.data?.message || t("nodeDrawer.processes.killSuccess")
          );
          await loadNodeProcesses(false);
        } else {
          messageApi.error(result.error || t("nodeDrawer.processes.killFailed"));
        }
      } catch {
        messageApi.error(t("nodeDrawer.processes.killFailed"));
      } finally {
        setKillingPid(null);
      }
    },
    [loadNodeProcesses, messageApi, t]
  );

  const handleKillNodeProcess = useCallback(
    (process: NodeProcessInfo) => {
      if (skipKillConfirm) {
        void executeKillNodeProcess(process);
        return;
      }

      setRememberKillChoice(false);
      setPendingKillProcess(process);
    },
    [executeKillNodeProcess, skipKillConfirm]
  );

  const handleConfirmKillNodeProcess = useCallback(async () => {
    if (!pendingKillProcess) return;

    await executeKillNodeProcess(pendingKillProcess, rememberKillChoice);
  }, [executeKillNodeProcess, pendingKillProcess, rememberKillChoice]);

  const handleCancelKillNodeProcess = useCallback(() => {
    if (killingPid !== null) return;
    setPendingKillProcess(null);
    setRememberKillChoice(false);
  }, [killingPid]);

  const pendingKillCommand = pendingKillProcess
    ? pendingKillProcess.commandLine ||
      pendingKillProcess.executable ||
      pendingKillProcess.name
    : "";

  const isKillingPendingProcess =
    pendingKillProcess !== null && killingPid === pendingKillProcess.pid;

  const handleRefresh = () => {
    if (activeTab === "processes") {
      void loadNodeProcesses();
      return;
    }

    if (versionFetchInFlightRef.current) return;
    setHasFetched(false);
    setExpandedGroups(new Set());
    void fetchData();
  };

  const filterButtons: { key: FilterMode; label: string }[] = [
    { key: "all", label: t("nodeDrawer.filterAll") },
    { key: "lts", label: t("nodeDrawer.filterLts") },
    { key: "installed", label: t("nodeDrawer.filterInstalled") },
  ];

  const isBusy = busy !== null;
  const isRefreshing = activeTab === "processes" ? processLoading : loading;

  return (
    <>
      {contextHolder}

      <Drawer
        open={open}
        onClose={onClose}
        width={640}
        closable={false}
        className="node-drawer"
        styles={{ body: { padding: 0 }, header: { display: "none" } }}
      >
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
                  disabled={isRefreshing}
                >
                  <ReloadOutlined spin={isRefreshing} />
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

          {activeTab === "versions" && nvmInfo && (
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
                  <span className="nd-current-num">v{nvmInfo.currentVersion}</span>
                </div>
              )}
              {remoteVersions.length > 0 && (
                <span className="nd-total-count">
                  {t("common.versions", { count: remoteVersions.length })}
                </span>
              )}
            </div>
          )}

          {activeTab === "processes" && (
            <div className="nd-meta">
              <div className="nd-manager-badge">
                <span
                  className={`nd-manager-dot ${nodeProcesses.length > 0 ? "active" : ""}`}
                />
                <span>
                  {t("nodeDrawer.processes.count", {
                    count: nodeProcesses.length,
                  })}
                </span>
              </div>
              <span className="nd-total-count">
                {t("nodeDrawer.processes.autoRefresh")}
              </span>
            </div>
          )}

          {activeTab === "versions" && (
            <div
              className="nd-meta"
              style={{ marginTop: nvmInfo ? 8 : 14, gap: 8, flexWrap: "wrap" }}
            >
              <MirrorSelector mirror={mirror} onMirrorChange={handleMirrorChange} />
              {nvmInfo?.manager === "builtin" && (
                <InstallDirSelector
                  dir={installDir}
                  resolvedDir={installDirDisplay}
                  onDirChange={handleInstallDirChange}
                />
              )}
            </div>
          )}

          {activeTab === "versions" &&
            nvmInfo?.manager === "builtin" &&
            nvmInfo.currentVersion &&
            ((!nodeInPath && !nodeAvailable) || !powerShellPolicyReady) && (
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

          <div className="nd-tabs">
            <button
              className={`nd-tab-btn ${activeTab === "versions" ? "active" : ""}`}
              onClick={() => setActiveTab("versions")}
            >
              {t("nodeDrawer.tabs.versions")}
            </button>
            <button
              className={`nd-tab-btn ${activeTab === "processes" ? "active" : ""}`}
              onClick={() => setActiveTab("processes")}
            >
              {t("nodeDrawer.tabs.processes")}
              {nodeProcesses.length > 0 && (
                <span className="nd-tab-count">{nodeProcesses.length}</span>
              )}
            </button>
          </div>
        </div>

        {activeTab === "versions" ? (
          <>
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
                          <span className="nd-group-major">Node.js {group.major}</span>
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
          </>
        ) : (
          <>
            <div className="nd-toolbar">
              <Input
                placeholder={t("nodeDrawer.processes.searchPlaceholder")}
                prefix={<SearchOutlined style={{ color: "var(--text-muted)" }} />}
                value={processSearch}
                onChange={(e) => setProcessSearch(e.target.value)}
                allowClear
                className="nd-search"
              />
              <label className="nd-port-filter">
                <Switch
                  size="small"
                  checked={onlyPortProcesses}
                  onChange={setOnlyPortProcesses}
                />
                <span>{t("nodeDrawer.processes.portOnly")}</span>
              </label>
            </div>

            <div className="nd-content">
              {processLoading && nodeProcesses.length === 0 ? (
                <div className="nd-loading">
                  <Spin indicator={<LoadingOutlined style={{ fontSize: 28 }} />} />
                  <span className="nd-loading-text">
                    {t("nodeDrawer.processes.loading")}
                  </span>
                </div>
              ) : filteredNodeProcesses.length === 0 ? (
                <div className="nd-empty">
                  <ApiOutlined className="nd-empty-icon" />
                  <span className="nd-empty-text">
                    {processSearch
                      ? t("nodeDrawer.processes.noMatch")
                      : onlyPortProcesses
                        ? t("nodeDrawer.processes.noPortProcesses")
                        : t("nodeDrawer.processes.noProcesses")}
                  </span>
                  {!processSearch && !onlyPortProcesses && (
                    <button className="nd-retry-btn" onClick={() => loadNodeProcesses()}>
                      {t("common.refresh")}
                    </button>
                  )}
                </div>
              ) : (
                filteredNodeProcesses.map((process, index) => (
                  <NodeProcessRow
                    key={process.pid}
                    process={process}
                    index={index}
                    killing={killingPid === process.pid}
                    onKill={handleKillNodeProcess}
                  />
                ))
              )}
            </div>
          </>
        )}
      </Drawer>

      <Modal
        open={pendingKillProcess !== null}
        zIndex={2200}
        title={
          pendingKillProcess
            ? t("nodeDrawer.processes.killTitle", { pid: pendingKillProcess.pid })
            : undefined
        }
        okText={t("nodeDrawer.processes.killConfirm")}
        okType="danger"
        cancelText={t("common.cancel")}
        confirmLoading={isKillingPendingProcess}
        maskClosable={!isKillingPendingProcess}
        closable={!isKillingPendingProcess}
        onOk={handleConfirmKillNodeProcess}
        onCancel={handleCancelKillNodeProcess}
      >
        <div className="nd-process-confirm">
          <p>{t("nodeDrawer.processes.killContent")}</p>
          <code title={pendingKillCommand}>{shortenMiddle(pendingKillCommand, 120)}</code>
          <Checkbox
            className="nd-process-remember"
            checked={rememberKillChoice}
            disabled={isKillingPendingProcess}
            onChange={(event) => setRememberKillChoice(event.target.checked)}
          >
            {t("nodeDrawer.processes.rememberKillChoice")}
          </Checkbox>
        </div>
      </Modal>
    </>
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
        {typeof v.lts === "string" && <span className="nd-version-lts">{v.lts}</span>}
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

interface NodeProcessRowProps {
  process: NodeProcessInfo;
  index: number;
  killing: boolean;
  onKill: (process: NodeProcessInfo) => void;
}

const NodeProcessRow: React.FC<NodeProcessRowProps> = ({
  process,
  index,
  killing,
  onKill,
}) => {
  const { t } = useTranslation();
  const command = process.commandLine || process.executable || process.name;
  const nodeCommandDisplay = formatNodeCommandDisplay(
    command,
    process.matchedProjectPath
  );
  const launchCommand = process.launchCommand;
  const ports = process.ports || [];
  const portSummary = ports.map(formatProcessPort).join(", ");
  const hasMatchedProject = Boolean(process.matchedProjectName);
  const locationDisplay = process.matchedProjectPath || nodeCommandDisplay;
  const primaryDisplay = process.matchedProjectName || locationDisplay;

  return (
    <div
      className={`nd-process-row ${hasMatchedProject ? "matched" : ""}`}
      style={{ animationDelay: `${index * 30}ms` }}
    >
      <div className="nd-process-main">
        <div className="nd-process-head">
          <div className="nd-process-identity">
            <span
              className={`nd-process-title ${hasMatchedProject ? "project" : "path"}`}
              title={primaryDisplay}
            >
              {primaryDisplay}
            </span>
            <div className="nd-process-meta">
              <span className="nd-process-runtime">{process.name}</span>
              <span className="nd-process-pid">
                {t("nodeDrawer.processes.pid", { pid: process.pid })}
              </span>
              {process.parentPid && (
                <span className="nd-process-ppid">
                  {t("nodeDrawer.processes.ppid", { pid: process.parentPid })}
                </span>
              )}
              {process.startedAt && (
                <span className="nd-process-started">
                  <ClockCircleOutlined />
                  {process.startedAt}
                </span>
              )}
            </div>
          </div>

          {ports.length > 0 ? (
            <div className="nd-process-ports" title={portSummary}>
              {ports.slice(0, 4).map((port) => (
                <span
                  key={`${port.protocol}-${port.localAddress}-${port.localPort}`}
                  className="nd-process-port"
                >
                  {formatProcessPort(port)}
                </span>
              ))}
              {ports.length > 4 && (
                <span className="nd-process-port">
                  {t("nodeDrawer.processes.morePorts", {
                    count: ports.length - 4,
                  })}
                </span>
              )}
            </div>
          ) : (
            <span className="nd-process-no-port">
              {t("nodeDrawer.processes.noPorts")}
            </span>
          )}

          {hasMatchedProject && locationDisplay && (
            <div className="nd-process-path" title={locationDisplay}>
              <FolderOpenOutlined />
              <span>{locationDisplay}</span>
            </div>
          )}
        </div>

        {launchCommand && (
          <div className="nd-process-command launch" title={launchCommand}>
            <PlayCircleOutlined />
            <span>
              {t("nodeDrawer.processes.launchCommand")}: {shortenMiddle(launchCommand)}
            </span>
          </div>
        )}
      </div>

      <Tooltip title={t("nodeDrawer.processes.killTooltip")}>
        <button
          type="button"
          className={`nd-kill-btn ${killing ? "killing" : ""}`}
          onClick={() => onKill(process)}
          disabled={killing}
        >
          {killing ? <LoadingOutlined /> : <PoweroffOutlined />}
        </button>
      </Tooltip>
    </div>
  );
};

const NodeIcon: React.FC = () => (
  <svg width="16" height="16" viewBox="0 0 256 289" fill="currentColor">
    <path d="M128 288.464c-3.975 0-7.685-1.06-11.13-2.915l-35.247-20.936c-5.3-2.915-2.65-3.975-1.06-4.505 7.155-2.385 8.48-2.915 15.9-7.155.796-.53 1.856-.265 2.65.265l27.032 16.166c1.06.53 2.385.53 3.18 0l105.74-61.217c1.06-.53 1.59-1.59 1.59-2.915V83.08c0-1.325-.53-2.385-1.59-2.915L129.06 19.213c-1.06-.53-2.385-.53-3.18 0L20.14 80.43c-1.06.53-1.59 1.855-1.59 2.915v122.17c0 1.06.53 2.385 1.59 2.915l28.887 16.695c15.636 7.95 25.44-1.325 25.44-10.6V93.68c0-1.59 1.325-3.18 3.18-3.18h13.25c1.59 0 3.18 1.325 3.18 3.18v120.58c0 20.936-11.396 33.126-31.272 33.126-6.095 0-10.865 0-24.38-6.625L10.865 224.33C4.24 220.62 0 213.465 0 205.78V83.08c0-7.685 4.24-14.84 10.865-18.55L116.605 3.05c6.36-3.445 14.84-3.445 21.2 0L243.545 64.53c6.625 3.71 10.865 10.865 10.865 18.55v122.7c0 7.685-4.24 14.84-10.865 18.55L137.805 285.55c-3.18 1.855-7.155 2.915-9.805 2.915z" />
  </svg>
);

export default NodeVersionDrawer;
