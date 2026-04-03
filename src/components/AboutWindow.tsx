import React, { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import appIcon from "../assets/app-icon.png";
import "./AboutWindow.css";

type AboutInfo = {
  name: string;
  version: string;
};

// 后续如果需要替换 About 页面文案，优先修改这里即可。
const ABOUT_COPY = {
  titleName: "软件名称占位",
  versionLabel: "版本号占位",
  buildInfo: "内部版本号 #BUILD-NUMBER，占位日期 构建",
  licenseLine1: "授权给 占位姓名 / 团队名称",
  licenseLine2: "订阅有效期至 占位日期。",
  licenseLine3: "这里可以放一句授权说明或使用说明占位。",
  runtimeLine1: "运行时版本：占位版本号 / 架构 / WebView 版本",
  runtimeLine2: "VM / Runtime / Engine 信息占位",
  supportPrefix: "由",
  supportLinkText: "开源软件",
  supportSuffix: "提供支持",
  copyrightText: "版权所有 © 占位年份 占位作者 / 公司名称",
};

const FALLBACK_INFO: AboutInfo = {
  name: "devFleet",
  version: "2.1.6",
};

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
  const [info] = useState<AboutInfo>(FALLBACK_INFO);
  const [copied, setCopied] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void appWindow.close();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [appWindow]);

  // 复制按钮会把占位信息整体带走，方便后续校对或直接粘贴给他人确认。
  const summary = [
    `${ABOUT_COPY.titleName} ${ABOUT_COPY.versionLabel}`,
    ABOUT_COPY.buildInfo,
    ABOUT_COPY.licenseLine1,
    ABOUT_COPY.licenseLine2,
    ABOUT_COPY.licenseLine3,
    ABOUT_COPY.runtimeLine1,
    ABOUT_COPY.runtimeLine2,
    `${ABOUT_COPY.supportPrefix}${ABOUT_COPY.supportLinkText}${ABOUT_COPY.supportSuffix}`,
    ABOUT_COPY.copyrightText,
    `当前应用名: ${info.name}`,
    `当前版本号: ${info.version}`,
  ].join("\n");

  const handleCopyAndClose = async () => {
    try {
      await copyText(summary);
      setCopied(true);
      window.setTimeout(() => {
        void appWindow.close();
      }, 220);
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
            <div className="about-brand-stack">
              <div className="about-brand-gradient about-brand-gradient-a" />
              <div className="about-brand-gradient about-brand-gradient-b" />
              <img className="about-brand-icon" src={appIcon} alt={info.name} draggable={false} />
            </div>
          </aside>

          <main className="about-content">
            <header className="about-header">
              <h1 className="about-heading">
                {ABOUT_COPY.titleName} {ABOUT_COPY.versionLabel}
              </h1>
              <p className="about-build">{ABOUT_COPY.buildInfo}</p>
            </header>

            <section className="about-copy-block">
              <p>{ABOUT_COPY.licenseLine1}</p>
              <p>{ABOUT_COPY.licenseLine2}</p>
              <p>{ABOUT_COPY.licenseLine3}</p>
            </section>

            <section className="about-copy-block">
              <p>{ABOUT_COPY.runtimeLine1}</p>
              <p>{ABOUT_COPY.runtimeLine2}</p>
            </section>

            <section className="about-copy-block about-copy-block-compact">
              <p>
                {ABOUT_COPY.supportPrefix}
                <span className="about-link-placeholder">{ABOUT_COPY.supportLinkText}</span>
                {ABOUT_COPY.supportSuffix}
              </p>
              <p>
                <span>{ABOUT_COPY.copyrightText}</span>
              </p>
            </section>
          </main>
        </div>

        <footer className="about-footer">
          <button className="about-action about-action-secondary" onClick={() => appWindow.close()}>
            关闭(C)
          </button>
          <button className="about-action about-action-primary" onClick={handleCopyAndClose}>
            {copied ? "已复制并关闭" : "复制并关闭"}
          </button>
        </footer>
      </div>
    </section>
  );
};

export default AboutWindow;
