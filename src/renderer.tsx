import "@fontsource/plus-jakarta-sans/400.css";
import "@fontsource/plus-jakarta-sans/500.css";
import "@fontsource/plus-jakarta-sans/600.css";
import "@fontsource/plus-jakarta-sans/700.css";
import "@fontsource/plus-jakarta-sans/800.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";

import { createRoot } from "react-dom/client";
import "./i18n";
import "./index.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Root element not found. Ensure the HTML has an element with id=\"root\".");
}

function hideBootSplash() {
  const splash = document.getElementById("boot-splash");
  if (!splash) return;

  splash.classList.add("is-hidden");
  window.setTimeout(() => {
    splash.remove();
  }, 280);
}

async function bootstrap() {
  if (!container) {
    throw new Error("Root element not found. Ensure the HTML has an element with id=\"root\".");
  }

  const rootContainer = container;

  // 生产包使用预生成的 Ant Design 静态主题样式，避免系统 WebView
  // 在冷启动时漏掉 cssinjs 注入；开发环境继续只走运行时注入，便于调试。
  if (import.meta.env.PROD) {
    await import("./generated/antd-static.css");
  }

  const { default: App } = await import("./App");
  const root = createRoot(rootContainer);
  root.render(<App />);

  window.requestAnimationFrame(() => {
    window.setTimeout(hideBootSplash, 120);
  });
}

void bootstrap();
