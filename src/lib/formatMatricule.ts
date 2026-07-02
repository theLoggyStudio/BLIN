/** Date du jour au format jjmmaaaa. */
export function todayJjmmaaaa(): string {
  const n = new Date();
  const dd = String(n.getDate()).padStart(2, "0");
  const mm = String(n.getMonth() + 1).padStart(2, "0");
  const yyyy = String(n.getFullYear());
  return `${dd}${mm}${yyyy}`;
}

/** Aperçu local : base + date du jour + n° (sans appel backend). */
export function matriculeLocalPreview(base: string, numero = 1): string {
  return formatMatriculeDisplay(base, todayJjmmaaaa(), numero);
}

/** Format affiché : `<base><date jjmmaaaa><compteur>` — ex. MAT1203202601. */
export function formatMatriculeDisplay(
  base: unknown,
  date: unknown,
  numero: unknown,
): string {
  const b = String(base ?? "").trim();
  const d = String(date ?? "").trim();
  const n =
    typeof numero === "number"
      ? numero
      : parseInt(String(numero ?? "").trim(), 10);
  if (!b && !d && (Number.isNaN(n) || n === 0)) return "";
  if (!b) return `${d}${Number.isNaN(n) ? "" : String(n).padStart(2, "0")}`;
  return `${b}${d}${Number.isNaN(n) ? "" : String(n).padStart(2, "0")}`;
}

/** Extrait du preview backend les clés liées à un champ matricule. */
export function matriculePreviewPatchForField(
  preview: Record<string, unknown>,
  fieldKey: string,
): Record<string, unknown> {
  const patch: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(preview)) {
    if (v === null || v === undefined || v === "") continue;
    if (k === fieldKey || k.startsWith(`${fieldKey}_`)) {
      patch[k] = v;
    }
  }
  return patch;
}

/** Lit les colonnes stockées `{key}_base`, `_jjmmaaaa`, `_numero` ou la valeur `{key}` déjà hydratée. */
export function matriculeDisplayFromRow(row: Record<string, unknown>, fieldKey: string): string {
  const hydrated = row[fieldKey];
  if (hydrated != null && String(hydrated).trim()) {
    return String(hydrated).trim();
  }
  return formatMatriculeDisplay(
    row[`${fieldKey}_base`],
    row[`${fieldKey}_jjmmaaaa`],
    row[`${fieldKey}_numero`],
  );
}
