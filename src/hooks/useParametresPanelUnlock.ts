import { useCallback, useRef, useState } from "react";

/**
 * Demande le mot de passe avant chaque dépliage de panneau Paramètres.
 * Le repliage reste immédiat (sans mot de passe).
 */
export function useParametresPanelUnlock() {
  const [passwordModalOpen, setPasswordModalOpen] = useState(false);
  const resolveRef = useRef<((ok: boolean) => void) | null>(null);

  const requestUnlock = useCallback(() => {
    return new Promise<boolean>((resolve) => {
      resolveRef.current = resolve;
      setPasswordModalOpen(true);
    });
  }, []);

  const closePasswordModal = useCallback(() => {
    setPasswordModalOpen(false);
    resolveRef.current?.(false);
    resolveRef.current = null;
  }, []);

  const onPasswordVerified = useCallback(() => {
    resolveRef.current?.(true);
    resolveRef.current = null;
  }, []);

  return {
    passwordModalOpen,
    requestUnlock,
    closePasswordModal,
    onPasswordVerified,
  };
}
