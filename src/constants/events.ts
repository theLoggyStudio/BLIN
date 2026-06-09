/** Émis après enregistrement du registre entités (suggestions tableau de bord). */
export const ENTITY_REGISTRY_SYNCED_EVENT = "entity-registry-synced";

/** Session métier active modifiée (filtrage / préremplissage). */
export const BUSINESS_SESSION_CHANGED_EVENT = "business-session-changed";

/** Ouvre le modal d'import CSV sur l'écran entité active. */
export const ENTITY_CSV_IMPORT_OPEN_EVENT = "entity-csv-import-open";

/** Fenêtre entité : focus / fermeture depuis la sidebar. */
export const FOCUS_ENTITY_WINDOW_EVENT = "focus-entity-window";
export const CLOSE_ENTITY_WINDOW_EVENT = "close-entity-window";

/** Fenêtre tâches (modal). */
export const FOCUS_TACHES_WINDOW_EVENT = "focus-taches-window";
export const CLOSE_TACHES_WINDOW_EVENT = "close-taches-window";

/** Recharge la surveillance des rappels horaires (après CRUD tâche). */
export const TASK_REMINDERS_REFRESH_EVENT = "task-reminders-refresh";

/** Fenêtre stock (modal). */
export const FOCUS_STOCK_WINDOW_EVENT = "focus-stock-window";
export const CLOSE_STOCK_WINDOW_EVENT = "close-stock-window";

/** Fenêtre discussion IA (tableau de bord). */
export const FOCUS_AI_WINDOW_EVENT = "focus-ai-window";
export const CLOSE_AI_WINDOW_EVENT = "close-ai-window";

/** Liste des conversations IA (tableau de bord, hors sidebar). */
export const AI_CONVERSATIONS_REFRESH_EVENT = "ai-conversations-refresh";

/** Ouvrir une conversation existante sur le tableau de bord. */
export const AI_CONVERSATION_SELECT_EVENT = "ai-conversation-select";

/** Nouvelle discussion vide (tableau de bord). */
export const AI_CONVERSATION_NEW_EVENT = "ai-conversation-new";
