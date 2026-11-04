// Open /scripts/check-settings.html for a safe preview, ?check=1 for checks, or ?theme=dark.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { App, ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import jaJP from "antd/locale/ja_JP";
import { isTauri } from "@tauri-apps/api/core";
import { createInstance } from "i18next";
import { I18nextProvider, initReactI18next, useTranslation } from "react-i18next";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import SettingsWindow from "../src/components/SettingsWindow";
import { ThemeProvider, useTheme } from "../src/contexts/ThemeContext";
import { tauriAPI } from "../src/lib/tauri";
import type { EditorStatus, UpsertCustomEditorInput } from "../src/types/project";
import { getDevFleetAntdThemeConfig } from "../src/theme/antdTheme";
import zhMessages from "../src/i18n/locales/zh-CN.json";
import enMessages from "../src/i18n/locales/en-US.json";
import jaMessages from "../src/i18n/locales/ja-JP.json";
import "antd/dist/reset.css";
import "../src/index.css";
import "../src/App.css";

if (isTauri()) throw new Error("Run this fixture in a browser, never in the desktop runtime.");
const params = new URLSearchParams(location.search);
const runChecks = params.get("check") === "1";
const output = document.getElementById("result")!;
const fixture = document.getElementById("fixture")!;
const root = createRoot(fixture);
const originalAPI = { ...tauriAPI };
const originalTheme = localStorage.getItem("devfleet-theme");
const checks: string[] = [];
let defaultEditorId: string | null = "code";
let shellEnabled = false;
let failShellSave = false;
let shellWrites = 0;
let editorReads = 0;
const editors: EditorStatus = {
  code: { name: "Visual Studio Code", installed: true, source: "auto", path: "C:\\Program Files\\Microsoft VS Code\\Code.exe", args: [], canEditArgs: false },
  cursor: { name: "Cursor", installed: true, source: "auto", path: "C:\\Users\\Developer\\AppData\\Local\\Programs\\Cursor\\Cursor.exe", args: [], canEditArgs: false },
  webstorm: { name: "WebStorm", installed: true, source: "auto", path: "C:\\Program Files\\JetBrains\\WebStorm\\bin\\webstorm64.exe", args: [], canEditArgs: false },
  "custom:legacy": { name: "Sublime Text", installed: false, source: "custom", path: "D:\\Tools\\Sublime Text\\sublime_text.exe", args: [], canEditArgs: true },
};

// Block every native API first so newly added settings actions cannot escape the fixture.
Object.assign(tauriAPI, Object.fromEntries(Object.keys(tauriAPI).map(name => [name, async () => ({ success: false, error: `Fixture does not implement ${name}` })])), {
  detectEditors: async () => { editorReads++; return { success: true, data: { ...editors } }; },
  getDefaultEditor: async () => ({ success: true, data: { editorId: defaultEditorId } }),
  setDefaultEditor: async (editorId: string | null) => { defaultEditorId = editorId; return { success: true, data: { editorId } }; },
  getShellContextMenuState: async () => ({ success: true, data: { supported: true, enabled: shellEnabled, mode: "managed" } }),
  setShellContextMenuEnabled: async (enabled: boolean) => {
    shellWrites++;
    if (failShellSave) return { success: false, error: "fixture shell setting save failed" };
    shellEnabled = enabled;
    return { success: true, data: { supported: true, enabled, mode: "managed" } };
  },
  selectEditor: async () => "C:\\Fixture\\Editor.exe",
  selectFolder: async () => null,
  upsertCustomEditor: async (request: UpsertCustomEditorInput) => {
    const id = request.id || "custom:fixture";
    editors[id] = { name: request.name || "Fixture Editor", installed: true, source: "custom", path: request.path || editors[id]?.path, args: request.args || [], canEditArgs: true };
    return { success: true, data: { id } };
  },
  removeCustomEditor: async (id: string) => { delete editors[id]; return { success: true, data: { id } }; },
  discoverEditorCandidates: async () => ({ success: true, data: { candidates: [], warnings: [], truncated: false } }),
  importEditorCandidate: async () => ({ success: false, error: "No import candidates in this fixture" }),
});
localStorage.setItem("devfleet-theme", params.get("theme") === "dark" ? "dark" : "light");
function restoreTheme() {
  if (originalTheme === null) localStorage.removeItem("devfleet-theme");
  else localStorage.setItem("devfleet-theme", originalTheme);
}
window.addEventListener("pagehide", restoreTheme, { once: true });
const i18n = createInstance();
await i18n.use(initReactI18next).init({
  resources: { "zh-CN": { translation: zhMessages }, "en-US": { translation: enMessages }, "ja-JP": { translation: jaMessages } },
  lng: "zh-CN", fallbackLng: "zh-CN", interpolation: { escapeValue: false },
});
const router = createMemoryRouter([
  { path: "/settings", element: <SettingsWindow /> },
  { path: "/", element: <div id="fixture-home">已返回项目页（预览）</div> },
], { initialEntries: ["/settings"] });

