// Run pnpm dev, then open /scripts/check-project-workflow.html. IPC is mocked; no user projects are read or written.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { App, ConfigProvider } from "antd";
import enUS from "antd/locale/en_US";
import { createInstance } from "i18next";
import { I18nextProvider, initReactI18next } from "react-i18next";
import ProjectManager from "../src/components/ProjectManager";
import EditorSettings from "../src/components/EditorSettings";
import type { NodeProcessesState } from "../src/hooks/useNodeProcesses";
import { tauriAPI } from "../src/lib/tauri";
import messages from "../src/i18n/locales/en-US.json";
import { DEVFLEET_DARK_ANTD_THEME } from "../src/theme/antdTheme";
import "../src/index.css";

const output = document.getElementById("result")!;
const root = createRoot(document.getElementById("fixture")!);
const originalAPI = { ...tauriAPI };
const checks: string[] = [];
const opened: string[] = [];
const openedEditors: string[] = [];
const defaultEditorWrites: (string | null)[] = [];
const writes: Parameters<typeof tauriAPI.setProjectNodeVersion>[0][] = [];
let scriptRuns = 0;
let folderDialogs = 0;
let retryCount = 0;
let failPreview = false;
let failWrite = false;
let failDefaultEditorSave = false;
let defaultEditorSaveGate: Promise<void> | null = null;
let installedEditorsAvailable = true;
let defaultEditorId: string | null = "code";
const projects = Array.from({ length: 10 }, (_, index) => ({
  id: `fixture-${index}`,
  name: index === 0 ? "Alpha" : index === 1 ? "Beta" : `Project ${index}`,
  path: `C:/fixture/project-${index}`,
  scripts: [{ name: "dev", command: "vite --host 127.0.0.1 --port 5173 --strictPort --mode development" }],
  selectedScript: "dev",
  nodeVersion: "22.0.0",
}));
const snapshot = {
  projects,
  availability: Object.fromEntries(projects.map((p, i) => [p.id, i !== 2])),
  pinnedProjectIds: ["fixture-1"],
  lastUpdated: "2026-10-21",
};
let processState: NodeProcessesState = {
  processes: [{ pid: 42, name: "node", matchedProjectId: "fixture-0", ports: [] }],
  status: "ready",
  loading: false,
  error: null,
  portsError: "fixture port scan failed",
  refresh: async () => { retryCount++; },
};
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
const i18n = createInstance();
await i18n.use(initReactI18next).init({ resources: { en: { translation: messages } }, lng: "en", interpolation: { escapeValue: false } });
Object.assign(tauriAPI, {
  loadProjectConfig: async () => ({ success: true, data: snapshot }),
  refreshProjectConfig: async () => ({ success: true, data: snapshot }),
  detectEditors: async () => ({ success: true, data: {
    code: { installed: installedEditorsAvailable, source: "auto", name: "VS Code", args: [], canEditArgs: false },
    cursor: { installed: installedEditorsAvailable, source: "auto", name: "Cursor", args: [], canEditArgs: false },
  } }),
  getDefaultEditor: async () => ({ success: true, data: { editorId: defaultEditorId } }),
  setDefaultEditor: async (editorId: string | null) => {
    if (defaultEditorSaveGate) await defaultEditorSaveGate;
    if (failDefaultEditorSave) return { success: false, error: "fixture default editor save failed" };
    defaultEditorWrites.push(editorId);
    defaultEditorId = editorId;
    return { success: true, data: { editorId } };
  },
  getNvmInfo: async () => ({ success: true, data: { isInstalled: true, manager: "builtin", availableVersions: [{ version: "22.0.0", fullVersion: "v22.0.0" }, { version: "24.0.0", fullVersion: "v24.0.0" }] } }),
  openInEditor: async ({ editor, projectPath }: { editor: string; projectPath: string }) => { opened.push(projectPath); openedEditors.push(editor); return { success: true }; },
  runScript: async () => { scriptRuns++; return { success: true }; },
  selectFolder: async () => { folderDialogs++; return null; },
  getProjectNodeVersionFile: async () => failPreview ? { success: false, error: "fixture preview failed" } : { success: true, data: { filePath: "C:/fixture/project-0/.nvmrc" } },
  setProjectNodeVersion: async (params: Parameters<typeof tauriAPI.setProjectNodeVersion>[0]) => {
    writes.push(params);
    return failWrite ? { success: false, error: "fixture write failed" } : { success: true, data: { message: "Saved", project: { ...projects[0], nodeVersion: params.nodeVersion || undefined } } };
  },
});

