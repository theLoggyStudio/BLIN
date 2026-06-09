export type AppWindowKind = "entity" | "taches" | "stock" | "ai";

export interface AppWindow {
  id: string;
  kind: AppWindowKind;
  title: string;
  entityKey?: string;
  openedAt: number;
}
