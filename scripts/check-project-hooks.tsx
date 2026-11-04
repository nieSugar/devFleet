// Run pnpm dev, then open /scripts/check-project-hooks.html. No test dependencies required.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { useNodeProcesses, type NodeProcessesState } from "../src/hooks/useNodeProcesses";
import { tauriAPI } from "../src/lib/tauri";

const output = document.getElementById("result")!;
const root = createRoot(document.getElementById("fixture")!);
const originalScan = tauriAPI.listNodeProcesses;
const originalInterval = window.setInterval;
const originalClearInterval = window.clearInterval;
const originalVisibility = Object.getOwnPropertyDescriptor(document, "visibilityState");
const intervals = new Map<number, () => void>();
const pending: ((result: Awaited<ReturnType<typeof originalScan>>) => void)[] = [];
let latest: NodeProcessesState;
let visible = true;
let nextTimer = 1;
const checks: string[] = [];

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
Object.defineProperty(document, "visibilityState", { configurable: true, get: () => visible ? "visible" : "hidden" });
window.setInterval = ((handler: () => void, delay?: number) => {
  if (delay !== 5000) return originalInterval(handler, delay);
  const id = nextTimer++;
  intervals.set(id, handler);
  return id;
}) as typeof window.setInterval;
window.clearInterval = ((id: number) => {
  if (!intervals.delete(id)) originalClearInterval(id);
}) as typeof window.clearInterval;
tauriAPI.listNodeProcesses = () => new Promise(resolve => pending.push(resolve));

function Harness({ enabled }: { enabled: boolean }) {
  latest = useNodeProcesses(enabled);
  return null;
}

function check(condition: boolean, message: string) {
  if (!condition) throw new Error(message);
  checks.push(message);
}

const resolveScan = (index: number, success = true, portsError?: string) => pending[index](success
  ? { success: true, data: { processes: [{ pid: index + 1, name: "node", ports: [] }], portsError } }
  : { success: false, error: "fixture scan failed" });

try {
  await act(async () => { root.render(<Harness enabled />); });
  check(pending.length === 1 && intervals.size === 1, `one consumer creates one scan and one timer (scans=${pending.length}, timers=${intervals.size}, status=${latest!.status})`);
  await act(async () => { intervals.forEach(tick => tick()); intervals.forEach(tick => tick()); });
  check(pending.length === 1, "slow IPC requests do not overlap");
  await act(async () => { resolveScan(0); });
  check(latest!.status === "ready" && latest!.processes[0]?.pid === 1 && latest!.portsError === null, "successful response without ports is not a scan failure");
  await act(async () => { intervals.forEach(tick => tick()); });
  await act(async () => { visible = false; document.dispatchEvent(new Event("visibilitychange")); });
  check(intervals.size === 0, "hidden page stops polling timer");
  await act(async () => { resolveScan(1, true, "stale port failure"); });
  check(latest!.processes[0]?.pid === 1 && latest!.portsError === null, "response and warning from an old visibility generation are ignored");
  await act(async () => { visible = true; document.dispatchEvent(new Event("visibilitychange")); });
  check(pending.length === 3 && intervals.size === 1, "returning to the page restarts exactly one scan");
  await act(async () => { resolveScan(2, false); });
  check(latest!.status === "error" && latest!.processes.length === 0, "failed scan clears stale process data");
  await act(async () => { void latest!.refresh(); });
  await act(async () => { resolveScan(3); });
  check(latest!.status === "ready" && latest!.processes[0]?.pid === 4, "retry recovers after failure");
  await act(async () => { void latest!.refresh(); });
  await act(async () => { resolveScan(4, true, "fixture port scan failed"); });
  check(latest!.status === "ready" && latest!.processes[0]?.pid === 5 && latest!.error === null && latest!.portsError === "fixture port scan failed", "port scan failure preserves processes and exposes a separate warning");
  await act(async () => { void latest!.refresh(); });
  await act(async () => { resolveScan(5); });
  check(latest!.portsError === null && latest!.processes[0]?.pid === 6, "successful retry clears the port warning");
  await act(async () => { void latest!.refresh(); });
  await act(async () => { resolveScan(6, true, "fixture port scan failed"); });
  await act(async () => { void latest!.refresh(); });
  await act(async () => { resolveScan(7, false); });
  check(latest!.status === "error" && latest!.processes.length === 0 && latest!.portsError === null, "process scan failure clears the obsolete port warning");
  await act(async () => { root.render(<Harness enabled={false} />); });
  await act(async () => { await latest!.refresh(); });
  check(intervals.size === 0 && pending.length === 8, "no consumers means no polling or manual scan");
  await act(async () => { root.unmount(); });
  check(intervals.size === 0, "unmount removes polling timer");
  output.dataset.status = "passed";
  output.textContent = `PASS (${checks.length})\n${checks.join("\n")}`;
} catch (error) {
  output.dataset.status = "failed";
  output.textContent = `FAIL\n${checks.join("\n")}\n${String(error)}`;
  await act(async () => { root.unmount(); });
} finally {
  tauriAPI.listNodeProcesses = originalScan;
  window.setInterval = originalInterval;
  window.clearInterval = originalClearInterval;
  if (originalVisibility) Object.defineProperty(document, "visibilityState", originalVisibility);
  else Reflect.deleteProperty(document, "visibilityState");
}
