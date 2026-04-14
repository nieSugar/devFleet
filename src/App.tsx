import React, { useMemo, useState } from "react";
import { StyleProvider } from "@ant-design/cssinjs";
import { ConfigProvider, App as AntdApp } from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import { useTranslation } from "react-i18next";
import { RouterProvider } from "react-router-dom";
import "antd/dist/reset.css";
import { ThemeProvider, useTheme } from "./contexts/ThemeContext";
import ErrorBoundary from "./components/ErrorBoundary";
import { createAppRouter } from "./routes/createAppRouter";
import {
  DEVFLEET_ANTD_COMPAT_TRANSFORMERS,
  getDevFleetAntdThemeConfig,
} from "./theme/antdTheme";
import "./App.css";

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
  const [router] = useState(createAppRouter);
  const antdLocale = useMemo(() => resolveAntdLocale(i18n.language), [i18n.language]);
  const antdTheme = useMemo(() => getDevFleetAntdThemeConfig(isDark), [isDark]);
  const appTree = (
    <ConfigProvider
      locale={antdLocale}
      theme={antdTheme}
    >
      <AntdApp>
        <ErrorBoundary>
          <RouterProvider router={router} />
        </ErrorBoundary>
      </AntdApp>
    </ConfigProvider>
  );

  if (import.meta.env.DEV) {
    return appTree;
  }

  return (
    <StyleProvider
      // Tauri 打包后运行在系统 WebView 中；某些旧版 WebKit 对 :where 选择器
      // 和 CSS 逻辑属性支持不完整，Ant Design 官方建议在这类环境开启兼容降级。
      // 同时显式把 style 标签注入到 head，避免 macOS 生产包里样式插入位置不稳定。
      hashPriority="high"
      container={typeof document === "undefined" ? undefined : document.head}
      transformers={DEVFLEET_ANTD_COMPAT_TRANSFORMERS}
    >
      {appTree}
    </StyleProvider>
  );
};

const App: React.FC = () => (
  <ThemeProvider>
    <AppContent />
  </ThemeProvider>
);

export default App;
