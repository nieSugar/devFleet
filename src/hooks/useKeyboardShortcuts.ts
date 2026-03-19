import { useEffect } from "react";

interface ShortcutHandlers {
  onAddProject: () => void;
  onRefresh: () => void;
}

export function useKeyboardShortcuts(handlers: ShortcutHandlers) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod) return;

      switch (e.key.toLowerCase()) {
        case "n":
          e.preventDefault();
          handlers.onAddProject();
          break;
        case "r":
          e.preventDefault();
          handlers.onRefresh();
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handlers]);
}
