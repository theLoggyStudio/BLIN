import { useEffect, useRef } from "react";
import { usePrivilege } from "@/hooks/usePrivilege";
import { runAiStartupSequence } from "@/lib/aiStartup";

/** Lance le profilage matériel forcé puis le serveur IA au démarrage (connexion avec ai:utiliser). */
export function AiStartupHost() {
  const canAi = usePrivilege("ai:utiliser");
  const startedRef = useRef(false);

  useEffect(() => {
    if (!canAi || startedRef.current) return;
    startedRef.current = true;
    void runAiStartupSequence();
  }, [canAi]);

  return null;
}
