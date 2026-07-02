/** Identifiants des panneaux Paramètres (état replié / déplié). */
export const PARAMETRES_PANEL_IDS = [
  "assistant",
  "personnalisation_ia",
  "compte",
  "theme",
  "impression",
  "entites",
  "archives",
  "imports_exports",
  "roles",
  "utilisateurs",
] as const;

export type ParametresPanelId = (typeof PARAMETRES_PANEL_IDS)[number];

const STORAGE_KEY = "blin:parametres-panels-open";

export type ParametresPanelsState = Partial<Record<ParametresPanelId, boolean>>;

const DEFAULT_OPEN: ParametresPanelsState = {
  assistant: false,
  personnalisation_ia: false,
  compte: false,
  theme: false,
  impression: false,
  entites: false,
  archives: false,
  imports_exports: false,
  roles: false,
  utilisateurs: false,
};

/** Toujours repliés à l'arrivée (ré-auth mot de passe à chaque dépliage). */
export function loadParametresPanelsState(): ParametresPanelsState {
  return { ...DEFAULT_OPEN };
}

export function saveParametresPanelsState(state: ParametresPanelsState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    /* quota / mode privé */
  }
}

export function allPanelsOpen(
  state: ParametresPanelsState,
  visibleIds: ParametresPanelId[],
): boolean {
  return visibleIds.every((id) => state[id] === true);
}

export function allPanelsClosed(
  state: ParametresPanelsState,
  visibleIds: ParametresPanelId[],
): boolean {
  return visibleIds.every((id) => state[id] !== true);
}
