import React, { useMemo, useState } from "react";
import {
  StyleProvider,
  legacyLogicalPropertiesTransformer,
} from "@ant-design/cssinjs/es";
import { ConfigProvider, App as AntdApp, theme as antdTheme } from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import { useTranslation } from "react-i18next";
import { RouterProvider } from "react-router-dom";
import "antd/dist/reset.css";
import { ThemeProvider, useTheme } from "./contexts/ThemeContext";
import ErrorBoundary from "./components/ErrorBoundary";
import { createAppRouter } from "./routes/createAppRouter";
import "./App.css";

const ANTD_LOCALES: Record<string, typeof zhCN> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};
const ANTD_COMPAT_TRANSFORMERS = [legacyLogicalPropertiesTransformer];

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

  return (
    <StyleProvider
      // Tauri 打包后运行在系统 WebView 中；某些旧版 WebKit 对 :where 选择器
      // 和 CSS 逻辑属性支持不完整，Ant Design 官方建议在这类环境开启兼容降级。
      hashPriority="high"
      transformers={ANTD_COMPAT_TRANSFORMERS}
    >
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
            <RouterProvider router={router} />
          </ErrorBoundary>
        </AntdApp>
      </ConfigProvider>
    </StyleProvider>
  );
};

const App: React.FC = () => (
  <ThemeProvider>
    <AppContent />
  </ThemeProvider>
);

export default App;
