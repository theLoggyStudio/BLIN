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

/** Format affichage date : JJ mois AAAA (ex. 14 juin 2026). */
export function formatDatePartsFr(day: number, month: number, year: number): string {
  const mmmm = MONTHS_FR[month - 1];
  if (!mmmm) return `${pad2(day)} ? ${year}`;
  return `${pad2(day)} ${mmmm} ${year}`;
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

/** Date compacte compteur (jjmmaaaa, ex. 14062026) → JJ/mois/AAAA. */
export function formatJjmmaaaaFr(value: unknown): string {
  if (value == null || value === "") return "—";
  const raw = String(value).trim();
  if (MONTHS_FR.some((m) => raw.includes(`/${m}/`) || raw.includes(` ${m} `))) return raw;
  if (!/^\d{8}$/.test(raw)) return raw;
  const day = Number(raw.slice(0, 2));
  const month = Number(raw.slice(2, 4));
  const year = Number(raw.slice(4, 8));
  if (!Number.isFinite(day) || !Number.isFinite(month) || !Number.isFinite(year)) return raw;
  return formatDatePartsFr(day, month, year);
}

/** Format affichage : JJ mois AAAA HH:MM:SS (ex. 14 juin 2026 14:30:45). */
export function formatDateTimeFr(value: unknown): string {
  const d = parseToDate(value);
  if (!d) return value == null || value === "" ? "—" : String(value);
  const datePart = formatDatePartsFr(d.getDate(), d.getMonth() + 1, d.getFullYear());
  return `${datePart} ${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
}

/** Date seule : JJ mois AAAA (ex. 14 juin 2026). */
export function formatDateFr(value: unknown): string {
  const d = parseToDate(value);
  if (!d) {
    if (value == null || value === "") return "—";
    return formatJjmmaaaaFr(value);
  }
  return formatDatePartsFr(d.getDate(), d.getMonth() + 1, d.getFullYear());
}

/** Heure seule HH:MM:SS. */
export function formatTimeFr(value: unknown): string {
  const d = parseToDate(value);
  if (!d) return value == null || value === "" ? "—" : String(value);
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
}