function check(condition: boolean, message: string) {
  if (!condition) throw new Error(message);
  checks.push(message);
}
async function render(key = "normal", settings = false) {
  await act(async () => { root.render(<ConfigProvider locale={enUS} theme={DEVFLEET_DARK_ANTD_THEME}><App><I18nextProvider i18n={i18n}>{settings ? <EditorSettings key={key} /> : <ProjectManager key={key} processState={processState} onOpenNodeManager={() => {}} onViewProcesses={() => {}} />}</I18nextProvider></App></ConfigProvider>); });
}
async function key(target: Element, value: string, options: KeyboardEventInit = {}) {
  await act(async () => { target.dispatchEvent(new KeyboardEvent("keydown", { key: value, bubbles: true, cancelable: true, ...options })); });
}
async function click(element: Element | null) {
  if (!element) throw new Error("Expected click target");
  await act(async () => { (element as HTMLElement).click(); });
}
async function waitFor(condition: () => boolean, description: string) {
  const deadline = performance.now() + 3000;
  while (!condition()) {
    if (performance.now() >= deadline) throw new Error(`Timed out waiting for ${description}`);
    await act(async () => { await new Promise(resolve => window.setTimeout(resolve, 25)); });
  }
}
function search() { return document.querySelector<HTMLInputElement>(".header-search input")!; }
async function enterSearch(value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search(), value);
    search().dispatchEvent(new Event("input", { bubbles: true }));
  });
}
async function selectOption(selector: string, label: string) {
  const input = document.querySelector(`${selector} .ant-select-selector`);
  if (!input) throw new Error(`Missing select ${selector}`);
  await act(async () => { input.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })); });
  await click(Array.from(document.querySelectorAll(".ant-select-dropdown:not(.ant-select-dropdown-hidden) .ant-select-item-option")).find(el => el.textContent?.includes(label)) || null);
}
function visibleModal() {
  return Array.from(document.querySelectorAll<HTMLElement>(".ant-modal-confirm")).reverse().find(modal =>
    modal.getClientRects().length > 0 &&
    getComputedStyle(modal).visibility !== "hidden" &&
    !/\S+-leave(?:\s|$)/.test(modal.className),
  );
}
async function answerModal(label: string) {
  await waitFor(() => Boolean(visibleModal()), "a visible confirmation dialog");
  const modal = visibleModal()!;
  await click(Array.from(modal.querySelectorAll<HTMLButtonElement>(".ant-modal-confirm-btns button")).find(button => !button.disabled && button.textContent === label) || null);
  await waitFor(() => !modal.isConnected, "the answered confirmation dialog to unmount");
  await waitFor(() => !document.querySelector("#project-card-fixture-0 .card-node .ant-select-disabled"), "the Node change handler to settle");
}

