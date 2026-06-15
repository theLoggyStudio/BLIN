import type { ParametresPanelId } from "@/lib/parametresPanels";

/** Accès à la page / menu Paramètres. */
export const PARAMETRES_PAGE_PRIVILEGE = "parametres:voir";

/** Privilège de visibilité par panneau repliable. */
export const PARAMETRES_PANEL_PRIVILEGES: Record<ParametresPanelId, string> = {
  assistant: "parametres:assistant",
  compte: "parametres:compte",
  theme: "parametres:theme",
  impression: "parametres:impression",
  entites: "parametres:entites",
  roles: "parametres:roles",
  utilisateurs: "parametres:utilisateurs",
};

/** Tous les privilèges donnant accès à au moins un panneau ou à la page. */
export const PARAMETRES_VISIBILITY_PRIVILEGES: string[] = [
  PARAMETRES_PAGE_PRIVILEGE,
  ...Object.values(PARAMETRES_PANEL_PRIVILEGES),
  "parametres:entites:creer",
];

export function privilegeForParametresPanel(id: ParametresPanelId): string {
  return PARAMETRES_PANEL_PRIVILEGES[id];
}

export function formatParametresPrivilegeLabel(privilege: string): string {
  const labels: Record<string, string> = {
    "parametres:voir": "Paramètres — accès page",
    "parametres:assistant": "Paramètres — Assistant IA",
    "parametres:compte": "Paramètres — Compte",
    "parametres:theme": "Paramètres — Thème",
    "parametres:impression": "Paramètres — Modèles d'impression",
    "parametres:entites": "Paramètres — Entités métier",
    "parametres:entites:creer": "Paramètres — Créer une entité (Loggy)",
    "parametres:roles": "Paramètres — Rôles",
    "parametres:utilisateurs": "Paramètres — Utilisateurs",
  };
  return labels[privilege] ?? privilege;
}
