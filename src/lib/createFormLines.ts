import type { FieldDef, ScreenRow } from "@/types/screen";

export const CREATE_LINES_FORM_KEY = "__create_lines__";
export const LIGNES_COLUMN = "lignes";

export type ListLineRow = ScreenRow & {
  __parentId: string;
  __lineIndex: number;
  __lineCount: number;
  __isFirstLine: boolean;
};

/** Champs embarqués (entités filles) — contenu d'une ligne. */
export function isEmbedFormField(field: FieldDef): boolean {
  return (
    field.type === "entity_embed" ||
    field.type === "entity_embed_list" ||
    Boolean(field.form?.embedParent)
  );
}

export function embedHeaderFields(fields: FieldDef[]): FieldDef[] {
  return fields.filter(
    (f) => f.type === "entity_embed" || f.type === "entity_embed_list",
  );
}

export function scalarFormFields(fields: FieldDef[]): FieldDef[] {
  return fields.filter((f) => !isEmbedFormField(f));
}

/** Champs dont les valeurs varient par ligne (listes embarquées + colonnes filles 1-1). */
export function embedSnapshotFields(allFields: FieldDef[]): FieldDef[] {
  return allFields.filter(
    (f) => f.type === "entity_embed_list" || Boolean(f.form?.embedParent),
  );
}

export function collectEmbedValueKeys(allFields: FieldDef[]): string[] {
  const keys = embedSnapshotFields(allFields).map((f) => f.key);
  for (const f of embedHeaderFields(allFields)) {
    if (f.type === "entity_embed") keys.push(embedRefKey(f.key));
  }
  return keys;
}

/** ID de l'enregistrement choisi (UI) — conservé par ligne dans le JSON `lignes`. */
export function embedRefKey(parentFieldKey: string): string {
  return `__embed_ref__${parentFieldKey}`;
}

export function readFieldValue(values: ScreenRow, field: FieldDef): unknown {
  if (values[field.key] !== undefined) return values[field.key];
  if (field.column && values[field.column] !== undefined) return values[field.column];
  return undefined;
}

export function readEmbedSnapshot(
  values: ScreenRow,
  allFields: FieldDef[],
): Record<string, unknown> {
  const snap: Record<string, unknown> = {};
  for (const f of embedSnapshotFields(allFields)) {
    const v = readFieldValue(values, f);
    if (v !== undefined) snap[f.key] = v;
  }
  for (const f of embedHeaderFields(allFields)) {
    if (f.type === "entity_embed") {
      const ref = values[embedRefKey(f.key)];
      if (ref !== undefined && ref !== null && String(ref).trim()) {
        snap[embedRefKey(f.key)] = ref;
      }
    }
  }
  return snap;
}

/** Harmonise clés colonne → clés formulaire dans un snapshot de ligne. */
export function normalizeLineSnapshot(
  snap: Record<string, unknown>,
  allFields: FieldDef[],
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const f of embedSnapshotFields(allFields)) {
    const v = snap[f.key] ?? (f.column ? snap[f.column] : undefined);
    if (v !== undefined) out[f.key] = v;
  }
  for (const f of embedHeaderFields(allFields)) {
    if (f.type === "entity_embed") {
      const rk = embedRefKey(f.key);
      const ref = snap[rk];
      if (ref !== undefined && ref !== null && String(ref).trim()) out[rk] = ref;
    }
  }
  return out;
}

/** Applique un snapshot de ligne active sur les clés embarquées du formulaire. */
export function embedSnapshotToFormUpdates(
  snap: Record<string, unknown>,
  allFields: FieldDef[],
): Record<string, unknown> {
  const normalized = normalizeLineSnapshot(snap, allFields);
  const updates: Record<string, unknown> = {};
  for (const f of embedSnapshotFields(allFields)) {
    updates[f.key] = normalized[f.key] ?? "";
  }
  for (const f of embedHeaderFields(allFields)) {
    if (f.type === "entity_embed") {
      const rk = embedRefKey(f.key);
      updates[rk] = normalized[rk] ?? "";
    }
  }
  return updates;
}

export function emptyEmbedSnapshot(allFields: FieldDef[]): Record<string, unknown> {
  const snap: Record<string, unknown> = {};
  for (const f of embedSnapshotFields(allFields)) {
    snap[f.key] = "";
  }
  for (const f of embedHeaderFields(allFields)) {
    if (f.type === "entity_embed") snap[embedRefKey(f.key)] = "";
  }
  return snap;
}

export function parseExtraCreateLines(value: unknown): Record<string, unknown>[] {
  if (Array.isArray(value)) {
    return value.filter((v) => v && typeof v === "object") as Record<string, unknown>[];
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) return [];
    try {
      const parsed = JSON.parse(trimmed);
      if (Array.isArray(parsed)) {
        return parsed.filter((v) => v && typeof v === "object") as Record<string, unknown>[];
      }
    } catch {
      return [];
    }
  }
  return [];
}

