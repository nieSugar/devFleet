import { useState, useEffect, useCallback } from "react";
import { tauriAPI } from "../lib/tauri";
import { EditorStatus } from "../types/project";

export function useEditors() {
  const [editors, setEditors] = useState<EditorStatus | null>(null);

  const detect = useCallback(async (force?: boolean) => {
    try {
      const result = await tauriAPI.detectEditors(force);
      if (result.success && result.data) {
        setEditors(result.data);
      }
    } catch (e) {
      console.warn("检测编辑器失败:", e);
    }
  }, []);

  useEffect(() => {
    detect();
  }, [detect]);

  const openInEditor = useCallback(
    async (editor: string, projectPath: string) => {
      return tauriAPI.openInEditor({ editor, projectPath });
    },
    [],
  );

  const refreshEditors = useCallback(() => detect(true), [detect]);

  return { editors, openInEditor, refreshEditors };
}
