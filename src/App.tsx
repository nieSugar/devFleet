import React, { useState, useCallback } from "react";
import { ConfigProvider, App as AntdApp, theme as antdTheme } from "antd";
import zhCN from "antd/locale/zh_CN";
import "antd/dist/reset.css";
import { ThemeProvider, useTheme } from "./contexts/ThemeContext";
import TitleBar from "./components/TitleBar";
import ProjectManager from "./components/ProjectManager";
import NodeVersionDrawer from "./components/NodeVersionDrawer";
import ErrorBoundary from "./components/ErrorBoundary";
import "./App.css";

const AppContent: React.FC = () => {
  const { isDark } = useTheme();
  const [nodeDrawerOpen, setNodeDrawerOpen] = useState(false);
  const [nvmRefreshKey, setNvmRefreshKey] = useState(0);
  const handleVersionChange = useCallback(() => {
    setNvmRefreshKey((k) => k + 1);
  }, []);

  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: isDark
          ? antdTheme.darkAlgorithm
          : antdTheme.defaultAlgorithm,
        token: {
          colorPrimary: "#0d9488",
          borderRadius: 8,
          fontFamily:
            "'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, 'Microsoft YaHei', sans-serif",
          colorBgContainer: isDark ? "#161a24" : "#ffffff",
          colorBgElevated: isDark ? "#1c2130" : "#ffffff",
          colorBgLayout: isDark ? "#0e1118" : "#f5f7fa",
          colorBorder: isDark ? "#262c3a" : "#dfe2e8",
          colorBorderSecondary: isDark ? "#1a1f2c" : "#eceff4",
        },
      }}
    >
      <AntdApp>
        <ErrorBoundary>
          <div className="app">
            <TitleBar onOpenNodeManager={() => setNodeDrawerOpen(true)} />
            <main className="app-main">
              <ProjectManager nvmRefreshKey={nvmRefreshKey} />
            </main>
            <NodeVersionDrawer
              open={nodeDrawerOpen}
              onClose={() => setNodeDrawerOpen(false)}
              onVersionChange={handleVersionChange}
            />
          </div>
        </ErrorBoundary>
      </AntdApp>
    </ConfigProvider>
  );
};

const App: React.FC = () => (
  <ThemeProvider>
    <AppContent />
  </ThemeProvider>
);

export default App;
