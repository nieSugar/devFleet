import { useState, useEffect, useCallback } from "react";
import { NvmInfo } from "../types/project";
import { tauriAPI } from "../lib/tauri";

export function useNvmInfo() {
  const [nvmInfo, setNvmInfo] = useState<NvmInfo | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const result = await tauriAPI.getNvmInfo();
        if (result.success && result.data) {
          setNvmInfo(result.data);
        }
      } catch (e) {
        console.error("获取 NVM 信息失败:", e);
      }
    })();
  }, []);

  const changeNodeVersion = useCallback(
    async (projectId: string, nodeVersion: string | null | undefined) => {
      return tauriAPI.setProjectNodeVersion({
        projectId,
        nodeVersion: nodeVersion ?? null,
      });
    },
    []
  );

  return { nvmInfo, changeNodeVersion };
}
