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
  CLOSE_STOCK_WINDOW_EVENT,
  FOCUS_STOCK_WINDOW_EVENT,
} from "@/constants/events";
import { useOpenWindows } from "@/contexts/OpenWindowsContext";
import { StockModal } from "@/items/StockModal";

interface StockModalContextValue {
  openStock: () => void;
  closeStock: () => void;
}

const StockModalContext = createContext<StockModalContextValue | null>(null);

export function StockModalProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const { openWindow, closeWindow } = useOpenWindows();

  const openStock = useCallback(() => setOpen(true), []);
  const closeStock = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (open) {
      openWindow({ id: "stock", kind: "stock", title: "Stock" });
    } else {
      closeWindow("stock");
    }
  }, [open, openWindow, closeWindow]);

  useEffect(() => {
    const onFocus = () => openStock();
    const onClose = () => {
      if (open) closeStock();
    };
    window.addEventListener(FOCUS_STOCK_WINDOW_EVENT, onFocus);
    window.addEventListener(CLOSE_STOCK_WINDOW_EVENT, onClose);
    return () => {
      window.removeEventListener(FOCUS_STOCK_WINDOW_EVENT, onFocus);
      window.removeEventListener(CLOSE_STOCK_WINDOW_EVENT, onClose);
    };
  }, [open, openStock, closeStock]);

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
