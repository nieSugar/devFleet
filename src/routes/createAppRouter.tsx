import { getCurrentWindow } from "@tauri-apps/api/window";
import { createMemoryRouter } from "react-router-dom";
import AboutWindow from "../components/AboutWindow";
import SettingsWindow from "../components/SettingsWindow";
import AppShell from "../pages/AppShell";
import MainWindowPage from "../pages/MainWindowPage";

function getInitialWindowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

function resolveInitialRoute() {
  // 只有 About 仍然保留单独窗口；设置改为主窗口内的路由页面。
  switch (getInitialWindowLabel()) {
    case "about":
      return "/about";
    default:
      return "/";
  }
}

export function createAppRouter() {
  return createMemoryRouter(
    [
      {
        path: "/",
        element: <AppShell />,
        children: [
          {
            index: true,
            element: <MainWindowPage />,
          },
          {
            path: "settings",
            element: <SettingsWindow />,
          },
        ],
      },
      {
        path: "/about",
        element: <AboutWindow />,
      },
    ],
    {
      initialEntries: [resolveInitialRoute()],
    },
  );
}
