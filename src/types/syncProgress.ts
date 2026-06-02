/** Progression séquentielle des triggers (événement Tauri `entity-sync-progress`). */
export interface EntitySyncProgress {
  current: number;
  total: number;
  label: string;
  entityKey?: string;
  step: string;
  done: boolean;
}
