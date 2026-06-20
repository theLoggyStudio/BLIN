import { invoke } from "@tauri-apps/api/core";
import { runAiStartupSequence } from "@/lib/aiStartup";
import {
  setLoginMessagesCache,
  type LoginMessagesCache,
} from "@/lib/loginMessagesCache";
import { warmUpLoggyVoices } from "@/lib/loggyVoice";

let startupPromise: Promise<void> | null = null;

/** Profilage IA + serveur llama et messages de connexion Loggy, en parallèle. */
export function runAppStartupSequence(): Promise<void> {
  if (startupPromise) return startupPromise;

  warmUpLoggyVoices();

  startupPromise = (async () => {
    const [, messagesResult] = await Promise.allSettled([
      runAiStartupSequence(),
      invoke<LoginMessagesCache>("auth_prepare_login_messages"),
    ]);

    if (messagesResult.status === "fulfilled") {
      setLoginMessagesCache(messagesResult.value);
    } else {
      console.warn("[LoggMagic] Préparation messages connexion :", messagesResult.reason);
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
