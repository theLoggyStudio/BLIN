import type { EntityDef, EntityRegistry } from "@/types/entity";

const NUMERIC_IMPACT_TYPES = new Set([
  "stock",
  "number",
  "integer",
  "float",
  "compteur",
  "matricule",
]);

export function isNumericImpactType(type: string): boolean {
  return NUMERIC_IMPACT_TYPES.has(type);
}

/** Chemins numériques imbriqués : article.qte_initial, client.age, da.article.prix… */
export function nestedNumericImpactPaths(
  registry: EntityRegistry,
  startEntity: EntityDef | undefined,
  maxDepth = 6,
): { value: string; label: string }[] {
  if (!startEntity) return [];
  const out: { value: string; label: string }[] = [];
  const seen = new Set<string>();

  const walk = (ent: EntityDef, prefix: string, depth: number, chain: string[]) => {
    if (depth > maxDepth) return;
    if (chain.includes(ent.nom)) return;

    for (const a of ent.attributs) {
      const type = String(a.type);
      if (type === "entity") {
        const refKey = a.ref?.trim();
        if (!refKey) continue;
        const child = registry.entities.find((e) => e.nom === refKey);
        if (!child) continue;
        const seg = a.nom.trim();
        const nextPrefix = prefix ? `${prefix}${seg}.` : `${seg}.`;
        walk(child, nextPrefix, depth + 1, [...chain, ent.nom]);
        continue;
      }
      if (!isNumericImpactType(type)) continue;
      const nom = a.nom.trim();
      if (!nom) continue;
      const path = prefix ? `${prefix}${nom}` : nom;
      if (seen.has(path)) continue;
      seen.add(path);
      const attrLabel = a.label?.trim() || nom;
      const entLabel = ent.label?.trim() || ent.nom;
      out.push({
        value: path,
        label: `${path} — ${attrLabel} (${entLabel})`,
      });
    }
  };

  walk(startEntity, "", 0, []);
  return out.sort((a, b) => a.value.localeCompare(b.value, "fr"));
}

export function localNumericImpactPaths(ent: EntityDef | undefined): { value: string; label: string }[] {
  if (!ent) return [];
  return ent.attributs
    .filter((a) => isNumericImpactType(String(a.type)))
    .map((a) => ({
      value: a.nom,
      label: a.label?.trim() ? `${a.label} (${a.nom}) — local` : `${a.nom} — local`,
    }));
}