export function stripCreateLineMeta(data: ScreenRow): ScreenRow {
  const { [CREATE_LINES_FORM_KEY]: _, ...rest } = data;
  return rest;
}

export function pickSharedCreateValues(
  values: ScreenRow,
  allFields: FieldDef[],
): ScreenRow {
  const embedKeySet = new Set(collectEmbedValueKeys(allFields));
  const shared: ScreenRow = {};
  for (const [k, v] of Object.entries(values)) {
    if (k === CREATE_LINES_FORM_KEY || k === LIGNES_COLUMN || embedKeySet.has(k)) continue;
    shared[k] = v;
  }
  return shared;
}

/**
 * Ligne 1 = valeurs à la racine (DB + hydrate).
 * Lignes 2+ = __create_lines__ ou lignes[1..] en JSON.
 */
export function parseParentLignes(
  row: ScreenRow,
  allFields: FieldDef[],
): Record<string, unknown>[] {
  const line1 = normalizeLineSnapshot(readEmbedSnapshot(row, allFields), allFields);
  const fromMeta = parseExtraCreateLines(row[CREATE_LINES_FORM_KEY]).map((s) =>
    normalizeLineSnapshot(s, allFields),
  );
  if (fromMeta.length > 0) {
    return [line1, ...fromMeta];
  }
  const raw = row[LIGNES_COLUMN];
  if (raw != null) {
    const items = parseExtraCreateLines(raw).map((s) => normalizeLineSnapshot(s, allFields));
    if (items.length > 1) {
      return [line1, ...items.slice(1)];
    }
  }
  return [line1];
}

/** Normalise les valeurs après dda_get pour le formulaire d'édition multi-lignes. */
export function normalizeEditFormValues(row: ScreenRow, allFields: FieldDef[]): ScreenRow {
  const out: ScreenRow = { ...row };
  const lines = parseParentLignes(out, allFields);

  for (const f of embedSnapshotFields(allFields)) {
    const fromRoot = readFieldValue(out, f);
    const fromLine = lines[0]?.[f.key];
    const chosen =
      fromRoot !== undefined && fromRoot !== null ? fromRoot : (fromLine ?? fromRoot ?? "");
    out[f.key] = chosen;
  }

  const extras = lines.slice(1);
  out[CREATE_LINES_FORM_KEY] = extras.length > 0 ? JSON.stringify(extras) : "";
  return out;
}

/** Fusionne la ligne active (valeurs courantes du formulaire) dans le tableau de lignes. */
export function mergeActiveLineSnapshots(
  values: ScreenRow,
  allFields: FieldDef[],
  lines: Record<string, unknown>[],
  activeIndex: number,
): Record<string, unknown>[] {
  const snap = normalizeLineSnapshot(readEmbedSnapshot(values, allFields), allFields);
  return lines.map((line, i) =>
    i === activeIndex ? normalizeLineSnapshot({ ...line, ...snap }, allFields) : { ...line },
  );
}

/**
 * Patch à appliquer avant enregistrement : ligne 1 à la racine, lignes 2+ dans __create_lines__.
 * Nécessaire quand l'utilisateur enregistre depuis une ligne autre que la ligne 1.
 */
export function multilineFormPatch(
  values: ScreenRow,
  allFields: FieldDef[],
  lines: Record<string, unknown>[],
  activeIndex: number,
): ScreenRow {
  const merged = mergeActiveLineSnapshots(values, allFields, lines, activeIndex);
  if (merged.length <= 1) {
    return { ...embedSnapshotToFormUpdates(merged[0] ?? {}, allFields), [CREATE_LINES_FORM_KEY]: "" };
  }
  return {
    ...embedSnapshotToFormUpdates(merged[0] ?? {}, allFields),
    [CREATE_LINES_FORM_KEY]: JSON.stringify(merged.slice(1)),
  };
}

/** Colonne liste partagée (même matricule) — rowspan si plusieurs lignes. */
export function isSharedListColumn(field: FieldDef): boolean {
  if (field.type === "entity_embed" || field.type === "entity_embed_list") return false;
  if (field.form?.embedParent) return false;
  return true;
}

export function expandRowsForLignes(
  rows: ScreenRow[],
  pk: string,
  allFields: FieldDef[],
): ListLineRow[] {
  const out: ListLineRow[] = [];
  for (const row of rows) {
    const parentId = String(row[pk] ?? "");
    const lines = parseParentLignes(row, allFields);
    lines.forEach((lineSnap, index) => {
      out.push({
        ...row,
        ...lineSnap,
        __parentId: parentId,
        __lineIndex: index,
        __lineCount: lines.length,
        __isFirstLine: index === 0,
      });
    });
  }
  return out;
}
