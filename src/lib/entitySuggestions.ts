import type { EntitySuggestion } from "@/types/entity";

/** Suggestions triées A→Z sur la phrase affichée (français). */
export function sortEntitySuggestionsByPhrase(
  items: EntitySuggestion[],
): EntitySuggestion[] {
  return [...items].sort((a, b) =>
    a.phrase.localeCompare(b.phrase, "fr", { sensitivity: "base" }),
  );
}
