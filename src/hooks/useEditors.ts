import { useState, useEffect, useCallback } from "react";
import { tauriAPI } from "../lib/tauri";
import { EditorStatus } from "../types/project";

export function useEditors() {
  const [editors, setEditors] = useState<EditorStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const detect = useCallback(async (force?: boolean) => {
    try {
      setError(null);
      const result = await tauriAPI.detectEditors(force);
      if (result.success && result.data) {
        setEditors(result.data);
      } else {
        setError(result.error || "检测编辑器失败");
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : "检测编辑器异常";
      setError(msg);
      console.warn("检测编辑器失败:", e);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    tauriAPI.detectEditors().then((result) => {
      if (cancelled) return;
      if (result.success && result.data) {
        setEditors(result.data);
      } else {
        setError(result.error || "检测编辑器失败");
      }
    }).catch((e) => {
      if (cancelled) return;
      const msg = e instanceof Error ? e.message : "检测编辑器异常";
      setError(msg);
      console.warn("检测编辑器失败:", e);
    });
    return () => { cancelled = true; };
  }, []);

  const openInEditor = useCallback(
    async (editor: string, projectPath: string) => {
      return tauriAPI.openInEditor({ editor, projectPath });
    },
    [],
  );

  const refreshEditors = useCallback(() => detect(true), [detect]);

  return { editors, error, openInEditor, refreshEditors };
}
