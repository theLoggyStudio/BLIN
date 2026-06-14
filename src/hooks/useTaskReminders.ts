import { useCallback, useEffect, useRef } from "react";
import { DateTime } from "luxon";
import { useAlert } from "@/contexts/AlertContext";
import {
  FOCUS_TACHES_WINDOW_EVENT,
  TASK_REMINDERS_REFRESH_EVENT,
} from "@/constants/events";
import { useAuth } from "@/hooks/useAuth";
import {
  buildTaskReminderMessage,
  findDueTaskReminders,
  loadFiredReminderKeys,
  markReminderFired,
  msUntilNextReminderCheck,
} from "@/lib/taskReminders";
import { fetchDdaListPage } from "@/lib/ddaList";
import { REMINDER_TASKS_PAGE_SIZE } from "@/constants/variable.constant";

const TACHE_ENTITY_KEY = "tache";

/**
 * Surveille les tâches générales et affiche un rappel persistant (Loggy)
 * à l'heure indiquée — vérification chaque seconde près de l'échéance.
 */
export function useTaskReminders() {
  const { user } = useAuth();
  const { showTaskReminder } = useAlert();
  const timerRef = useRef<number | null>(null);
  const runningRef = useRef(false);

  const checkReminders = useCallback(async () => {
    if (runningRef.current) return;
    runningRef.current = true;
    try {
      const data = await fetchDdaListPage(TACHE_ENTITY_KEY, {
        page: 0,
        pageSize: REMINDER_TASKS_PAGE_SIZE,
      });
      const now = DateTime.local();
      const fired = loadFiredReminderKeys();
      const due = findDueTaskReminders(data.rows, now, fired);

      for (const item of due) {
        markReminderFired(item.fireKey);
        const message = buildTaskReminderMessage(item.task);
        showTaskReminder(message, {
          taskId: item.taskId,
          onOpenTaches: () => {
            window.dispatchEvent(new CustomEvent(FOCUS_TACHES_WINDOW_EVENT));
          },
        });
      }

      const delay = msUntilNextReminderCheck(data.rows, now);
      if (timerRef.current != null) window.clearTimeout(timerRef.current);
      timerRef.current = window.setTimeout(() => {
        void checkReminders();
      }, delay);
    } catch {
      if (timerRef.current != null) window.clearTimeout(timerRef.current);
      timerRef.current = window.setTimeout(() => {
        void checkReminders();
      }, 30_000);
    } finally {
      runningRef.current = false;
    }
  }, [showTaskReminder]);

  useEffect(() => {
    if (!user) return;

    void checkReminders();

    const onVisible = () => {
      if (document.visibilityState === "visible") void checkReminders();
    };
    const onRefresh = () => void checkReminders();

    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener(TASK_REMINDERS_REFRESH_EVENT, onRefresh);

    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener(TASK_REMINDERS_REFRESH_EVENT, onRefresh);
      if (timerRef.current != null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [user, checkReminders]);
}
