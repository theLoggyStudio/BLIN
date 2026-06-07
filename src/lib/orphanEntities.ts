/** Entités obsolètes retirées du registre (ex. typo « atricles »). */
export const ORPHAN_ENTITY_KEYS = new Set(["atricles"]);

export function isOrphanEntityKey(key: string): boolean {
  const k = key.trim().toLowerCase();
  return ORPHAN_ENTITY_KEYS.has(k);
}
