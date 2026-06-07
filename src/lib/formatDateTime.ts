const MONTHS_FR = [
  "janvier",
  "février",
  "mars",
  "avril",
  "mai",
  "juin",
  "juillet",
  "août",
  "septembre",
  "octobre",
  "novembre",
  "décembre",
] as const;

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

/** Parse ISO, date seule, datetime-local, heure, timestamp numérique. */
export function parseToDate(value: unknown): Date | null {
  if (value == null || value === "") return null;
  if (value instanceof Date && !Number.isNaN(value.getTime())) return value;
  if (typeof value === "number" && Number.isFinite(value)) {
    const d = new Date(value);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  const raw = String(value).trim();
  if (!raw) return null;
  if (/^\d{4}-\d{2}-\d{2}$/.test(raw)) {
    const d = new Date(`${raw}T00:00:00`);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  if (/^\d{2}:\d{2}(:\d{2})?$/.test(raw)) {
    const d = new Date(`1970-01-01T${raw.length === 5 ? `${raw}:00` : raw}`);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** Format affichage : JJ/mois/AAAA HH:MM:SS (ex. 07/juin/2026 14:30:45). */
export function formatDateTimeFr(value: unknown): string {
  const d = parseToDate(value);
  if (!d) return value == null || value === "" ? "—" : String(value);
  const jj = pad2(d.getDate());
  const mmmm = MONTHS_FR[d.getMonth()] ?? "";
  const aaaa = d.getFullYear();
  const hh = pad2(d.getHours());
  const mm = pad2(d.getMinutes());
  const ss = pad2(d.getSeconds());
  return `${jj}/${mmmm}/${aaaa} ${hh}:${mm}:${ss}`;
}

/** Date seule au même style (heure 00:00:00). */
export function formatDateFr(value: unknown): string {
  const d = parseToDate(value);
  if (!d) return value == null || value === "" ? "—" : String(value);
  return `${pad2(d.getDate())}/${MONTHS_FR[d.getMonth()]}/${d.getFullYear()} 00:00:00`;
}

/** Heure seule HH:MM:SS. */
export function formatTimeFr(value: unknown): string {
  const d = parseToDate(value);
  if (!d) return value == null || value === "" ? "—" : String(value);
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
}
