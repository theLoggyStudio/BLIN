import { DateTime } from "luxon";
import { formatDateFr } from "@/lib/formatDateTime";
import type { ScreenRow } from "@/types/screen";

const FIRED_STORAGE_KEY = "loggmagic-task-reminders-fired";
const REMINDER_TYPES = new Set(["generale", ""]);
const ACTIVE_STATUSES = new Set(["a_faire", "en_cours", ""]);

export interface TaskReminderCandidate {
  taskId: string;
  task: ScreenRow;
  dueAt: DateTime;
  fireKey: string;
}

function readFiredKeys(): Set<string> {
  try {
    const raw = sessionStorage.getItem(FIRED_STORAGE_KEY);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as string[];
    return new Set(Array.isArray(parsed) ? parsed : []);
  } catch {
    return new Set();
  }
}

export function markReminderFired(fireKey: string): void {
  const keys = readFiredKeys();
  keys.add(fireKey);
  try {
    sessionStorage.setItem(FIRED_STORAGE_KEY, JSON.stringify([...keys]));
  } catch {
    /* quota */
  }
}

export function clearReminderFired(fireKey: string): void {
  const keys = readFiredKeys();
  if (!keys.delete(fireKey)) return;
  try {
    sessionStorage.setItem(FIRED_STORAGE_KEY, JSON.stringify([...keys]));
  } catch {
    /* quota */
  }
}

/** Réarme les rappels après modification ou suppression d'une tâche. */
export function clearTaskReminderKeys(taskId: string): void {
  const keys = readFiredKeys();
  const prefix = `${taskId}:`;
  let changed = false;
  for (const key of [...keys]) {
    if (key.startsWith(prefix)) {
      keys.delete(key);
      changed = true;
    }
  }
  if (!changed) return;
  try {
    sessionStorage.setItem(FIRED_STORAGE_KEY, JSON.stringify([...keys]));
  } catch {
    /* quota */
  }
}

function taskTitle(task: ScreenRow): string {
  return String(task.intitule ?? task.libelle ?? "Tâche").trim() || "Tâche";
}

function parseTimeParts(raw: unknown): { hour: number; minute: number } | null {
  const text = String(raw ?? "").trim();
  if (!text) return null;
  const match = text.match(/^(\d{1,2}):(\d{2})(?::\d{2})?$/);
  if (!match) return null;
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (hour < 0 || hour > 23 || minute < 0 || minute > 59) return null;
  return { hour, minute };
}

function parseDateIso(raw: unknown): string | null {
  const text = String(raw ?? "").trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) return null;
  const dt = DateTime.fromISO(text, { zone: "local" });
  return dt.isValid ? text : null;
}

/** Tâches système (validation, signature, déstockage) exclues des rappels horaires. */
export function isReminderEligibleTask(task: ScreenRow): boolean {
  const type = String(task.type_tache ?? "generale").trim().toLowerCase();
  if (!REMINDER_TYPES.has(type)) return false;

  const statut = String(task.statut ?? "a_faire").trim().toLowerCase();
  if (!ACTIVE_STATUSES.has(statut)) return false;

  const entityLink = String(
    task.entite_a_signer ?? task.entite_a_valider ?? "",
  ).trim();
  const recordId = String(task.enregistrement_id ?? "").trim();
  if (entityLink && recordId) return false;

  return parseTimeParts(task.heure_debut) != null;
}

export function buildTaskReminderMessage(task: ScreenRow): string {
  const title = taskTitle(task);
  const desc = String(task.description ?? "").trim();
  const dateRaw = parseDateIso(task.date_echeance);
  const timeParts = parseTimeParts(task.heure_debut);
  const timeLabel = timeParts
    ? `${String(timeParts.hour).padStart(2, "0")}:${String(timeParts.minute).padStart(2, "0")}`
    : String(task.heure_debut ?? "").trim();
  const dateLabel = dateRaw ? formatDateFr(dateRaw) : "aujourd'hui";
  const priorite = String(task.priorite ?? "").trim();

  let msg = `Rappel de tâche : « ${title} » prévue le ${dateLabel} à ${timeLabel}.`;
  if (desc) msg += ` Description : ${desc}.`;
  if (priorite) msg += ` Priorité : ${priorite}.`;
  return msg;
}

function reminderFireKey(taskId: string, now: DateTime, hasDate: boolean): string {
  if (hasDate) return `${taskId}:once`;
  return `${taskId}:${now.toFormat("yyyy-MM-dd")}`;
}

/** Prochaine échéance (locale) pour une tâche. */
export function getNextTaskDueAt(task: ScreenRow, now: DateTime): DateTime | null {
  if (!isReminderEligibleTask(task)) return null;

  const timeParts = parseTimeParts(task.heure_debut);
  if (!timeParts) return null;

  const dateIso = parseDateIso(task.date_echeance);
  if (dateIso) {
    const due = DateTime.fromISO(`${dateIso}T00:00:00`, { zone: "local" }).set({
      hour: timeParts.hour,
      minute: timeParts.minute,
      second: 0,
      millisecond: 0,
    });
    return due.isValid ? due : null;
  }

  let due = now.set({
    hour: timeParts.hour,
    minute: timeParts.minute,
    second: 0,
    millisecond: 0,
  });
  if (due <= now) {
    due = due.plus({ days: 1 });
  }
  return due;
}

/** Tâches dont l'heure est atteinte (fenêtre de 90 s pour ne pas rater la minute). */
export function findDueTaskReminders(
  tasks: ScreenRow[],
  now: DateTime,
  firedKeys: Set<string>,
): TaskReminderCandidate[] {
  const due: TaskReminderCandidate[] = [];

  for (const task of tasks) {
    if (!isReminderEligibleTask(task)) continue;

    const taskId = String(task.id ?? "").trim();
    if (!taskId) continue;

    const timeParts = parseTimeParts(task.heure_debut);
    if (!timeParts) continue;

    const dateIso = parseDateIso(task.date_echeance);
    const fireKey = reminderFireKey(taskId, now, Boolean(dateIso));
    if (firedKeys.has(fireKey)) continue;

    const dueAt = dateIso
      ? DateTime.fromISO(`${dateIso}T00:00:00`, { zone: "local" }).set({
          hour: timeParts.hour,
          minute: timeParts.minute,
          second: 0,
          millisecond: 0,
        })
      : now.startOf("day").set({
          hour: timeParts.hour,
          minute: timeParts.minute,
          second: 0,
          millisecond: 0,
        });

    if (!dueAt.isValid) continue;

    const deltaMs = now.toMillis() - dueAt.toMillis();
    if (deltaMs < 0) continue;
    if (!now.hasSame(dueAt, "day")) continue;

    due.push({ taskId, task, dueAt, fireKey });
  }

  return due;
}

/** Délai (ms) avant la prochaine vérification — 1 s près de l'échéance, sinon adaptatif. */
export function msUntilNextReminderCheck(
  tasks: ScreenRow[],
  now: DateTime,
): number {
  let nearestMs = 30_000;

  for (const task of tasks) {
    if (!isReminderEligibleTask(task)) continue;
    const next = getNextTaskDueAt(task, now);
    if (!next) continue;
    const diff = next.diff(now).toMillis();
    if (diff <= 0) return 1_000;
    if (diff < nearestMs) nearestMs = diff;
  }

  if (nearestMs <= 120_000) return 1_000;
  if (nearestMs <= 600_000) return 5_000;
  return 30_000;
}

export function loadFiredReminderKeys(): Set<string> {
  return readFiredKeys();
}
