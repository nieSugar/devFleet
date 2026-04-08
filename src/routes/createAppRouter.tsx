import { getCurrentWindow } from "@tauri-apps/api/window";
import { createMemoryRouter } from "react-router-dom";
import AboutWindow from "../components/AboutWindow";
import SettingsWindow from "../components/SettingsWindow";
import AppShell from "../pages/AppShell";
import MainWindowPage from "../pages/MainWindowPage";

function getInjectedWindowKind() {
  if (typeof window === "undefined") return null;
  return window.__DEVFLEET_WINDOW_KIND__ ?? null;
}

function getInitialWindowLabel() {
  const injectedWindowKind = getInjectedWindowKind();
  if (injectedWindowKind) {
    return injectedWindowKind;
  }

  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

function resolveInitialRoute() {
  // 打包后的原生窗口里，前端首屏不再依赖 Tauri API 注入时机去“猜”窗口标签。
  // About 窗口会在 Rust 创建时通过 initialization_script 显式写入 window kind。
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
