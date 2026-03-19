import { useState, useEffect, useCallback } from "react";
import { tauriAPI } from "../lib/tauri";

interface EditorStatus {
  vscode: boolean;
  cursor: boolean;
  webstorm: boolean;
}

export function useEditors() {
  const [editors, setEditors] = useState<EditorStatus | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const result = await tauriAPI.detectEditors();
        if (result.success && result.data) {
          setEditors(result.data);
        }
      } catch (e) {
        console.warn("检测编辑器失败:", e);
      }
    })();
  }, []);

  const openInEditor = useCallback(
    async (editor: string, projectPath: string) => {
      return tauriAPI.openInEditor({ editor, projectPath });
    },
    []
  );

  return { editors, openInEditor };
}