// eslint-disable-next-line react-refresh/only-export-components -- standalone browser fixture
function Preview() {
  const { isDark } = useTheme();
  const { i18n: language } = useTranslation();
  const locale = language.language === "en-US" ? enUS : language.language === "ja-JP" ? jaJP : zhCN;
  return <ConfigProvider theme={getDevFleetAntdThemeConfig(isDark)} locale={locale}><App><main className="app-main"><RouterProvider router={router} /></main></App></ConfigProvider>;
}
function check(condition: boolean, message: string) {
  if (!condition) throw new Error(message);
  checks.push(message);
}
async function click(element: Element | null) {
  if (!element) throw new Error("Missing click target");
  await act(async () => { (element as HTMLElement).click(); });
}
function panel(id: string) { return document.getElementById(`settings-panel-${id}`)!; }
function button(text: string) { return Array.from(document.querySelectorAll("button")).find(item => item.textContent?.trim() === text) || null; }
async function waitFor(condition: () => boolean, label: string) {
  const deadline = performance.now() + 3000;
  while (!condition()) {
    if (performance.now() >= deadline) throw new Error(`Timed out waiting for ${label}`);
    await act(async () => { await new Promise(resolve => window.setTimeout(resolve, 25)); });
  }
}
function visibleModal() {
  return Array.from(document.querySelectorAll<HTMLElement>(".ant-modal-wrap")).find(item => getComputedStyle(item).display !== "none" && item.getClientRects().length > 0);
}

