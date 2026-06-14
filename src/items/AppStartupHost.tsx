import { useEffect, useRef } from "react";
import { runAppStartupSequence } from "@/lib/appStartup";

/** Lance IA + préparation messages connexion dès l'ouverture de l'application. */
export function AppStartupHost() {
  const startedRef = useRef(false);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    void runAppStartupSequence();
  }, []);

  return null;
}
