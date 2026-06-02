/** Identifiants des panneaux Paramètres (état replié / déplié). */
export const PARAMETRES_PANEL_IDS = [
  "assistant",
  "compte",
  "theme",
  "impression",
  "entites",
  "roles",
  "utilisateurs",
] as const;

export type ParametresPanelId = (typeof PARAMETRES_PANEL_IDS)[number];

const STORAGE_KEY = "blin:parametres-panels-open";

export type ParametresPanelsState = Partial<Record<ParametresPanelId, boolean>>;

const DEFAULT_OPEN: ParametresPanelsState = {
  assistant: true,
  compte: false,
  theme: false,
  impression: false,
  entites: false,
  roles: false,
  utilisateurs: false,
};

export function loadParametresPanelsState(): ParametresPanelsState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_OPEN };
    const parsed = JSON.parse(raw) as ParametresPanelsState;
    return { ...DEFAULT_OPEN, ...parsed };
  } catch {
    return { ...DEFAULT_OPEN };
  }
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
  return visibleIds.every((id) => state[id] !== false);
}

export function allPanelsClosed(
  state: ParametresPanelsState,
  visibleIds: ParametresPanelId[],
): boolean {
  return visibleIds.every((id) => state[id] !== true);
}
