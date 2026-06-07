import { useEffect } from "react";
import { useAuth } from "@/hooks/useAuth";
import { useAlert } from "@/contexts/AlertContext";

/** Affiche les alertes post-connexion (signatures en attente, infos post-signature). */
export function LoginNoticesHost() {
  const { loginNotices, clearLoginNotices } = useAuth();
  const { showInfo } = useAlert();

  useEffect(() => {
    if (loginNotices.length === 0) return;
    for (const message of loginNotices) {
      showInfo(message);
    }
    clearLoginNotices();
  }, [loginNotices, clearLoginNotices, showInfo]);

  return null;
}
