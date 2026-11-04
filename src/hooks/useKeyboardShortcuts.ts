import { useEffect, useRef } from "react";

interface ShortcutHandlers {
  onAddProject: () => void;
  onRefresh: () => void;
  onFocusSearch: () => void;
}

export function useKeyboardShortcuts(handlers: ShortcutHandlers) {
  const ref = useRef(handlers);
  useEffect(() => {
    ref.current = handlers;
  });

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod || e.altKey || e.shiftKey || e.isComposing || e.keyCode === 229 || e.defaultPrevented || e.repeat) return;
      const target = e.target instanceof HTMLElement ? e.target : null;
      if (target?.closest('[role="dialog"], [aria-modal="true"]')) return;
      const editing = target?.isContentEditable || target?.closest('input, textarea, select, [role="combobox"]');
      if (editing && !(e.key.toLowerCase() === "k" && target?.closest(".header-search"))) return;

      switch (e.key.toLowerCase()) {
        case "k":
          e.preventDefault();
          ref.current.onFocusSearch();
          break;
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
