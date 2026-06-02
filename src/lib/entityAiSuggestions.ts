import type { EntityDef } from "@/types/entity";

const SYSTEM_HIDDEN = new Set(["stock", "tache"]);

/**
 * Une entité apparaît dans la barre « Gérer … » seulement si son formulaire
 * contient au moins une liaison `entity` vers une entité avec `ai_suggestions: false`.
 */
export function qualifiesForAiSuggestions(
  ent: EntityDef,
  registry: EntityDef[],
): boolean {
  if (SYSTEM_HIDDEN.has(ent.nom)) return false;
  return ent.attributs.some((attr) => {
    if (attr.type !== "entity") return false;
    const ref = attr.ref?.trim();
    if (!ref) return false;
    const target = registry.find((e) => e.nom === ref);
    return target != null && target.ai_suggestions === false;
  });
}

export function applyAiSuggestionsVisibility(entities: EntityDef[]): EntityDef[] {
  return entities.map((ent) => ({
    ...ent,
    ai_suggestions: qualifiesForAiSuggestions(ent, entities),
  }));
}
