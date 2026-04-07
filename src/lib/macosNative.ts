type ThemeMode = "light" | "dark";
type Unlisten = () => void;

const MACOS_ADD_PROJECT_EVENT = "macos://add-project";
const MACOS_OPEN_SETTINGS_EVENT = "macos://open-settings";

function isMacOSUserAgent() {
  return typeof window !== "undefined" && /mac/i.test(window.navigator.userAgent);
}

async function isMacOSTauriRuntime() {
  // 先做平台短路，避免 Windows / Linux 进入任何 macOS 原生能力分支。
  if (!isMacOSUserAgent()) return false;

  const { isTauri } = await import("@tauri-apps/api/core");
  return isTauri();
}

export async function syncMacOSNativeTheme(theme: ThemeMode) {
  // 原生标题栏主题只对 macOS 有意义，其他平台直接跳过。
  if (!(await isMacOSTauriRuntime())) return;

  const { setTheme } = await import("@tauri-apps/api/app");
  await setTheme(theme);
}

export async function syncMacOSAppLanguage(language: string) {
  // macOS 顶部原生菜单需要单独同步，Windows / Linux 不调用这条链路。
  if (!(await isMacOSTauriRuntime())) return;

  const { tauriAPI } = await import("./tauri");
  await tauriAPI.syncAppLanguage(language);
}

export async function listenForMacOSAddProject(
  handler: () => void | Promise<void>,
): Promise<Unlisten> {
  // 原生菜单的“添加项目”只会在 macOS 上发事件，其他平台直接返回空清理函数。
  if (!(await isMacOSTauriRuntime())) return () => {};

  const { listen } = await import("@tauri-apps/api/event");
  return listen(MACOS_ADD_PROJECT_EVENT, () => {
    void handler();
  });
}

export async function listenForMacOSOpenSettings(
  handler: () => void | Promise<void>,
): Promise<Unlisten> {
  // 原生菜单的“设置”只会通知主窗口导航，Windows / Linux 完全跳过。
  if (!(await isMacOSTauriRuntime())) return () => {};

  const { listen } = await import("@tauri-apps/api/event");
  return listen(MACOS_OPEN_SETTINGS_EVENT, () => {
    void handler();
  });
}
