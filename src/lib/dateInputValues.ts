/** Valeurs pour <input type="date"> (AAAA-MM-JJ). */
export function toDateInputValue(val: unknown): string {
  const s = String(val ?? "").trim();
  if (!s) return "";
  if (/^\d{4}-\d{2}-\d{2}$/.test(s)) return s;
  const head = s.slice(0, 10);
  if (/^\d{4}-\d{2}-\d{2}$/.test(head)) return head;
  return s;
}

/** Valeurs pour <input type="time"> (HH:MM). */
export function toTimeInputValue(val: unknown): string {
  const s = String(val ?? "").trim();
  if (!s) return "";
  if (/^\d{2}:\d{2}$/.test(s)) return s;
  if (/^\d{2}:\d{2}:\d{2}$/.test(s)) return s.slice(0, 5);
  const fromIso = s.match(/T(\d{2}:\d{2})/);
  if (fromIso) return fromIso[1];
  return s;
}

/** Valeurs pour <input type="datetime-local">. */
export function toDatetimeLocalValue(val: unknown): string {
  const s = String(val ?? "").trim();
  if (!s) return "";
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(s)) return s.slice(0, 16);
  if (/^\d{4}-\d{2}-\d{2}$/.test(s)) return `${s}T00:00`;
  return s;
}
