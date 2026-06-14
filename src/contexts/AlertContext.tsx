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
import { useAuth } from "@/contexts/AuthContext";
import { formatDateTimeFr } from "@/lib/formatDateTime";
import {
  personifyAlertMessage,
  personifyTaskReminderMessage,
} from "@/lib/alertPersonify";

const MAX_ALERTS = 5;
const ALERT_TTL_MS = 8000;

export interface AlertItem {
  id: number;
  message: string;
  variant: AlertVariant;
  createdAt: string;
  entering?: boolean;
  exiting?: boolean;
  persistent?: boolean;
  personify?: boolean;
  loading?: boolean;
  actionLabel?: string;
  onAction?: () => void;
}

export interface TaskReminderAlertOptions {
  taskId?: string;
  onOpenTaches: () => void;
}

interface ShowAlertOptions {
  persistent?: boolean;
  /** false = texte déjà réécrit ou brut voulu */
  personify?: boolean;
  actionLabel?: string;
  onAction?: () => void;
}

interface AlertContextValue {
  showAlert: (message: string, variant?: AlertVariant, options?: ShowAlertOptions) => void;
  showSuccess: (message: string) => void;
  showError: (message: string) => void;
  showWarning: (message: string) => void;
  showInfo: (message: string) => void;
  showTaskReminder: (message: string, options: TaskReminderAlertOptions) => void;
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

  const updateAlert = useCallback((id: number, patch: Partial<AlertItem>) => {
    setAlerts((prev) => prev.map((a) => (a.id === id ? { ...a, ...patch } : a)));
  }, []);

  const scheduleAutoDismiss = useCallback(
    (id: number, persistent?: boolean) => {
      if (persistent || timersRef.current.has(id)) return;
      const timer = window.setTimeout(() => dismiss(id), ALERT_TTL_MS) as unknown as number;
      timersRef.current.set(id, timer);
    },
    [dismiss],
  );

  const mountAlertItem = useCallback((item: AlertItem) => {
    setAlerts((prev) => {
      const next = [...prev, item];
      if (!item.persistent) {
        const ephemeral = next.filter((a) => !a.persistent);
        if (ephemeral.length > MAX_ALERTS) {
          const overflow = ephemeral.length - MAX_ALERTS;
          let removed = 0;
          return next.filter((a) => {
            if (a.persistent || removed >= overflow) return true;
            const t = timersRef.current.get(a.id);
            if (t) {
              clearTimeout(t);
              timersRef.current.delete(a.id);
            }
            removed += 1;
            return false;
          });
        }
      }
      return next;
    });

    window.setTimeout(() => {
      setAlerts((prev) =>
        prev.map((a) => (a.id === item.id ? { ...a, entering: false } : a)),
      );
    }, 320);
  }, []);

  const showAlert = useCallback(
    (message: string, variant: AlertVariant = "info", options?: ShowAlertOptions) => {
      const id = ++idRef.current;
      const needsPersonify = options?.personify !== false;
      const persistent = options?.persistent ?? false;

      mountAlertItem({
        id,
        message,
        variant,
        createdAt: formatDateTimeFr(new Date()),
        entering: true,
        persistent,
        personify: false,
        loading: needsPersonify,
        actionLabel: options?.actionLabel,
        onAction: options?.onAction,
      });

      if (!needsPersonify) {
        scheduleAutoDismiss(id, persistent);
        return;
      }

      void personifyAlertMessage(message, variant)
        .then((displayMessage) => {
          updateAlert(id, { message: displayMessage, loading: false });
          scheduleAutoDismiss(id, persistent);
        })
        .catch(() => {
          updateAlert(id, { loading: false });
          scheduleAutoDismiss(id, persistent);
        });
    },
    [mountAlertItem, scheduleAutoDismiss, updateAlert],
  );

  const showTaskReminder = useCallback(
    (message: string, options: TaskReminderAlertOptions) => {
      const id = ++idRef.current;

      mountAlertItem({
        id,
        message,
        variant: "warning",
        createdAt: formatDateTimeFr(new Date()),
        entering: true,
        persistent: true,
        personify: false,
        loading: true,
        actionLabel: "Ouvrir les tâches",
        onAction: () => {
          dismiss(id);
          options.onOpenTaches();
        },
      });

      void personifyTaskReminderMessage(message)
        .then((displayMessage) => {
          updateAlert(id, { message: displayMessage, loading: false });
        })
        .catch(() => {
          updateAlert(id, { loading: false });
        });
    },
    [dismiss, mountAlertItem, updateAlert],
  );

  const { loginGreeting, loginNotices, clearLoginNotices } = useAuth();

  useEffect(() => {
    registerGlobalAlert(showAlert);
    return () => {
      registerGlobalAlert(() => {});
      for (const t of timersRef.current.values()) clearTimeout(t);
      timersRef.current.clear();
    };
  }, [showAlert]);

  useEffect(() => {
    if (!loginGreeting && loginNotices.length === 0) return;
    if (loginGreeting) {
      showAlert(loginGreeting, "success", { personify: false });
    }
    for (const message of loginNotices) {
      showAlert(message, "warning");
    }
    clearLoginNotices();
  }, [loginGreeting, loginNotices, clearLoginNotices, showAlert]);

  const value: AlertContextValue = {
    showAlert,
    showSuccess: (m) => showAlert(m, "success"),
    showError: (m) => showAlert(m, "danger"),
    showWarning: (m) => showAlert(m, "warning"),
    showInfo: (m) => showAlert(m, "info"),
    showTaskReminder,
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
            personify={alert.personify}
            loading={alert.loading}
            time={alert.createdAt}
            entering={alert.entering}
            exiting={alert.exiting}
            persistent={alert.persistent}
            actionLabel={alert.actionLabel}
            onAction={alert.onAction}
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
