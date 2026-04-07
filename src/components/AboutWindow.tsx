import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { isTauri } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import projectLogo from "../../logo.png";
import "./AboutWindow.css";

type AboutInfo = {
  version: string;
};

const FALLBACK_INFO: AboutInfo = {
  version: "2.1.7",
};
const BUILD_DATE = new Date(__APP_BUILD_DATE__);

function formatBuildDate(language: string) {
  return new Intl.DateTimeFormat(language, {
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(BUILD_DATE);
}

async function copyText(text: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();
  document.execCommand("copy");
  textarea.remove();
}

const AboutWindow: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [info, setInfo] = useState<AboutInfo>(FALLBACK_INFO);
  const [copied, setCopied] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    let cancelled = false;

    // About 页面优先读取打包后的真实版本号，浏览器预览则回退到本地默认值。
    void (async () => {
      if (!isTauri()) return;

      try {
        const version = await getVersion();
        if (!cancelled) {
          setInfo({ version });
        }
      } catch {
        // 无法读取原生版本信息时保留前端回退值即可。
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void appWindow.close();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [appWindow]);

  const buildDateLabel = formatBuildDate(i18n.language);
  const versionLine = t("about.versionAndBuildDate", {
    version: info.version,
    date: buildDateLabel,
  });
  const copyrightLine = t("about.copyright", {
    year: BUILD_DATE.getFullYear(),
  });

  // 复制信息保持和界面展示一致，避免菜单、窗口、文档里出现不同版本描述。
  const summary = [
    t("about.productName"),
    versionLine,
    copyrightLine,
  ].join("\n");

  const handleCopy = async () => {
    try {
      await copyText(summary);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  return (
    <section className="about-screen">
      <div className="about-backdrop" />
      <div className="about-content-shell">
        {/* macOS 原生标题栏已经显示“关于 xxx”，这里不再绘制假标题，避免把窗口比例撑大。 */}
        <div className="about-body">
          <aside className="about-brand">
            <img
              className="about-brand-logo"
              src={projectLogo}
              alt={t("about.productName")}
              draggable={false}
            />
          </aside>

          <main className="about-content">
            <header className="about-header">
              <h1 className="about-heading">{t("about.productName")}</h1>
              <p className="about-meta">{versionLine}</p>
              <p className="about-copy-line">{copyrightLine}</p>
            </header>
          </main>
        </div>

        <footer className="about-footer">
          <button className="about-action about-action-secondary" onClick={() => appWindow.close()}>
            {t("about.close")}
          </button>
          <button className="about-action about-action-primary" onClick={handleCopy}>
            {copied ? t("about.copied") : t("about.copyInfo")}
          </button>
        </footer>
      </div>
    </section>
  );
};

export default AboutWindow;
