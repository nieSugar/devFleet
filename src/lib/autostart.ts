interface AutostartState {
  supported: boolean;
  enabled: boolean;
}

async function isTauriDesktopRuntime() {
  if (typeof window === "undefined") return false;

  const { isTauri } = await import("@tauri-apps/api/core");
  return isTauri();
}

export async function getAutostartState(): Promise<AutostartState> {
  // 浏览器预览或纯前端调试时不加载桌面插件，避免 Windows / macOS / Linux
  // 之外的环境误触系统自启动逻辑。
  if (!(await isTauriDesktopRuntime())) {
    return { supported: false, enabled: false };
  }

  const { isEnabled } = await import("@tauri-apps/plugin-autostart");
  return {
    supported: true,
    enabled: await isEnabled(),
  };
}

export async function setAutostartEnabled(enabled: boolean) {
  if (!(await isTauriDesktopRuntime())) return;

  const { enable, disable } = await import("@tauri-apps/plugin-autostart");

  if (enabled) {
    await enable();
    return;
  }

  await disable();
}
