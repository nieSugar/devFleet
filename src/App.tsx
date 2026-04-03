import React, { useState, useCallback } from "react";
import { ConfigProvider, App as AntdApp, theme as antdTheme } from "antd";
import zhCN from "antd/locale/zh_CN";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "antd/dist/reset.css";
import { ThemeProvider, useTheme } from "./contexts/ThemeContext";
import TitleBar from "./components/TitleBar";
import ProjectManager from "./components/ProjectManager";
import NodeVersionDrawer from "./components/NodeVersionDrawer";
import AboutWindow from "./components/AboutWindow";
import ErrorBoundary from "./components/ErrorBoundary";
import "./App.css";

// 自定义 About 窗口目前只在 macOS 菜单栏中创建。
// 其他平台即使共用同一套前端代码，也始终走主应用界面，避免影响 Windows / Linux。
const IS_MACOS =
  typeof window !== "undefined" &&
  /mac/i.test(window.navigator.userAgent);

function getInitialWindowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

const AppContent: React.FC = () => {
  const { isDark } = useTheme();
  const [nodeDrawerOpen, setNodeDrawerOpen] = useState(false);
  const [nvmRefreshKey, setNvmRefreshKey] = useState(0);
  const [windowLabel] = useState(getInitialWindowLabel);
  const handleVersionChange = useCallback(() => {
    setNvmRefreshKey((k) => k + 1);
  }, []);

  // 只有 macOS 且窗口标签为 about 时，才渲染自定义 About 页面。
  // 这样可以明确保证新增界面不会改变 Windows / Linux 的主窗口渲染路径。
  const isAboutWindow = IS_MACOS && windowLabel === "about";

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
          {isAboutWindow ? (
            <AboutWindow />
          ) : (
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
          )}
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
