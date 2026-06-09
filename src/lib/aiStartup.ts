import { invoke } from "@tauri-apps/api/core";

/** Démarre llama-server au lancement (profilage et réindexation restent manuels dans Paramètres). */
export async function runAiStartupSequence(): Promise<void> {
  try {
    await invoke<string>("ai_start_server");
  } catch (e) {
    console.warn("[LoggMagic] Démarrage IA — serveur :", e);
  }
}
