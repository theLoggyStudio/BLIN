import type { EntityAttribute, EntityDef } from "@/types/entity";

export const ENTITY_ATTR_TYPES: { value: string; label: string }[] = [
  { value: "boolean", label: "Booléen" },
  { value: "compteur", label: "Compteur auto" },
  { value: "date", label: "Date" },
  { value: "datetime", label: "Date/heure" },
  { value: "email", label: "E-mail" },
  { value: "entity", label: "Liaison entité" },
  { value: "enum", label: "Liste (enum)" },
  { value: "float", label: "Décimal" },
  { value: "integer", label: "Entier" },
  { value: "matricule", label: "Matricule" },
  { value: "number", label: "Nombre" },
  { value: "photo", label: "Photo" },
  { value: "stock", label: "Stock" },
  { value: "string", label: "Texte" },
  { value: "time", label: "Heure" },
];

/** Libellé affiché dérivé du nom technique (underscores → espaces, 1re lettre majuscule). */
export function labelFromNom(nom: string): string {
  const t = nom.trim().replace(/_/g, " ");
  if (!t) return "";
  return t.charAt(0).toLocaleUpperCase("fr-FR") + t.slice(1);
}

export function normalizeAttributeNom(nom: string): string {
  return nom.trim().toLowerCase().replace(/\s+/g, "_");
}

export function normalizeEntityNom(nom: string): string {
  return normalizeAttributeNom(nom);
}

/** Vérifie l'unicité des noms d'attributs au sein d'une entité. */
export function findDuplicateAttributeNom(attributs: EntityAttribute[]): string | null {
  const seen = new Set<string>();
  for (const a of attributs) {
    const key = normalizeAttributeNom(a.nom);
    if (!key) continue;
    if (seen.has(key)) return key;
    seen.add(key);
  }
  return null;
}

export function applyLabelsFromNoms(entity: EntityDef): EntityDef {
  const nom = normalizeEntityNom(entity.nom);
  return {
    ...entity,
    nom,
    label: labelFromNom(nom),
    attributs: entity.attributs
      .filter((a) => a.nom.trim())
      .map((a) => {
        const attrNom = normalizeAttributeNom(a.nom);
        return {
          ...a,
          nom: attrNom,
          label: labelFromNom(attrNom),
        };
      }),
  };
}

export function emptyEntityDef(): EntityDef {
  return {
    nom: "",
    description: "",
    ai_suggestions: true,
    requires_signature: false,
    signatory_role_ids: [],
    is_session: true,
    attributs: [],
  };
}

export function emptyEntityAttribute(): EntityAttribute {
  return {
    nom: "",
    type: "string",
    required: false,
    relation_multiple: false,
    relation_exclusive_parent: true,
    relation_impact_defer: false,
  };
}

export function normalizeEntityDefForSave(entity: EntityDef): EntityDef {
  const withLabels = applyLabelsFromNoms(entity);
  const nom = withLabels.nom;
  return {
    nom,
    label: withLabels.label,
    description: withLabels.description?.trim() || undefined,
    ai_suggestions: Boolean(withLabels.ai_suggestions),
    requires_signature: Boolean(withLabels.requires_signature),
    signatory_role_ids: withLabels.requires_signature
      ? [...(withLabels.signatory_role_ids ?? [])]
      : [],
    is_session: true,
    attributs: withLabels.attributs.map((a) => ({
      ...a,
      required: Boolean(a.required),
      type:
        String(a.type).startsWith("enum[") || a.type === "enum"
          ? "enum"
          : a.type,
      ref: a.type === "entity" ? (a.ref?.trim().toLowerCase().replace(/\s+/g, "_") ?? undefined) : undefined,
      relation_multiple: a.type === "entity" ? Boolean(a.relation_multiple) : undefined,
      enum_options:
        a.type === "enum" || String(a.type).startsWith("enum")
          ? (a.enum_options ?? []).filter(Boolean)
          : undefined,
    })),
  };
}
