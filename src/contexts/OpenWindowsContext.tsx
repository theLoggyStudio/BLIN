import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  CLOSE_AI_WINDOW_EVENT,
  CLOSE_ENTITY_WINDOW_EVENT,
  CLOSE_STOCK_WINDOW_EVENT,
  CLOSE_TACHES_WINDOW_EVENT,
  FOCUS_AI_WINDOW_EVENT,
  FOCUS_ENTITY_WINDOW_EVENT,
  FOCUS_STOCK_WINDOW_EVENT,
  FOCUS_TACHES_WINDOW_EVENT,
} from "@/constants/events";
import type { AppWindow, AppWindowKind } from "@/types/windows";

interface OpenWindowInput {
  id: string;
  kind: AppWindowKind;
  title: string;
  entityKey?: string;
}

interface OpenWindowsContextValue {
  windows: AppWindow[];
  activeWindowId: string | null;
  openWindow: (input: OpenWindowInput) => void;
  closeWindow: (id: string) => void;
  focusWindow: (id: string) => void;
  setActiveWindowId: (id: string | null) => void;
}

const OpenWindowsContext = createContext<OpenWindowsContextValue | null>(null);

function dispatchFocus(appWindow: AppWindow) {
  switch (appWindow.kind) {
    case "entity":
      if (appWindow.entityKey) {
        globalThis.dispatchEvent(
          new CustomEvent(FOCUS_ENTITY_WINDOW_EVENT, {
            detail: { entityKey: appWindow.entityKey },
          }),
        );
      }
      break;
    case "taches":
      globalThis.dispatchEvent(new CustomEvent(FOCUS_TACHES_WINDOW_EVENT));
      break;
    case "stock":
      globalThis.dispatchEvent(new CustomEvent(FOCUS_STOCK_WINDOW_EVENT));
      break;
    case "ai":
      globalThis.dispatchEvent(new CustomEvent(FOCUS_AI_WINDOW_EVENT));
      break;
  }
}

function dispatchClose(appWindow: AppWindow) {
  switch (appWindow.kind) {
    case "entity":
      globalThis.dispatchEvent(
        new CustomEvent(CLOSE_ENTITY_WINDOW_EVENT, {
          detail: { entityKey: appWindow.entityKey },
        }),
      );
      break;
    case "taches":
      globalThis.dispatchEvent(new CustomEvent(CLOSE_TACHES_WINDOW_EVENT));
      break;
    case "stock":
      globalThis.dispatchEvent(new CustomEvent(CLOSE_STOCK_WINDOW_EVENT));
      break;
    case "ai":
      globalThis.dispatchEvent(new CustomEvent(CLOSE_AI_WINDOW_EVENT));
      break;
  }
}

export function OpenWindowsProvider({ children }: { children: ReactNode }) {
  const [windows, setWindows] = useState<AppWindow[]>([]);
  const [activeWindowId, setActiveWindowId] = useState<string | null>(null);

  const openWindow = useCallback((input: OpenWindowInput) => {
    setWindows((prev) => {
      const existing = prev.find((w) => w.id === input.id);
      if (existing) {
        return prev.map((w) =>
          w.id === input.id ? { ...w, title: input.title, entityKey: input.entityKey } : w,
        );
      }
      return [
        ...prev,
        {
          id: input.id,
          kind: input.kind,
          title: input.title,
          entityKey: input.entityKey,
          openedAt: Date.now(),
        },
      ];
    });
    setActiveWindowId(input.id);
  }, []);

  const closeWindow = useCallback((id: string) => {
    setWindows((prev) => {
      const target = prev.find((w) => w.id === id);
      if (!target) return prev;
      dispatchClose(target);
      return prev.filter((w) => w.id !== id);
    });
    setActiveWindowId((current) => (current === id ? null : current));
  }, []);

  const focusWindow = useCallback(
    (id: string) => {
      const target = windows.find((w) => w.id === id);
      if (!target) return;
      setActiveWindowId(id);
      dispatchFocus(target);
    },
    [windows],
  );

  const value = useMemo(
    () => ({
      windows,
      activeWindowId,
      openWindow,
      closeWindow,
      focusWindow,
      setActiveWindowId,
    }),
    [windows, activeWindowId, openWindow, closeWindow, focusWindow],
  );

  return (
    <OpenWindowsContext.Provider value={value}>{children}</OpenWindowsContext.Provider>
  );
}

export function useOpenWindows(): OpenWindowsContextValue {
  const ctx = useContext(OpenWindowsContext);
  if (!ctx) {
    throw new Error("useOpenWindows doit être utilisé dans OpenWindowsProvider");
  }
  return ctx;
}

export function useOpenWindowsOptional(): OpenWindowsContextValue | null {
  return useContext(OpenWindowsContext);
}
