import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { AlertBubble, type AlertVariant } from "@/items/Alert";
import { formatDateTimeFr } from "@/lib/formatDateTime";

const MAX_ALERTS = 5;
const ALERT_TTL_MS = 5000;

export interface AlertItem {
  id: number;
  message: string;
  variant: AlertVariant;
  createdAt: string;
  entering?: boolean;
  exiting?: boolean;
}

interface AlertContextValue {
  showAlert: (message: string, variant?: AlertVariant) => void;
  showSuccess: (message: string) => void;
  showError: (message: string) => void;
  showWarning: (message: string) => void;
  showInfo: (message: string) => void;
}

const AlertContext = createContext<AlertContextValue | null>(null);

export function AlertProvider({ children }: { children: ReactNode }) {
  const [alerts, setAlerts] = useState<AlertItem[]>([]);
  const idRef = useRef(0);
  const timersRef = useRef<Map<number, number>>(new Map());

  const dismiss = useCallback((id: number) => {
    const timer = timersRef.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timersRef.current.delete(id);
    }
    setAlerts((prev) =>
      prev.map((a) => (a.id === id ? { ...a, exiting: true, entering: false } : a)),
    );
    window.setTimeout(() => {
      setAlerts((prev) => prev.filter((a) => a.id !== id));
    }, 280);
  }, []);

  const showAlert = useCallback(
    (message: string, variant: AlertVariant = "info") => {
      const id = ++idRef.current;
      const item: AlertItem = {
        id,
        message,
        variant,
        createdAt: formatDateTimeFr(new Date()),
        entering: true,
      };

      setAlerts((prev) => {
        const next = [...prev, item];
        if (next.length > MAX_ALERTS) {
          const oldest = next[0];
          if (oldest) {
            const t = timersRef.current.get(oldest.id);
            if (t) {
              clearTimeout(t);
              timersRef.current.delete(oldest.id);
            }
            return next.slice(1);
          }
        }
        return next;
      });

      window.setTimeout(() => {
        setAlerts((prev) =>
          prev.map((a) => (a.id === id ? { ...a, entering: false } : a)),
        );
      }, 320);

      const timer = window.setTimeout(() => dismiss(id), ALERT_TTL_MS) as unknown as number;
      timersRef.current.set(id, timer);
    },
    [dismiss],
  );

  useEffect(() => {
    registerGlobalAlert(showAlert);
    return () => {
      registerGlobalAlert(() => {});
      for (const t of timersRef.current.values()) clearTimeout(t);
      timersRef.current.clear();
    };
  }, [showAlert]);

  const value: AlertContextValue = {
    showAlert,
    showSuccess: (m) => showAlert(m, "success"),
    showError: (m) => showAlert(m, "danger"),
    showWarning: (m) => showAlert(m, "warning"),
    showInfo: (m) => showAlert(m, "info"),
  };

  return (
    <AlertContext.Provider value={value}>
      {children}
      <div
        className="loggy-alert-stack"
        aria-live="polite"
        aria-label="Messages de Loggy"
      >
        {alerts.map((alert) => (
          <AlertBubble
            key={alert.id}
            message={alert.message}
            variant={alert.variant}
            time={alert.createdAt}
            entering={alert.entering}
            exiting={alert.exiting}
            onClose={() => dismiss(alert.id)}
          />
        ))}
      </div>
    </AlertContext.Provider>
  );
}

export function useAlert(): AlertContextValue {
  const ctx = useContext(AlertContext);
  if (!ctx) {
    throw new Error("useAlert doit être utilisé dans AlertProvider");
  }
  return ctx;
}

/** Pour modules hors React (optionnel — préférer le hook). */
let globalShowAlert: AlertContextValue["showAlert"] | null = null;

export function registerGlobalAlert(fn: AlertContextValue["showAlert"]) {
  globalShowAlert = fn;
}

export function pushLoggyAlert(message: string, variant: AlertVariant = "info") {
  globalShowAlert?.(message, variant);
}
