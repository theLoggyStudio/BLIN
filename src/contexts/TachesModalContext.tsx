import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
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

  const openTaches = useCallback((draft?: ScreenRow) => {
    setInitialCreate(draft);
    setOpen(true);
  }, []);

  const closeTaches = useCallback(() => {
    setOpen(false);
    setInitialCreate(undefined);
  }, []);

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
