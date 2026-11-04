import React, { useEffect, useState } from "react";
import { App, Switch } from "antd";
import { ArrowLeftOutlined, CodeOutlined, DesktopOutlined, SkinOutlined } from "@ant-design/icons";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useTheme } from "../contexts/ThemeContext";
import { getSupportedLanguages } from "../i18n";
import { getAutostartState, setAutostartEnabled } from "../lib/autostart";
import { tauriAPI } from "../lib/tauri";
import EditorSettings from "./EditorSettings";
import "./SettingsWindow.css";

const LANGUAGE_LABELS: Record<string, string> = {
  "zh-CN": "简体中文",
  "en-US": "English",
  "ja-JP": "日本語",
};

const SettingsWindow: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const navigate = useNavigate();
  const languages = getSupportedLanguages();
  const { message } = App.useApp();
  const [activeSection, setActiveSection] = useState("editors");
  const [autostartEnabled, setAutostartEnabledState] = useState(false);
  const [autostartSupported, setAutostartSupported] = useState(false);
  const [autostartLoading, setAutostartLoading] = useState(true);
  const [autostartSaving, setAutostartSaving] = useState(false);
  const [shellMenuEnabled, setShellMenuEnabled] = useState(false);
  const [shellMenuSupported, setShellMenuSupported] = useState(false);
  const [shellMenuMode, setShellMenuMode] = useState<
    "managed" | "packaged" | "unsupported"
  >("unsupported");
  const [shellMenuLoading, setShellMenuLoading] = useState(true);
  const [shellMenuSaving, setShellMenuSaving] = useState(false);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      const hasVisibleModal = Array.from(
        document.querySelectorAll<HTMLElement>('.ant-modal-wrap, [aria-modal="true"]'),
      ).some((modal) => modal.getClientRects().length > 0 && window.getComputedStyle(modal).visibility !== "hidden");
      if (!hasVisibleModal) navigate("/");
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigate]);

  useEffect(() => {
    let cancelled = false;

    // 设置页打开时读取系统当前自启动状态，默认保持关闭。
    void getAutostartState()
      .then((state) => {
        if (cancelled) return;

        setAutostartSupported(state.supported);
        setAutostartEnabledState(state.enabled);
      })
      .catch((error) => {
        console.warn("[settings] failed to load autostart state", error);

        if (cancelled) return;

        setAutostartSupported(false);
        setAutostartEnabledState(false);
      })
      .finally(() => {
        if (!cancelled) {
          setAutostartLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;

    void tauriAPI.getShellContextMenuState()
      .then((result) => {
        if (cancelled) return;

        if (result.success && result.data) {
          setShellMenuSupported(result.data.supported);
          setShellMenuEnabled(result.data.enabled);
          setShellMenuMode(
            result.data.mode ??
              (result.data.supported ? "managed" : "unsupported"),
          );
        } else {
          setShellMenuSupported(false);
          setShellMenuEnabled(false);
          setShellMenuMode("unsupported");
        }
      })
      .catch((error) => {
        console.warn("[settings] failed to load shell context menu state", error);

        if (cancelled) return;

        setShellMenuSupported(false);
        setShellMenuEnabled(false);
        setShellMenuMode("unsupported");
      })
      .finally(() => {
        if (!cancelled) {
          setShellMenuLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const handleAutostartChange = async (checked: boolean) => {
    const previous = autostartEnabled;

    setAutostartEnabledState(checked);
    setAutostartSaving(true);

    try {
      await setAutostartEnabled(checked);
      message.success(
        checked
          ? t("settings.startupEnabledMessage")
          : t("settings.startupDisabledMessage"),
      );
    } catch (error) {
      console.warn("[settings] failed to update autostart", error);
      setAutostartEnabledState(previous);
      message.error(t("settings.startupUpdateFailed"));
    } finally {
      setAutostartSaving(false);
    }
  };

  const autostartStatusText = !autostartSupported
    ? t("settings.startupUnsupported")
    : autostartEnabled
      ? t("settings.startupOn")
      : t("settings.startupOff");

  const handleShellMenuChange = async (checked: boolean) => {
    const previous = shellMenuEnabled;

    setShellMenuEnabled(checked);
    setShellMenuSaving(true);

    try {
      const result = await tauriAPI.setShellContextMenuEnabled(checked);
      if (!result.success || !result.data) {
        throw new Error(result.error || "Failed to update shell context menu");
      }

      setShellMenuSupported(result.data.supported);
      setShellMenuEnabled(result.data.enabled);
      setShellMenuMode(
        result.data.mode ??
          (result.data.supported ? "managed" : "unsupported"),
      );
      message.success(
        result.data.enabled
          ? t("settings.shellMenuEnabledMessage")
          : t("settings.shellMenuDisabledMessage"),
      );
    } catch (error) {
      console.warn("[settings] failed to update shell context menu", error);
      setShellMenuEnabled(previous);
      message.error(t("settings.shellMenuUpdateFailed"));
    } finally {
      setShellMenuSaving(false);
    }
  };

  const shellMenuManaged = shellMenuMode === "managed";

  const shellMenuStatusText = !shellMenuSupported
    ? t("settings.shellMenuUnsupported")
    : !shellMenuManaged
      ? t("settings.shellMenuPackaged")
    : shellMenuEnabled
      ? t("settings.shellMenuOn")
      : t("settings.shellMenuOff");

  return (
    <section className="settings-screen">
      <div className="settings-shell">
        <header className="settings-header">
          <div>
            <h1 className="settings-title">{t("settings.title")}</h1>
            <p className="settings-subtitle">{t("settings.subtitle")}</p>
          </div>
          <button className="settings-back-button" type="button" onClick={() => navigate("/")}>
            <ArrowLeftOutlined aria-hidden="true" />
            {t("settings.backToProjects")}
          </button>
        </header>

        <div className="settings-layout">
          <nav className="settings-navigation" aria-label={t("settings.navigation")}>
            {[
              { id: "editors", label: t("settings.editors.title"), icon: <CodeOutlined /> },
              { id: "appearance", label: t("settings.preferencesTitle"), icon: <SkinOutlined /> },
              { id: "system", label: t("settings.systemTitle"), icon: <DesktopOutlined /> },
            ].map(({ id, label, icon }) => (
              <button
                key={id}
                id={`settings-nav-${id}`}
                type="button"
                aria-current={activeSection === id ? "page" : undefined}
                aria-controls={`settings-panel-${id}`}
                onClick={() => setActiveSection(id)}
              >
                <span aria-hidden="true">{icon}</span>
                {label}
              </button>
            ))}
          </nav>

          <div className="settings-content">
            <section id="settings-panel-editors" className="settings-panel" aria-labelledby="settings-nav-editors" hidden={activeSection !== "editors"}>
              <EditorSettings />
            </section>

            <section id="settings-panel-appearance" className="settings-panel" aria-labelledby="settings-nav-appearance" hidden={activeSection !== "appearance"}>
              <div className="settings-panel-heading">
                <h2>{t("settings.preferencesTitle")}</h2>
                <p>{t("settings.preferencesDesc")}</p>
              </div>

              <section className="settings-section">
                <div className="settings-section-head">
                  <h3 className="settings-section-title">{t("settings.appearanceTitle")}</h3>
                  <p className="settings-section-desc">{t("settings.appearanceDesc")}</p>
                </div>

                <div className="settings-options-grid">
                  {(["light", "dark"] as const).map((option) => (
                    <button
                      key={option}
                      type="button"
                      className="settings-option-card"
                      data-active={theme === option}
                      aria-label={t(`settings.${option}Title`)}
                      aria-pressed={theme === option}
                      onClick={() => setTheme(option)}
                    >
                      <span className="settings-theme-preview" data-preview={option} aria-hidden="true">
                        <span className="settings-preview-bar"><i /><i /><i /></span>
                        <span className="settings-preview-body">
                          <span className="settings-preview-nav"><i /><i /><i /></span>
                          <span className="settings-preview-cards"><i /><i /></span>
                        </span>
                      </span>
                      <span className="settings-option-label">
                        <span className="settings-option-title">{t(`settings.${option}Title`)}</span>
                        <span className="settings-choice-indicator" aria-hidden="true" />
                      </span>
                      <span className="settings-option-desc">{t(`settings.${option}Desc`)}</span>
                    </button>
                  ))}
                </div>
              </section>

              <section className="settings-section">
                <div className="settings-section-head">
                  <h3 className="settings-section-title">{t("settings.languageTitle")}</h3>
                  <p className="settings-section-desc">{t("settings.languageDesc")}</p>
                </div>

                <div className="settings-language-list">
                  {languages.map((language) => {
                    const active = i18n.language === language;
                    return (
                      <button
                        key={language}
                        type="button"
                        className="settings-language-item"
                        data-active={active}
                        aria-label={LANGUAGE_LABELS[language] || language}
                        aria-pressed={active}
                        onClick={() => void i18n.changeLanguage(language)}
                      >
                        <span>
                          <span className="settings-language-name">{LANGUAGE_LABELS[language] || language}</span>
                          <span className="settings-language-meta">{active ? t("settings.current") : language}</span>
                        </span>
                        <span className="settings-choice-indicator" aria-hidden="true" />
                      </button>
                    );
                  })}
                </div>
              </section>
            </section>

            <section id="settings-panel-system" className="settings-panel" aria-labelledby="settings-nav-system" hidden={activeSection !== "system"}>
              <div className="settings-panel-heading">
                <h2>{t("settings.systemTitle")}</h2>
                <p>{t("settings.systemDesc")}</p>
              </div>

              <section className="settings-section">
                <div className="settings-section-head">
                  <h3 className="settings-section-title">{t("settings.startupTitle")}</h3>
                  <p className="settings-section-desc">{t("settings.startupDesc")}</p>
                </div>
                <div className="settings-toggle-row">
                  <div className="settings-toggle-copy">
                    <span className="settings-toggle-title">{t("settings.startupToggleTitle")}</span>
                    <span className="settings-toggle-desc" id="settings-startup-status">
                      {autostartLoading ? t("settings.startupChecking") : autostartStatusText}
                    </span>
                  </div>
                  <Switch
                    aria-label={t("settings.startupToggleTitle")}
                    aria-describedby="settings-startup-status"
                    checked={autostartEnabled}
                    disabled={!autostartSupported || autostartLoading || autostartSaving}
                    loading={autostartLoading || autostartSaving}
                    onChange={(checked) => void handleAutostartChange(checked)}
                  />
                </div>
              </section>

              <section className="settings-section">
                <div className="settings-section-head">
                  <h3 className="settings-section-title">{t("settings.shellMenuTitle")}</h3>
                  <p className="settings-section-desc">{t("settings.shellMenuDesc")}</p>
                </div>
                <div className="settings-toggle-row">
                  <div className="settings-toggle-copy">
                    <span className="settings-toggle-title">{t("settings.shellMenuToggleTitle")}</span>
                    <span className="settings-toggle-desc" id="settings-shell-menu-status">
                      {shellMenuLoading ? t("settings.shellMenuChecking") : shellMenuStatusText}
                    </span>
                  </div>
                  <Switch
                    aria-label={t("settings.shellMenuToggleTitle")}
                    aria-describedby="settings-shell-menu-status"
                    checked={shellMenuEnabled}
                    disabled={!shellMenuSupported || !shellMenuManaged || shellMenuLoading || shellMenuSaving}
                    loading={shellMenuLoading || shellMenuSaving}
                    onChange={(checked) => void handleShellMenuChange(checked)}
                  />
                </div>
              </section>
            </section>
          </div>
        </div>
      </div>
    </section>
  );
};

export default SettingsWindow;
