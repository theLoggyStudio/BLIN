import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { runAppStartupSequence } from "@/lib/appStartup";
import { ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";

/** Lance IA + préparation messages connexion ; propage la fin de sync Rust au front. */
export function AppStartupHost() {
  const startedRef = useRef(false);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    void runAppStartupSequence();
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const status = await invoke<{ phase: string }>("app_startup_sync_status");
        if (status.phase === "done") {
          window.dispatchEvent(new CustomEvent(ENTITY_REGISTRY_SYNCED_EVENT));
        }
      } catch {
        /* ignore */
      }
      unlisten = await listen("app-startup-sync-done", () => {
        window.dispatchEvent(new CustomEvent(ENTITY_REGISTRY_SYNCED_EVENT));
      });
    })();
    return () => {
      unlisten?.();
    };
  }, []);

  return null;
}
