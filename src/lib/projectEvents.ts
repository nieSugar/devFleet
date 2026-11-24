type Unlisten = () => void;

const PROJECTS_CHANGED_EVENT = "projects://changed";
const PROJECT_SCAN_REQUESTED_EVENT = "projects://scan-requested";

export async function listenForProjectsChanged(
  handler: () => void | Promise<void>,
): Promise<Unlisten> {
  const { isTauri } = await import("@tauri-apps/api/core");
  if (!isTauri()) return () => {};

  const { listen } = await import("@tauri-apps/api/event");
  return listen(PROJECTS_CHANGED_EVENT, () => {
    void handler();
  });
}

export async function listenForProjectScanRequests(
  handler: (rootPath: string) => void,
): Promise<Unlisten> {
  const { invoke, isTauri } = await import("@tauri-apps/api/core");
  if (!isTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  let stopped = false;
  let pull = Promise.resolve();

  const receive = () => {
    pull = pull.then(async () => {
      if (stopped) return;
      const rootPath = await invoke<string | null>("take_pending_project_scan");
      if (rootPath && !stopped) handler(rootPath);
    }).catch((error) => console.warn("Failed to receive project scan request", error));
  };

  const unlisten = await listen(PROJECT_SCAN_REQUESTED_EVENT, receive);
  receive();

  return () => {
    stopped = true;
    unlisten();
  };
}
