import type { ScreenConfigFile } from "@/types/screen";

const modules = import.meta.glob<{ default: ScreenConfigFile }>("./*.json", {
  eager: true,
});

/** Registre auto des écrans métier DDA créés par Loggy (hors `screen.system`). */
export const screenRegistry: ScreenConfigFile[] = Object.values(modules)
  .map((m) => m.default)
  .filter((cfg) => !cfg.screen.system);

export function getScreenByKey(key: string): ScreenConfigFile | undefined {
  return screenRegistry.find((c) => c.screen.key === key);
}

export function getScreenByRoute(route: string): ScreenConfigFile | undefined {
  return screenRegistry.find((c) => c.screen.route === route);
}
