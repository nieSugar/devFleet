import { useCallback, useEffect, useRef, useState } from "react";
import { tauriAPI } from "../lib/tauri";
import type { NodeProcessInfo } from "../types/project";

export interface NodeProcessesState {
  processes: NodeProcessInfo[];
  status: "loading" | "ready" | "error";
  loading: boolean;
  error: string | null;
  refresh: (afterCurrent?: boolean) => Promise<void>;
}

// Instantiate once in AppShell; cards and the drawer share the same scan.
export function useNodeProcesses(enabled: boolean): NodeProcessesState {
  const [processes, setProcesses] = useState<NodeProcessInfo[]>([]);
  const [status, setStatus] = useState<NodeProcessesState["status"]>("loading");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const lifecycle = useRef({ active: false, generation: 0 });
  const inFlight = useRef<Promise<void> | null>(null);

  const refresh = useCallback(async function refresh(afterCurrent = false): Promise<void> {
    if (!lifecycle.current.active) return;
    if (inFlight.current) {
      await inFlight.current;
      if (afterCurrent) await refresh();
      return;
    }
    const requestGeneration = lifecycle.current.generation;
    setLoading(true);
    const request = (async () => {
      try {
        const result = await tauriAPI.listNodeProcesses();
        if (!lifecycle.current.active || requestGeneration !== lifecycle.current.generation) return;
        if (!result.success || !result.data) throw new Error(result.error || "Node process scan failed");
        setProcesses(result.data);
        setError(null);
        setStatus("ready");
      } catch (e) {
        if (!lifecycle.current.active || requestGeneration !== lifecycle.current.generation) return;
        setProcesses([]);
        setError(e instanceof Error ? e.message : String(e));
        setStatus("error");
      } finally {
        if (lifecycle.current.active && requestGeneration === lifecycle.current.generation) setLoading(false);
      }
    })();
    inFlight.current = request;
    try {
      await request;
    } finally {
      if (inFlight.current === request) inFlight.current = null;
    }
  }, []);

  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    const currentLifecycle = lifecycle.current;
    const updateVisibility = () => {
      currentLifecycle.generation++;
      currentLifecycle.active = enabled && document.visibilityState !== "hidden";
      clearInterval(timer);
      if (!currentLifecycle.active) return;
      setStatus("loading");
      void refresh(true);
      timer = setInterval(() => void refresh(), 5000);
    };
    updateVisibility();
    document.addEventListener("visibilitychange", updateVisibility);
    return () => {
      currentLifecycle.active = false;
      currentLifecycle.generation++;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", updateVisibility);
    };
  }, [enabled, refresh]);

  return { processes, status, loading, error, refresh };
}
