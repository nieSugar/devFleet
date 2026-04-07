import React, { useCallback, useEffect, useState } from "react";
import { Outlet, useNavigate } from "react-router-dom";
import TitleBar from "../components/TitleBar";
import NodeVersionDrawer from "../components/NodeVersionDrawer";
import { listenForMacOSOpenSettings } from "../lib/macosNative";
import "../App.css";

const AppShell: React.FC = () => {
  const [nodeDrawerOpen, setNodeDrawerOpen] = useState(false);
  const [nvmRefreshKey, setNvmRefreshKey] = useState(0);
  const navigate = useNavigate();

  const handleVersionChange = useCallback(() => {
    setNvmRefreshKey((k) => k + 1);
  }, []);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    // macOS 原生菜单的“设置”会切回主窗口并导航到 /settings。
    void listenForMacOSOpenSettings(() => {
      navigate("/settings");
    }).then((cleanup) => {
      if (cancelled) cleanup();
      else unlisten = cleanup;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [navigate]);

  return (
    <div className="app">
      <TitleBar onOpenNodeManager={() => setNodeDrawerOpen(true)} />
      <main className="app-main">
        <Outlet context={{ nvmRefreshKey }} />
      </main>
      <NodeVersionDrawer
        open={nodeDrawerOpen}
        onClose={() => setNodeDrawerOpen(false)}
        onVersionChange={handleVersionChange}
      />
    </div>
  );
};

export default AppShell;
