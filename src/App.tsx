import React from "react";
import ProjectManager from "./components/ProjectManager";
import "./App.css";
import { ConfigProvider, App as AntdApp } from "antd";
import zhCN from "antd/locale/zh_CN";
import "antd/dist/reset.css";

const App: React.FC = () => {
  return (
    <ConfigProvider locale={zhCN} theme={{ token: { colorPrimary: "#667eea" } }}>
      <AntdApp>
        <div className="app">
          <main className="app-main">
            <ProjectManager />
          </main>
        </div>
      </AntdApp>
    </ConfigProvider>
  );
};

export default App;
