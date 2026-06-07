import type { ScreenRow } from "@/types/screen";

const LABEL_KEYS = ["libelle", "nom", "titre", "reference", "intitule"] as const;

function labelFromEmbedValue(value: unknown): string | null {
  let parsed: unknown = value;
  if (typeof value === "string") {
    const t = value.trim();
    if (!t) return null;
    try {
      parsed = JSON.parse(t);
    } catch {
      return null;
    }
  }
  if (Array.isArray(parsed)) {
    for (const item of parsed) {
      if (!item || typeof item !== "object") continue;
      for (const key of LABEL_KEYS) {
        const v = (item as Record<string, unknown>)[key];
        if (v != null && String(v).trim()) return String(v).trim();
      }
    }
    return null;
  }
  if (parsed && typeof parsed === "object") {
    for (const key of LABEL_KEYS) {
      const v = (parsed as Record<string, unknown>)[key];
      if (v != null && String(v).trim()) return String(v).trim();
    }
  }
  return null;
}

/** Libellé lisible pour une ligne DDA (liste, session métier, etc.). */
export function entityRowDisplayLabel(row: ScreenRow): string {
  for (const key of LABEL_KEYS) {
    const v = row[key];
    if (v != null && String(v).trim()) return String(v).trim();
  }
  for (const v of Object.values(row)) {
    const fromEmbed = labelFromEmbedValue(v);
    if (fromEmbed) return fromEmbed;
  }
  const id = String(row.id ?? "").trim();
  return id || "—";
}
