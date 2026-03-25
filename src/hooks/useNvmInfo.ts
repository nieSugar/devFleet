import { useState, useEffect, useCallback } from "react";
import { NvmInfo } from "../types/project";
import { tauriAPI } from "../lib/tauri";

export function useNvmInfo() {
  const [nvmInfo, setNvmInfo] = useState<NvmInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchNvmInfo = useCallback(async () => {
    try {
      setError(null);
      const result = await tauriAPI.getNvmInfo();
      if (result.success && result.data) {
        setNvmInfo(result.data);
      } else {
        setError(result.error || "获取 NVM 信息失败");
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : "获取 NVM 信息异常";
      setError(msg);
      console.error("获取 NVM 信息失败:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchNvmInfo();
  }, [fetchNvmInfo]);

  const changeNodeVersion = useCallback(
    async (projectId: string, nodeVersion: string | null | undefined) => {
      return tauriAPI.setProjectNodeVersion({
        projectId,
        nodeVersion: nodeVersion ?? null,
      });
    },
    []
  );

  return { nvmInfo, loading, error, changeNodeVersion, refreshNvmInfo: fetchNvmInfo };
}
