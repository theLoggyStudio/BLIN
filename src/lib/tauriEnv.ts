import { isTauri } from "@tauri-apps/api/core";

export function isTauriApp(): boolean {
  return isTauri();
}

/** Message affiché quand le front tourne hors fenêtre Tauri (ex. localhost:1420 dans Chrome). */
export function tauriUnavailableMessage(): string {
  return "Disponible uniquement dans l'application Blin. Lancez « npm run tauri dev » et utilisez la fenêtre de l'app, pas le navigateur.";
}

export function isIpcConnectionError(err: unknown): boolean {
  const s = String(err).toLowerCase();
  return (
    s.includes("connection refused") ||
    s.includes("failed to fetch") ||
    s.includes("ipc custom protocol") ||
    s.includes("net::err_connection_refused")
  );
}
