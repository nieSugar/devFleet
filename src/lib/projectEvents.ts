type Unlisten = () => void;

const PROJECTS_CHANGED_EVENT = "projects://changed";

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
