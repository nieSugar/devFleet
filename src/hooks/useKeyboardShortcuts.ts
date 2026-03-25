import { useEffect, useRef } from "react";

interface ShortcutHandlers {
  onAddProject: () => void;
  onRefresh: () => void;
}

export function useKeyboardShortcuts(handlers: ShortcutHandlers) {
  const ref = useRef(handlers);
  useEffect(() => {
    ref.current = handlers;
  });

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod) return;

      switch (e.key.toLowerCase()) {
        case "n":
          e.preventDefault();
          ref.current.onAddProject();
          break;
        case "r":
          e.preventDefault();
          ref.current.onRefresh();
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}
