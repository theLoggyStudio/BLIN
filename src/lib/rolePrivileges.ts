export const WILDCARD_PRIVILEGE = "*";

/** État coché : `*` implique tous les privilèges du catalogue. */
export function isPrivilegeChecked(selected: Set<string>, priv: string): boolean {
  if (priv === WILDCARD_PRIVILEGE) return selected.has(WILDCARD_PRIVILEGE);
  return selected.has(WILDCARD_PRIVILEGE) || selected.has(priv);
}

export function sortPrivileges(privileges: Iterable<string>): string[] {
  return Array.from(privileges).sort((a, b) => {
    if (a === WILDCARD_PRIVILEGE) return -1;
    if (b === WILDCARD_PRIVILEGE) return 1;
    return a.localeCompare(b, "fr");
  });
}

export function mergePrivilegeCatalog(catalog: string[], selected: Set<string>): string[] {
  return sortPrivileges(new Set([WILDCARD_PRIVILEGE, ...catalog, ...selected]));
}

export function togglePrivilege(
  selected: Set<string>,
  priv: string,
  allPrivileges: string[],
): Set<string> {
  const catalog = allPrivileges.filter((p) => p !== WILDCARD_PRIVILEGE);

  if (priv === WILDCARD_PRIVILEGE) {
    if (selected.has(WILDCARD_PRIVILEGE)) {
      return new Set();
    }
    return new Set([WILDCARD_PRIVILEGE, ...catalog]);
  }

  if (selected.has(WILDCARD_PRIVILEGE)) {
    const next = new Set<string>();
    for (const p of catalog) {
      if (p !== priv) next.add(p);
    }
    return next;
  }

  const next = new Set(selected);
  if (next.has(priv)) next.delete(priv);
  else next.add(priv);
  return next;
}

/** Si tout est coché (ou `*`), n'enregistrer que le joker. */
export function normalizePrivilegesForSave(
  selected: Set<string>,
  catalog: string[],
): string[] {
  if (selected.has(WILDCARD_PRIVILEGE)) {
    return [WILDCARD_PRIVILEGE];
  }
  const nonStar = catalog.filter((p) => p !== WILDCARD_PRIVILEGE);
  if (nonStar.length > 0 && nonStar.every((p) => selected.has(p))) {
    return [WILDCARD_PRIVILEGE];
  }
  return sortPrivileges(
    Array.from(selected).filter((p) => p !== WILDCARD_PRIVILEGE),
  );
}
