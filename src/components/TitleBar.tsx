import React, { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { useTheme } from "../contexts/ThemeContext";
import UpdateChecker from "./UpdateChecker";
import "./TitleBar.css";

interface TitleBarProps {
  onOpenNodeManager?: () => void;
}

const IS_MACOS =
  typeof window !== "undefined" &&
  /mac/i.test(window.navigator.userAgent);

const TitleBar: React.FC<TitleBarProps> = ({ onOpenNodeManager }) => {
  const { theme, toggleTheme } = useTheme();
  const { t, i18n } = useTranslation();
  const [maximized, setMaximized] = useState(false);

  const toggleLanguage = () => {
    i18n.changeLanguage(i18n.language === "zh-CN" ? "en-US" : "zh-CN");
  };

  useEffect(() => {
    if (IS_MACOS) return;

    const win = getCurrentWindow();
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let debounceTimer: ReturnType<typeof setTimeout> | undefined;

    win.isMaximized().then((m) => {
      if (!cancelled) setMaximized(m);
    });

    win
      .onResized(() => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(async () => {
          if (cancelled) return;
          const m = await win.isMaximized();
          if (!cancelled) setMaximized(m);
        }, 150);
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });

    return () => {
      cancelled = true;
      clearTimeout(debounceTimer);
      unlisten?.();
    };
  }, []);

  return (
    <header 
      className="titlebar"
      {...(!IS_MACOS ? { "data-tauri-drag-region": true } : {})}
    >
      <div className="titlebar-actions">
        <button
          className="tb-btn node-btn"
          onClick={onOpenNodeManager}
          title={t("titleBar.nodeManager")}
        >
          <NodeHexIcon />
        </button>

        <UpdateChecker />

        <button
          className="tb-btn lang-btn"
          onClick={toggleLanguage}
          title={t("titleBar.language")}
        >
          <LangIcon />
          <span className="lang-label">{i18n.language === "zh-CN" ? "中" : "En"}</span>
        </button>

        <button
          className="tb-btn theme-btn"
          onClick={toggleTheme}
          title={theme === "dark" ? t("titleBar.lightMode") : t("titleBar.darkMode")}
        >
          {theme === "dark" ? <SunIcon /> : <MoonIcon />}
        </button>

        {!IS_MACOS && (
          <div className="tb-win-controls">
            <button
              className="tb-btn win-btn"
              onClick={() => getCurrentWindow().minimize()}
              title={t("titleBar.minimize")}
            >
              <svg width="10" height="1" viewBox="0 0 10 1">
                <rect width="10" height="1" fill="currentColor" />
              </svg>
            </button>

            <button
              className="tb-btn win-btn"
              onClick={() => getCurrentWindow().toggleMaximize()}
              title={maximized ? t("titleBar.restore") : t("titleBar.maximize")}
            >
              {maximized ? (
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 10 10"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1"
                >
                  <rect x="2" y="0.5" width="7.5" height="7" rx="1" />
                  <rect x="0.5" y="2.5" width="7.5" height="7" rx="1" />
                </svg>
              ) : (
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 10 10"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1"
                >
                  <rect x="0.5" y="0.5" width="9" height="9" rx="1.5" />
                </svg>
              )}
            </button>

            <button
              className="tb-btn win-btn close-btn"
              onClick={() => getCurrentWindow().close()}
              title={t("titleBar.close")}
            >
              <svg
                width="10"
                height="10"
                viewBox="0 0 10 10"
                stroke="currentColor"
                strokeWidth="1.2"
                strokeLinecap="round"
              >
                <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" />
              </svg>
            </button>
          </div>
        )}
      </div>
    </header>
  );
};

const SunIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
  >
    <circle cx="8" cy="8" r="3" />
    <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" />
  </svg>
);

const MoonIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinejoin="round"
  >
    <path d="M13.6 9.8A6 6 0 016.2 2.4 6 6 0 1013.6 9.8z" />
  </svg>
);

const LangIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.3"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <circle cx="8" cy="8" r="6.5" />
    <path d="M1.5 8h13" />
    <path d="M8 1.5c-2 2-2.5 4-2.5 6.5s.5 4.5 2.5 6.5" />
    <path d="M8 1.5c2 2 2.5 4 2.5 6.5s-.5 4.5-2.5 6.5" />
  </svg>
);

const NodeHexIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.3"
    strokeLinejoin="round"
    strokeLinecap="round"
  >
    <path d="M8 1L14 4.5V11.5L8 15L2 11.5V4.5L8 1Z" />
    <path d="M8 5V11" />
    <path d="M8 11L5 9.5" />
    <path d="M8 11L11 9.5" />
  </svg>
);

export default TitleBar;
