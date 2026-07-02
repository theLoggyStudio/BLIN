import { invoke, isTauri } from "@tauri-apps/api/core";

let aiStartupPromise: Promise<void> | null = null;

async function hasActiveSession(): Promise<boolean> {
  try {
    await invoke("auth_current_user");
    return true;
  } catch {
    return false;
  }
}

/** Profilage matériel forcé puis démarrage llama-server — uniquement si session active. */
export async function runAiStartupSequence(): Promise<void> {
  if (!isTauri()) return;
  if (aiStartupPromise) return aiStartupPromise;

  aiStartupPromise = (async () => {
    if (!(await hasActiveSession())) return;

    try {
      await invoke<string>("ai_profile_runtime", { payload: { force: true } });
    } catch (e) {
      console.warn("[LoggMagic] Démarrage IA — profilage :", e);
    }
    try {
      await invoke<string>("ai_start_server");
    } catch (e) {
      console.warn("[LoggMagic] Démarrage IA — serveur :", e);
    }
  })();

  return aiStartupPromise;
}

export function resetAiStartupSequence(): void {
  aiStartupPromise = null;
}
