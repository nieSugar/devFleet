import React from "react";
import { useOutletContext } from "react-router-dom";
import ProjectManager from "../components/ProjectManager";
type AppShellContext = {
  nvmRefreshKey: number;
};

const MainWindowPage: React.FC = () => {
  const { nvmRefreshKey } = useOutletContext<AppShellContext>();

  return <ProjectManager nvmRefreshKey={nvmRefreshKey} />;
};

export default MainWindowPage;
