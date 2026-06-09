import type { EntityDef } from "@/types/entity";

export interface AttributField {
  key: string;
  label: string;
}

export interface AttributTable {
  key: string;
  label: string;
  fields: AttributField[];
}

/** Bloc tableau HTML injecté dans les modèles liste (ex. {{eleves}}, {{stock}}). */
export interface TableBlockAttribut {
  entityKey: string;
  token: string;
  label: string;
}

export interface EntityAttributCatalog {
  tables: AttributTable[];
  /** Attributs système : date, société, etc. */
  systemTables: AttributTable[];
  /** @deprecated Utiliser `systemTables` */
  systemTable: AttributTable;
  /** Attributs bloc tableau pleine largeur pour modèles « Liste ». */
  tableBlocks: TableBlockAttribut[];
}

export interface AttributSuggestion {
  /** Texte inséré après « {{ » (ex. « ecole. » ou « ecole.nom}} »). */
  insert: string;
  label: string;
  detail?: string;
}

const SYSTEM_TABLES: AttributTable[] = [
  {
    key: "date",
    label: "Date (système)",
    fields: [
      { key: "aujourdhui", label: "Date du jour" },
      { key: "heure", label: "Heure" },
    ],
  },
  {
    key: "societe",
    label: "Société (en-tête)",
    fields: [
      { key: "nom", label: "Nom de l'application" },
      { key: "slogan", label: "Slogan" },
    ],
  },
];

const RESERVED_ATTRS = new Set(["id", "uuid", "created_at", "updated_at"]);

/** `eleve` → `eleves`, `cour` → `cours`, `stock` → `stock`. */
export function tableTokenForEntity(entityNom: string): string {
  const n = entityNom.trim();
  if (n === "stock") return "stock";
  if (n.endsWith("s")) return n;
  if (n.endsWith("e")) return `${n}s`;
  if (n.endsWith("ou") || n.endsWith("u")) return `${n}s`;
  return `${n}s`;
}

export function formatTableBlockToken(entityNom: string): string {
  return `{{${tableTokenForEntity(entityNom)}}}`;
}

export function buildAttributCatalog(entities: EntityDef[]): EntityAttributCatalog {
  const tables = entities
    .map((ent) => ({
      key: ent.nom,
      label: ent.label?.trim() || ent.nom,
      fields: ent.attributs
        .filter((a) => !RESERVED_ATTRS.has(a.nom) && a.type !== "entity")
        .map((a) => ({
          key: a.nom,
          label: a.label?.trim() || a.nom,
        })),
    }))
    .sort((a, b) => a.key.localeCompare(b.key, "fr"));
  const tableBlocks = entities
    .map((ent) => ({
      entityKey: ent.nom,
      token: tableTokenForEntity(ent.nom),
      label: ent.label?.trim() || ent.nom,
    }))
    .sort((a, b) => a.label.localeCompare(b.label, "fr"));
  return {
    tables,
    systemTables: SYSTEM_TABLES,
    systemTable: SYSTEM_TABLES[0],
    tableBlocks,
  };
}

export function formatAttributToken(tableKey: string, fieldKey: string): string {
  return `{{${tableKey}.${fieldKey}}}`;
}

/** Détecte une saisie incomplète après « {{ » pour proposer des tables ou champs. */
export function getAttributSuggestions(
  text: string,
  cursor: number,
  catalog: EntityAttributCatalog,
): { replaceStart: number; suggestions: AttributSuggestion[] } | null {
  const before = text.slice(0, cursor);
  const open = before.lastIndexOf("{{");
  if (open < 0) return null;
  const afterOpen = before.slice(open + 2);
  if (afterOpen.includes("}}")) return null;

  const replaceStart = open + 2;

  if (!afterOpen.includes(".")) {
    const prefix = afterOpen.toLowerCase();
    const tableSuggestions: AttributSuggestion[] = [];
    for (const st of catalog.systemTables) {
      if (st.key.toLowerCase().startsWith(prefix)) {
        tableSuggestions.push({
          insert: `${st.key}.`,
          label: st.label,
          detail: st.key,
        });
      }
    }
    for (const t of catalog.tables) {
      if (t.key.toLowerCase().startsWith(prefix)) {
        tableSuggestions.push({
          insert: `${t.key}.`,
          label: t.label,
          detail: t.key,
        });
      }
    }
    return tableSuggestions.length > 0 ? { replaceStart, suggestions: tableSuggestions } : null;
  }

  const dot = afterOpen.indexOf(".");
  const tableKey = afterOpen.slice(0, dot);
  const fieldPrefix = afterOpen.slice(dot + 1).toLowerCase();

  const table =
    catalog.systemTables.find((t) => t.key === tableKey)
    ?? catalog.tables.find((t) => t.key === tableKey);
  if (!table) return null;

  const suggestions = table.fields
    .filter((f) => f.key.toLowerCase().startsWith(fieldPrefix))
    .map((f) => ({
      insert: `${table.key}.${f.key}}}`,
      label: f.label,
      detail: f.key,
    }));

  return suggestions.length > 0 ? { replaceStart, suggestions } : null;
}

export function applyAttributSuggestion(
  text: string,
  cursor: number,
  replaceStart: number,
  insert: string,
): { text: string; cursor: number } {
  const before = text.slice(0, replaceStart);
  const after = text.slice(cursor);
  const newText = `${before}${insert}${after}`;
  const newCursor = replaceStart + insert.length;
  return { text: newText, cursor: newCursor };
}