try {
  await render();
  check(document.querySelectorAll(".project-card").length === 10, "all projects render with existing pinned ordering");
  await key(document.body, "k", { ctrlKey: true });
  check(document.activeElement === search(), "Ctrl+K focuses the existing search field");
  check(document.querySelector(".is-keyboard-selected")?.id === "project-card-fixture-1", "first visible pinned project is selected");
  await key(search(), "ArrowDown");
  await key(search(), "Enter");
  check(opened[0] === projects[0].path && scriptRuns === 0, "arrows and Enter open the selected default editor without running a script");
  await key(search(), "Enter", { isComposing: true });
  await key(search(), "Enter", { repeat: true });
  check(opened.length === 1, "IME confirmation and held Enter do not launch editors");
  await key(search(), "n", { ctrlKey: true });
  check(folderDialogs === 0, "global add shortcut does not interrupt text entry");
  await enterSearch(" beta ");
  check(document.querySelectorAll(".project-card").length === 1, "trimmed search filters project names");
  await key(search(), "Enter");
  check(opened[1] === projects[1].path, "search change resets selection to a visible result");
  await enterSearch("");
  await click(document.querySelector('[aria-label="' + i18n.t("project.compactView") + '"]'));
  check(Boolean(document.querySelector(".projects-compact")), "compact mode reuses project cards and preserves their actions");
  check(Array.from(document.querySelectorAll<HTMLElement>(".project-card, .add-project-card")).every(el => parseInt(el.style.animationDelay) <= 300), "card entrance delay stays capped with many projects");
  check(document.querySelector(".card-script-command")?.textContent?.includes("vite") === true, "selected script exposes its actual command");
  check(document.querySelector(".card-script-command .ant-typography-copy") !== null, "actual script command has a copy action");
  check(document.querySelector(".card-processes")?.textContent?.includes(i18n.t("project.portsUnavailable")) === true, "port scan failure is visible instead of claiming no ports");
  await click(document.querySelector(".card-processes button"));
  check(retryCount === 1, "port scan failure offers retry");
  await selectOption(".header-filter", i18n.t("project.filterProcesses"));
  check(document.querySelectorAll(".project-card").length === 1 && document.querySelector(".project-card")?.id === "project-card-fixture-0", "process filter keeps only positively matched projects");
  processState = { ...processState, processes: [], status: "error", error: "fixture scan failed" };
  await render();
  check(Boolean(document.querySelector(".project-filter-status")) && !document.querySelector(".empty-title"), "unknown process state is shown as an error, not an empty-result claim");
  await selectOption(".header-filter", i18n.t("project.filterUnavailable"));
  check(document.querySelectorAll(".project-card").length === 1 && document.querySelector(".project-card")?.id === "project-card-fixture-2", "unavailable filter uses the existing path availability state");
  await selectOption(".header-filter", i18n.t("project.filterAll"));
  await selectOption("#project-card-fixture-0 .card-node", "24.0.0");
  await waitFor(() => Boolean(visibleModal()), "the Node write preview dialog");
  check(visibleModal()?.querySelector(".ant-modal-confirm-content")?.textContent?.includes("C:/fixture/project-0/.nvmrc") === true && writes.length === 0, "Node change previews the concrete file before any write");
  check(document.querySelector("#project-card-fixture-0 .card-node .ant-select-disabled") !== null, "Node select is locked during confirmation");
  await answerModal(i18n.t("common.cancel"));
  check(writes.length === 0 && document.querySelector("#project-card-fixture-0 .card-node")?.textContent?.includes("22.0.0") === true, "cancelling leaves the Node value and project files unchanged");
  await selectOption("#project-card-fixture-0 .card-node", "24.0.0");
  await answerModal(i18n.t("common.confirm"));
  await waitFor(() => writes.length === 1, "the confirmed Node version write");
  check(writes.length === 1 && writes[0].expectedFilePath === "C:/fixture/project-0/.nvmrc", "confirmed write sends the exact previewed file path");
  check(document.querySelector("#project-card-fixture-0 .card-node")?.textContent?.includes("24.0.0") === true, "successful write updates the Node value");
  failPreview = true;
  await selectOption("#project-card-fixture-0 .card-node", "22.0.0");
  check(writes.length === 1 && !document.querySelector("#project-card-fixture-0 .card-node .ant-select-disabled"), "failed preview never writes and unlocks the control");
  failPreview = false;
  failWrite = true;
  await selectOption("#project-card-fixture-0 .card-node", "22.0.0");
  await answerModal(i18n.t("common.confirm"));
  await waitFor(() => writes.length === 2, "the rejected Node version write");
  check(writes.length === 2 && document.querySelector("#project-card-fixture-0 .card-node")?.textContent?.includes("24.0.0") === true, "failed write preserves the previous Node value");
  failWrite = false;
  await act(async () => { document.querySelector("#project-card-fixture-0 .card-node .ant-select-clear")!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })); });
  await waitFor(() => Boolean(visibleModal()), "the Node file deletion dialog");
  check(visibleModal()?.querySelector(".ant-modal-confirm-title")?.textContent === i18n.t("project.nodeVersionRemoveTitle") && writes.length === 2, "clearing Node version explicitly confirms file deletion before writing");
  await answerModal(i18n.t("common.confirm"));
  await waitFor(() => writes.length === 3, "the confirmed Node file deletion");
  check(writes[2]?.nodeVersion === null && writes[2].expectedFilePath === "C:/fixture/project-0/.nvmrc", "confirmed clear sends null and the previewed file path");
  defaultEditorId = "missing";
  await render("missing-default");
  await key(document.body, "k", { metaKey: true });
  await key(search(), "Enter");
  check(document.querySelector("#project-card-fixture-1 details[open]") !== null && opened.length === 2, "missing default editor opens the existing editor picker without launching anything");
  await click(document.querySelector("#project-card-fixture-1 .card-header"));
  check(!document.querySelector("details[open]"), "outside click closes a keyboard-opened picker when the saved default is unavailable");

  await render("editor-settings", true);
  check(document.querySelector('[role="radiogroup"]')?.getAttribute("aria-label") === i18n.t("settings.editors.defaultEditor"), "settings exposes the global default editor with an accessible label");
  check(document.querySelector<HTMLInputElement>('input[name="default-editor"][value="missing"]')?.checked === true && document.querySelector<HTMLInputElement>('input[name="default-editor"][value="missing"]')?.disabled === true, "the saved unavailable default remains visible and selected but cannot be chosen again");
  let finishDefaultEditorSave!: () => void;
  defaultEditorSaveGate = new Promise(resolve => { finishDefaultEditorSave = resolve; });
  await click(document.querySelector('input[name="default-editor"][value="cursor"]'));
  check(Array.from(document.querySelectorAll<HTMLInputElement>('input[name="default-editor"]')).every(input => input.disabled), "saving a default editor disables every radio until the request settles");
  await act(async () => { finishDefaultEditorSave(); });
  defaultEditorSaveGate = null;
  check(defaultEditorWrites[0] === "cursor" && document.querySelector<HTMLInputElement>('input[name="default-editor"][value="cursor"]')?.checked === true, "choosing a global default saves and displays the selected editor");
  failDefaultEditorSave = true;
  await click(document.querySelector('input[name="default-editor"][value="code"]'));
  check(defaultEditorWrites.length === 1 && document.querySelector<HTMLInputElement>('input[name="default-editor"][value="cursor"]')?.checked === true && document.querySelector(".editor-default-error")?.textContent?.includes("fixture default editor save failed") === true, "failed default save preserves the previous editor and displays an error");
  failDefaultEditorSave = false;
  await render("saved-default");
  check(document.querySelectorAll('.card-editors > .editor-btn[aria-label="Cursor"]').length === projects.length, "returning from settings applies the saved default to every project card");
  await click(document.querySelector('#project-card-fixture-1 .card-editors > .editor-btn'));
  check(openedEditors[2] === "cursor" && opened[2] === projects[1].path, "the primary editor action uses the saved global default");
  const morePicker = document.querySelector<HTMLDetailsElement>("#project-card-fixture-1 .editor-picker")!;
  await click(morePicker.querySelector("summary"));
  check(morePicker.open, "other editors menu opens when a global default exists");
  await click(morePicker.querySelector(".editor-menu"));
  check(morePicker.open, "clicking inside the editor menu does not dismiss it");
  await click(document.querySelector("#project-card-fixture-1 .card-header"));
  check(!morePicker.open, "outside click dismisses the other editors menu");
  await click(morePicker.querySelector("summary"));
  await key(morePicker.querySelector(".editor-btn")!, "Escape");
  check(!morePicker.open && document.activeElement === morePicker.querySelector("summary"), "Escape dismisses the editor menu and restores focus to its trigger");
  await click(morePicker.querySelector("summary"));
  await click(morePicker.querySelector('.editor-btn[aria-label="VS Code"]'));
  check(!morePicker.open && openedEditors[3] === "code" && opened[3] === projects[1].path, "selecting another editor opens the correct project and dismisses the menu");

  await render("reloaded-editor-settings", true);
  check(document.querySelector<HTMLInputElement>('input[name="default-editor"][value="cursor"]')?.checked === true, "remounting settings reloads the saved global default");
  await click(document.querySelector('input[name="default-editor"][value=""]'));
  check(defaultEditorWrites[1] === null && document.querySelector<HTMLInputElement>('input[name="default-editor"][value=""]')?.checked === true, "clearing the global default saves null and resets the selector");
  await render("cleared-default");
  check(document.querySelectorAll(".editor-choice-btn").length === projects.length, "clearing the global default restores editor selection on every project card");
  const picker = document.querySelector<HTMLDetailsElement>("#project-card-fixture-1 .editor-picker")!;
  await click(picker.querySelector("summary"));
  check(picker.open, "choose editor opens with no global default");
  await click(document.querySelector("#project-card-fixture-0 .card-header"));
  check(!picker.open, "clicking another project dismisses the choose editor menu");
  await click(picker.querySelector("summary"));
  await act(async () => { picker.querySelector<HTMLButtonElement>(".editor-btn")!.focus(); });
  check(picker.open, "moving keyboard focus inside the menu keeps it open");
  await act(async () => { search().focus(); });
  check(!picker.open, "moving keyboard focus outside dismisses the editor menu");
  installedEditorsAvailable = false;
  defaultEditorId = "cursor";
  await render("no-installed-editors", true);
  check(document.querySelectorAll('input[name="default-editor"]').length === 2 && document.querySelector<HTMLInputElement>('input[name="default-editor"][value="cursor"]')?.disabled === true, "with no installed editors only the saved unavailable default and clear option remain");
  await click(document.querySelector('input[name="default-editor"][value=""]'));
  check(defaultEditorWrites[2] === null && document.querySelector<HTMLInputElement>('input[name="default-editor"][value=""]')?.checked === true, "the saved default can still be cleared when no editors are installed");
  output.dataset.status = "passed";
  output.textContent = `PASS (${checks.length})\n${checks.join("\n")}`;
} catch (error) {
  output.dataset.status = "failed";
  output.textContent = `FAIL\n${checks.join("\n")}\n${String(error)}`;
} finally {
  await act(async () => { root.unmount(); });
  Object.assign(tauriAPI, originalAPI);
}
