import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export interface AppBranding {
  title: string;
  slogan: string;
  logoSrc: string;
}

export function buildWindowTitle(title: string, slogan: string): string {
  const t = title.trim();
  const s = slogan.trim();
  if (t && s) return `${t} — ${s}`;
  return t || s || "Application";
}

function updateFavicon(logoSrc: string) {
  let link = document.querySelector<HTMLLinkElement>('link[rel="icon"]');
  if (!link) {
    link = document.createElement("link");
    link.rel = "icon";
    document.head.appendChild(link);
  }
  link.type = logoSrc.includes("image/svg") ? "image/svg+xml" : "image/png";
  link.href = logoSrc;
}

/** Titre navigateur, favicon (sans toucher l'icône barre des tâches — gérée côté Rust). */
export async function applyAppBranding(branding: AppBranding): Promise<void> {
  const windowTitle = buildWindowTitle(branding.title, branding.slogan);
  document.title = windowTitle;
  updateFavicon(branding.logoSrc);

  if (!isTauri()) return;

  try {
    const res = await invoke<{ window_title: string }>("entity_branding_apply_title");
    if (res.window_title) {
      document.title = res.window_title;
    }
  } catch {
    try {
      const win = getCurrentWindow();
      await win.setTitle(windowTitle);
    } catch {
      /* fenêtre non prête */
    }
  }
}

/** @deprecated L'icône barre des tâches n'est plus rafraîchie à l'exécution. */
export async function restoreTaskbarIcon(): Promise<void> {
  /* no-op — icône appliquée au démarrage ou si le logo écosystème change */
}
