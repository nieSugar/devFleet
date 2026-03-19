import React from "react";
import ProjectManager from "./components/ProjectManager";
import ErrorBoundary from "./components/ErrorBoundary";
import "./App.css";
import { ConfigProvider, App as AntdApp } from "antd";
import zhCN from "antd/locale/zh_CN";
import "antd/dist/reset.css";

const App: React.FC = () => {
  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        token: {
          colorPrimary: "#0d9488",
          colorBgContainer: "#ffffff",
          colorBorderSecondary: "#e2e8f0",
          borderRadius: 8,
        },
      }}
    >
      <AntdApp>
        <ErrorBoundary>
          <div className="app">
            <main className="app-main">
              <ProjectManager />
            </main>
          </div>
        </ErrorBoundary>
      </AntdApp>
    </ConfigProvider>
  );
};

export default App;