const tree = <ThemeProvider><I18nextProvider i18n={i18n}><Preview /></I18nextProvider></ThemeProvider>;
if (!runChecks) {
  root.render(tree);
} else {
  fixture.style.height = `${Math.min(window.innerHeight, 560)}px`;
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  try {
    await act(async () => { root.render(tree); });
    await waitFor(() => document.querySelector('input[name="default-editor"]:disabled') === null, "settings load");
    check(document.querySelectorAll(".settings-navigation button").length === 3, "settings has three navigation categories");
    check(!panel("editors").hidden && panel("appearance").hidden && panel("system").hidden, "editors is the initial panel and the others are hidden");
    check(Boolean(document.getElementById("settings-nav-editors")?.getAttribute("aria-current")), "the current category is announced accessibly");
    check(document.querySelectorAll('input[name="default-editor"]').length === 4, "default editor cards show three installed editors plus the choose-each-time option");
    const content = document.querySelector<HTMLElement>(".settings-content")!;
    const fixedElements = Array.from(document.querySelectorAll<HTMLElement>(".settings-header, .settings-back-button, .settings-navigation"));
    const fixedRects = fixedElements.map(element => element.getBoundingClientRect());
    const initialContentTop = panel("editors").getBoundingClientRect().top;
    await act(async () => { content.scrollTop = content.scrollHeight; });
    check(content.scrollTop > 0 && panel("editors").getBoundingClientRect().top < initialContentTop - 1, "settings content actually scrolls within the constrained app viewport");
    check(fixedElements.every((element, index) => {
      const rect = element.getBoundingClientRect();
      return Math.abs(rect.top - fixedRects[index].top) < 1 && Math.abs(rect.left - fixedRects[index].left) < 1;
    }), "scrolling keeps the header, return button and category navigation fixed");
    const lastEditorRect = document.querySelector(".editor-settings-item:last-child")!.getBoundingClientRect();
    const contentRect = content.getBoundingClientRect();
    check(lastEditorRect.bottom <= contentRect.bottom + 1 && lastEditorRect.top >= contentRect.top - 1, "the last editor entry is reachable at the bottom of the content scroller");
    check(Array.from(document.querySelectorAll<HTMLElement>("#fixture, #fixture > .ant-app, .app-main, .settings-screen, .settings-shell, .settings-layout")).every(element => element.scrollHeight <= element.clientHeight + 1) && document.documentElement.scrollHeight <= window.innerHeight + 1, "settings outer containers do not create a second vertical scrollbar");
    content.scrollTop = 0;
    await click(document.getElementById("settings-nav-appearance"));
    check(!panel("appearance").hidden && panel("editors").hidden, "appearance navigation reveals only its panel");
    await click(document.querySelector(`[aria-label="${i18n.t("settings.darkTitle")}"]`));
    check(document.documentElement.dataset.theme === "dark", "dark theme updates the real theme provider");
    await click(document.querySelector(`[aria-label="${i18n.t("settings.lightTitle")}"]`));
    check(document.documentElement.dataset.theme === "light", "light theme updates the real theme provider");
    await click(document.querySelector('[aria-label="English"]'));
    check(i18n.language === "en-US" && document.querySelector(".settings-title")?.textContent === enMessages.settings.title, "language choice updates the real settings page");
    await click(document.querySelector('[aria-label="简体中文"]'));
    check(i18n.language === "zh-CN", "language can be switched back to Chinese");
    await click(document.getElementById("settings-nav-system"));
    check(!panel("system").hidden && panel("appearance").hidden, "system navigation reveals its panel");
    const startup = document.querySelector<HTMLButtonElement>(`[role="switch"][aria-label="${i18n.t("settings.startupToggleTitle")}"]`)!;
    check(startup.disabled, "browser preview cannot change native autostart settings");
    const shell = document.querySelector<HTMLButtonElement>(`[role="switch"][aria-label="${i18n.t("settings.shellMenuToggleTitle")}"]`)!;
    failShellSave = true;
    await click(shell);
    check(shellWrites === 1 && shell.getAttribute("aria-checked") === "false" && !shell.disabled, "failed system switch save rolls back and unlocks the control");
    failShellSave = false;
    await click(shell);
    check(shellWrites === 2 && shell.getAttribute("aria-checked") === "true", "successful system switch save keeps the new value");
    await click(document.getElementById("settings-nav-editors"));
    check(editorReads === 1, "switching categories keeps editor state mounted without rescanning");
    await click(button(i18n.t("settings.editors.add")));
    await waitFor(() => Boolean(visibleModal()), "editor modal");
    await act(async () => { visibleModal()!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", keyCode: 27, bubbles: true, cancelable: true })); });
    await waitFor(() => !visibleModal(), "editor modal to close");
    check(router.state.location.pathname === "/settings", "Escape closes the editor modal without leaving settings");
    await click(document.querySelector(".settings-back-button"));
    check(router.state.location.pathname === "/" && document.querySelector("#fixture-home") !== null, "return button navigates back to the project page");
    await act(async () => { await router.navigate("/settings"); });
    await act(async () => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });
    check(router.state.location.pathname === "/", "Escape leaves settings when no modal is open");
    output.dataset.status = "passed";
    output.textContent = `PASS (${checks.length})\n${checks.join("\n")}`;
  } catch (error) {
    output.dataset.status = "failed";
    output.textContent = `FAIL\n${checks.join("\n")}\n${String(error)}`;
  } finally {
    await act(async () => { root.unmount(); });
    fixture.style.height = "";
    output.hidden = false;
    Object.assign(tauriAPI, originalAPI);
    restoreTheme();
  }
}
