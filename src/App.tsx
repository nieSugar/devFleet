import React, { Suspense, useState, useCallback, useMemo } from "react";
import { ConfigProvider, App as AntdApp, theme as antdTheme } from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "antd/dist/reset.css";
import { ThemeProvider, useTheme } from "./contexts/ThemeContext";
import TitleBar from "./components/TitleBar";
import ProjectManager from "./components/ProjectManager";
import NodeVersionDrawer from "./components/NodeVersionDrawer";
import ErrorBoundary from "./components/ErrorBoundary";
import "./App.css";

// About 窗口只在 macOS 菜单中打开，按需加载可以避免 Windows / Linux 主入口主动引入它。
const AboutWindow = React.lazy(() => import("./components/AboutWindow"));
const SettingsWindow = React.lazy(() => import("./components/SettingsWindow"));

// 自定义 About 窗口目前只在 macOS 菜单栏中创建。
// 其他平台即使共用同一套前端代码，也始终走主应用界面，避免影响 Windows / Linux。
const IS_MACOS = typeof window !== "undefined" && /mac/i.test(window.navigator.userAgent);

function getInitialWindowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

const ANTD_LOCALES: Record<string, typeof zhCN> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

function resolveAntdLocale(language: string) {
  if (ANTD_LOCALES[language]) return ANTD_LOCALES[language];
  if (language.startsWith("zh")) return zhCN;
  return enUS;
}

const AppContent: React.FC = () => {
  const { isDark } = useTheme();
  const { i18n } = useTranslation();
  const [nodeDrawerOpen, setNodeDrawerOpen] = useState(false);
  const [nvmRefreshKey, setNvmRefreshKey] = useState(0);
  const [windowLabel] = useState(getInitialWindowLabel);
  const handleVersionChange = useCallback(() => {
    setNvmRefreshKey((k) => k + 1);
  }, []);

  const antdLocale = useMemo(() => resolveAntdLocale(i18n.language), [i18n.language]);

  const isAboutWindow = IS_MACOS && windowLabel === "about";
  const isSettingsWindow = IS_MACOS && windowLabel === "settings";

  return (
    <ConfigProvider
      locale={antdLocale}
      theme={{
        algorithm: isDark ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
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
            <Suspense fallback={null}>
              <AboutWindow />
            </Suspense>
          ) : isSettingsWindow ? (
            <Suspense fallback={null}>
              <SettingsWindow />
            </Suspense>
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
