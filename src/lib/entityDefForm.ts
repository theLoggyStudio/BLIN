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

export function emptyEntityDef(): EntityDef {
  return {
    nom: "",
    label: "",
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
    label: "",
    required: false,
    relation_multiple: false,
    relation_exclusive_parent: true,
    relation_impact_defer: false,
  };
}

export function normalizeEntityDefForSave(entity: EntityDef): EntityDef {
  const nom = entity.nom.trim().toLowerCase().replace(/\s+/g, "_");
  return {
    nom,
    label: entity.label?.trim() || nom,
    description: entity.description?.trim() || undefined,
    ai_suggestions: Boolean(entity.ai_suggestions),
    requires_signature: Boolean(entity.requires_signature),
    signatory_role_ids: entity.requires_signature
      ? [...(entity.signatory_role_ids ?? [])]
      : [],
    is_session: true,
    attributs: entity.attributs
      .filter((a) => a.nom.trim())
      .map((a) => ({
        ...a,
        nom: a.nom.trim().toLowerCase().replace(/\s+/g, "_"),
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
