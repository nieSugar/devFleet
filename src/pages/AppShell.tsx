import React, { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { exit as exitApp } from "@tauri-apps/plugin-process";
import { Button, Checkbox, Modal } from "antd";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import TitleBar from "../components/TitleBar";
import NodeVersionDrawer from "../components/NodeVersionDrawer";
import { listenForMacOSOpenSettings } from "../lib/macosNative";
import { useNodeProcesses, type NodeProcessesState } from "../hooks/useNodeProcesses";
import type { Project } from "../types/project";
import "../App.css";

type CloseBehavior = "minimize" | "quit";

export interface AppShellContext {
  nvmRefreshKey: number;
  processState: NodeProcessesState;
  onOpenNodeManager: () => void;
  onViewProcesses: (project: Project) => void;
}

const CLOSE_BEHAVIOR_KEY = "devfleet.closeBehavior";

function loadCloseBehavior(): CloseBehavior | null {
  try {
    const value = window.localStorage.getItem(CLOSE_BEHAVIOR_KEY);
    return value === "minimize" || value === "quit" ? value : null;
  } catch {
    return null;
  }
}

function saveCloseBehavior(behavior: CloseBehavior) {
  try {
    window.localStorage.setItem(CLOSE_BEHAVIOR_KEY, behavior);
  } catch {
    // 忽略存储失败，确保本次关闭动作继续执行。
  }
}

const AppShell: React.FC = () => {
  const { t } = useTranslation();
  const [nodeDrawerOpen, setNodeDrawerOpen] = useState(false);
  const [nodeDrawerTab, setNodeDrawerTab] = useState<"versions" | "processes">("versions");
  const [processProject, setProcessProject] = useState<Project | null>(null);
  const [nvmRefreshKey, setNvmRefreshKey] = useState(0);
  const [closePromptOpen, setClosePromptOpen] = useState(false);
  const [rememberCloseChoice, setRememberCloseChoice] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const processState = useNodeProcesses(location.pathname === "/" || (nodeDrawerOpen && nodeDrawerTab === "processes"));

  const openNodeManager = useCallback(() => {
    setNodeDrawerTab("versions");
    setProcessProject(null);
    setNodeDrawerOpen(true);
  }, []);

  const viewProjectProcesses = useCallback((project: Project) => {
    setProcessProject(project);
    setNodeDrawerTab("processes");
    setNodeDrawerOpen(true);
  }, []);

  const handleVersionChange = useCallback(() => {
    setNvmRefreshKey((k) => k + 1);
  }, []);

  const performCloseBehavior = useCallback(async (behavior: CloseBehavior) => {
    setClosePromptOpen(false);
    if (behavior === "minimize") {
      await getCurrentWindow().hide();
      return;
    }

    await exitApp(0);
  }, []);

  const requestClose = useCallback(async () => {
    const savedBehavior = loadCloseBehavior();
    if (savedBehavior) {
      await performCloseBehavior(savedBehavior);
      return;
    }

    setClosePromptOpen(true);
  }, [performCloseBehavior]);

  const applyCloseBehavior = useCallback(
    async (behavior: CloseBehavior) => {
      if (rememberCloseChoice) {
        saveCloseBehavior(behavior);
      }

      await performCloseBehavior(behavior);
    },
    [performCloseBehavior, rememberCloseChoice],
  );

  useEffect(() => {
    let cancelled = false;
    let settingsUnlisten: (() => void) | undefined;
    let closeUnlisten: (() => void) | undefined;

    // macOS 原生菜单的“设置”会切回主窗口并导航到 /settings。
    void listenForMacOSOpenSettings(() => {
      navigate("/settings");
    }).then((cleanup) => {
      if (cancelled) cleanup();
      else settingsUnlisten = cleanup;
    });

    void getCurrentWindow().onCloseRequested((event) => {
      event.preventDefault();
      void requestClose();
    }).then((cleanup) => {
      if (cancelled) cleanup();
      else closeUnlisten = cleanup;
    });

    return () => {
      cancelled = true;
      settingsUnlisten?.();
      closeUnlisten?.();
    };
  }, [navigate, requestClose]);

  return (
    <div className="app">
      <TitleBar
        onOpenNodeManager={openNodeManager}
        onRequestClose={() => void requestClose()}
      />
      <main className="app-main">
        <Outlet context={{ nvmRefreshKey, processState, onOpenNodeManager: openNodeManager, onViewProcesses: viewProjectProcesses } satisfies AppShellContext} />
      </main>
      <NodeVersionDrawer
        open={nodeDrawerOpen}
        onClose={() => setNodeDrawerOpen(false)}
        onVersionChange={handleVersionChange}
        activeTab={nodeDrawerTab}
        onTabChange={setNodeDrawerTab}
        processState={processState}
        processProject={processProject}
        onClearProcessProject={() => setProcessProject(null)}
      />
      <Modal
        open={closePromptOpen}
        title={t("closeBehavior.title")}
        onCancel={() => setClosePromptOpen(false)}
        footer={[
          <Button key="quit" onClick={() => void applyCloseBehavior("quit")}>
            {t("closeBehavior.quit")}
          </Button>,
          <Button
            key="minimize"
            type="primary"
            onClick={() => void applyCloseBehavior("minimize")}
          >
            {t("closeBehavior.minimize")}
          </Button>,
        ]}
      >
        <p>{t("closeBehavior.description")}</p>
        <Checkbox
          checked={rememberCloseChoice}
          onChange={(event) => setRememberCloseChoice(event.target.checked)}
        >
          {t("closeBehavior.remember")}
        </Checkbox>
      </Modal>
    </div>
  );
};

export default AppShell;
