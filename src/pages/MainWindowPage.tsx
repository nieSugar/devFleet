import React from "react";
import { useOutletContext } from "react-router-dom";
import ProjectManager from "../components/ProjectManager";
import type { AppShellContext } from "./AppShell";

const MainWindowPage: React.FC = () => {
  const context = useOutletContext<AppShellContext>();

  return <ProjectManager {...context} />;
};

export default MainWindowPage;
