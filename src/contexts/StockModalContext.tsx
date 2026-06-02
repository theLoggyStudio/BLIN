import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { StockModal } from "@/items/StockModal";

interface StockModalContextValue {
  openStock: () => void;
  closeStock: () => void;
}

const StockModalContext = createContext<StockModalContextValue | null>(null);

export function StockModalProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);

  const openStock = useCallback(() => setOpen(true), []);
  const closeStock = useCallback(() => setOpen(false), []);

  const value = useMemo(
    () => ({ openStock, closeStock }),
    [openStock, closeStock],
  );

  return (
    <StockModalContext.Provider value={value}>
      {children}
      <StockModal open={open} onClose={closeStock} />
    </StockModalContext.Provider>
  );
}

export function useStockModal(): StockModalContextValue {
  const ctx = useContext(StockModalContext);
  if (!ctx) {
    throw new Error("useStockModal doit être utilisé dans StockModalProvider");
  }
  return ctx;
}
