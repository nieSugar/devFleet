import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useTheme } from "../contexts/ThemeContext";
import { getSupportedLanguages } from "../i18n";
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

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        navigate("/");
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigate]);

  return (
    <section className="settings-screen">
      <div className="settings-backdrop" />
      <div className="settings-shell">
        <header className="settings-header">
          <h1 className="settings-title">{t("settings.title")}</h1>
          <p className="settings-subtitle">{t("settings.subtitle")}</p>
        </header>

        <section className="settings-section">
          <div className="settings-section-head">
            <h2 className="settings-section-title">{t("settings.appearanceTitle")}</h2>
            <p className="settings-section-desc">{t("settings.appearanceDesc")}</p>
          </div>

          <div className="settings-options-grid">
            <button
              className="settings-option-card"
              data-active={theme === "light"}
              onClick={() => setTheme("light")}
            >
              <span className="settings-option-title">{t("settings.lightTitle")}</span>
              <span className="settings-option-desc">{t("settings.lightDesc")}</span>
            </button>

            <button
              className="settings-option-card"
              data-active={theme === "dark"}
              onClick={() => setTheme("dark")}
            >
              <span className="settings-option-title">{t("settings.darkTitle")}</span>
              <span className="settings-option-desc">{t("settings.darkDesc")}</span>
            </button>
          </div>
        </section>

        <section className="settings-section">
          <div className="settings-section-head">
            <h2 className="settings-section-title">{t("settings.languageTitle")}</h2>
            <p className="settings-section-desc">{t("settings.languageDesc")}</p>
          </div>

          <div className="settings-language-list">
            {languages.map((language) => {
              const active = i18n.language === language;

              return (
                <button
                  key={language}
                  className="settings-language-item"
                  data-active={active}
                  onClick={() => void i18n.changeLanguage(language)}
                >
                  <span className="settings-language-name">
                    {LANGUAGE_LABELS[language] || language}
                  </span>
                  <span className="settings-language-meta">
                    {active ? t("settings.current") : language}
                  </span>
                </button>
              );
            })}
          </div>
        </section>

        <footer className="settings-footer">
          <button
            className="settings-close-button"
            onClick={() => navigate("/")}
          >
            {t("settings.close")}
          </button>
        </footer>
      </div>
    </section>
  );
};

export default SettingsWindow;
