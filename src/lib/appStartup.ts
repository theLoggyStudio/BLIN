import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  setLoginMessagesCache,
  type LoginMessagesCache,
} from "@/lib/loginMessagesCache";
import { warmUpLoggyVoices } from "@/lib/loggyVoice";

let startupPromise: Promise<void> | null = null;

/** Messages de connexion Loggy (IA demarree separement apres installation). */
export function runAppStartupSequence(): Promise<void> {
  if (startupPromise) return startupPromise;

  warmUpLoggyVoices();

  startupPromise = (async () => {
    if (!isTauri()) return;

    // Laisser l'écran de connexion répondre avant les appels IPC lourds.
    await new Promise((r) => setTimeout(r, 800));

    try {
      const cached = await invoke<LoginMessagesCache>("auth_prepare_login_messages");
      setLoginMessagesCache(cached);
    } catch (e) {
      console.warn("[LoggMagic] Préparation messages connexion :", e);
      try {
        const cached = await invoke<LoginMessagesCache>("auth_get_login_messages");
        if (cached.prepared) setLoginMessagesCache(cached);
      } catch {
        /* ignore */
      }
    }
  })();

  return startupPromise;
}
