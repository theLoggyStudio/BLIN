import { invoke } from "@tauri-apps/api/core";

/** Profilage matériel forcé puis démarrage llama-server au lancement de l'application. */
export async function runAiStartupSequence(): Promise<void> {
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
}
