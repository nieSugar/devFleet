import { useState, useEffect, useCallback } from "react";
import { tauriAPI } from "../lib/tauri";
import { EditorStatus, UpsertCustomEditorInput } from "../types/project";

export function useEditors() {
  const [editors, setEditors] = useState<EditorStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [defaultEditorId, setDefaultEditorIdState] = useState<string | null>(null);
  const [defaultEditorLoading, setDefaultEditorLoading] = useState(true);
  const [defaultEditorError, setDefaultEditorError] = useState<string | null>(null);

  const detect = useCallback(async (force?: boolean) => {
    try {
      setLoading(true);
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
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    tauriAPI
      .detectEditors()
      .then((result) => {
        if (cancelled) return;
        if (result.success && result.data) {
          setEditors(result.data);
        } else {
          setError(result.error || "检测编辑器失败");
        }
      })
      .catch((e) => {
        if (cancelled) return;
        const msg = e instanceof Error ? e.message : "检测编辑器异常";
        setError(msg);
        console.warn("检测编辑器失败:", e);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    tauriAPI
      .getDefaultEditor()
      .then((result) => {
        if (cancelled) return;
        if (result.success && result.data) {
          setDefaultEditorIdState(result.data.editorId);
        } else {
          setDefaultEditorError(result.error || "读取默认编辑器失败");
        }
      })
      .catch((e) => {
        if (cancelled) return;
        setDefaultEditorError(e instanceof Error ? e.message : "读取默认编辑器异常");
      })
      .finally(() => {
        if (!cancelled) setDefaultEditorLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const openInEditor = useCallback(async (editor: string, projectPath: string) => {
    return tauriAPI.openInEditor({ editor, projectPath });
  }, []);

  const refreshEditors = useCallback(() => detect(true), [detect]);

  const setDefaultEditor = useCallback(async (editorId: string | null) => {
    setDefaultEditorLoading(true);
    setDefaultEditorError(null);
    try {
      const result = await tauriAPI.setDefaultEditor(editorId);
      if (result.success && result.data) {
        setDefaultEditorIdState(result.data.editorId);
      } else {
        setDefaultEditorError(result.error || "保存默认编辑器失败");
      }
      return result;
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : "保存默认编辑器异常";
      setDefaultEditorError(errorMessage);
      return { success: false, error: errorMessage };
    } finally {
      setDefaultEditorLoading(false);
    }
  }, []);

  const upsertCustomEditor = useCallback(
    async (request: UpsertCustomEditorInput) => {
      const result = await tauriAPI.upsertCustomEditor(request);
      if (result.success) await detect();
      return result;
    },
    [detect]
  );

  const removeCustomEditor = useCallback(
    async (editorId: string) => {
      const result = await tauriAPI.removeCustomEditor(editorId);
      if (result.success) await detect();
      return result;
    },
    [detect]
  );

  return {
    editors,
    error,
    loading,
    defaultEditorId,
    defaultEditorLoading,
    defaultEditorError,
    openInEditor,
    refreshEditors,
    setDefaultEditor,
    upsertCustomEditor,
    removeCustomEditor,
  };
}
