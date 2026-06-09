import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  CLOSE_TACHES_WINDOW_EVENT,
  FOCUS_TACHES_WINDOW_EVENT,
} from "@/constants/events";
import { useOpenWindows } from "@/contexts/OpenWindowsContext";
import { TachesModal } from "@/items/TachesModal";
import type { ScreenRow } from "@/types/screen";

interface TachesModalContextValue {
  openTaches: (initialCreate?: ScreenRow) => void;
  closeTaches: () => void;
}

const TachesModalContext = createContext<TachesModalContextValue | null>(null);

export function TachesModalProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [initialCreate, setInitialCreate] = useState<ScreenRow | undefined>();
  const { openWindow, closeWindow } = useOpenWindows();

  const openTaches = useCallback((draft?: ScreenRow) => {
    setInitialCreate(draft);
    setOpen(true);
  }, []);

  const closeTaches = useCallback(() => {
    setOpen(false);
    setInitialCreate(undefined);
  }, []);

  useEffect(() => {
    if (open) {
      openWindow({ id: "taches", kind: "taches", title: "Tâches" });
    } else {
      closeWindow("taches");
    }
  }, [open, openWindow, closeWindow]);

  useEffect(() => {
    const onFocus = () => openTaches();
    const onClose = () => {
      if (open) closeTaches();
    };
    window.addEventListener(FOCUS_TACHES_WINDOW_EVENT, onFocus);
    window.addEventListener(CLOSE_TACHES_WINDOW_EVENT, onClose);
    return () => {
      window.removeEventListener(FOCUS_TACHES_WINDOW_EVENT, onFocus);
      window.removeEventListener(CLOSE_TACHES_WINDOW_EVENT, onClose);
    };
  }, [open, openTaches, closeTaches]);

  const value = useMemo(
    () => ({ openTaches, closeTaches }),
    [openTaches, closeTaches],
  );

  return (
    <TachesModalContext.Provider value={value}>
      {children}
      <TachesModal
        open={open}
        onClose={closeTaches}
        initialCreateValues={initialCreate}
        onInitialCreateApplied={() => setInitialCreate(undefined)}
      />
    </TachesModalContext.Provider>
  );
}

export function useTachesModal(): TachesModalContextValue {
  const ctx = useContext(TachesModalContext);
  if (!ctx) {
    throw new Error("useTachesModal doit être utilisé dans TachesModalProvider");
  }
  return ctx;
}

export function useTachesModalOptional(): TachesModalContextValue | null {
  return useContext(TachesModalContext);
}
