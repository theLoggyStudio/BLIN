import type { EntityDef } from "@/types/entity";
import { isOrphanEntityKey } from "@/lib/orphanEntities";

const SYSTEM_HIDDEN = new Set(["stock", "tache"]);

/**
 * Une entité apparaît dans la barre « Gérer … » seulement si son formulaire
 * contient au moins une liaison `entity` vers une entité avec `ai_suggestions: false`.
 */
export function qualifiesForAiSuggestions(
  ent: EntityDef,
  registry: EntityDef[],
): boolean {
  if (SYSTEM_HIDDEN.has(ent.nom) || isOrphanEntityKey(ent.nom)) return false;
  const hasEntityLink = ent.attributs.some((a) => a.type === "entity");
  if (!hasEntityLink) return true;
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
    ai_suggestions: ent.is_session ? true : qualifiesForAiSuggestions(ent, entities),
  }));
}
