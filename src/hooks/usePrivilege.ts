import { useMemo } from "react";
import type { Privilege } from "@/types/auth";
import { useAuth } from "@/contexts/AuthContext";

function matchesPrivilege(userPrivileges: Privilege[], required: Privilege | string): boolean {
  const privs = userPrivileges as string[];
  if (privs.includes("*")) return true;
  if (privs.includes(required)) return true;
  const [module] = required.split(":");
  return privs.includes(`${module}:*`);
}

export function usePrivilege(required: Privilege | string): boolean {
  const { user } = useAuth();
  return useMemo(() => {
    if (!user) return false;
    return matchesPrivilege(user.privileges, required);
  }, [user, required]);
}

export function usePrivileges(required: Privilege[], mode: "all" | "any" = "all"): boolean {
  const { user } = useAuth();
  return useMemo(() => {
    if (!user) return false;
    if (mode === "any") {
      return required.some((p) => matchesPrivilege(user.privileges, p));
    }
    return required.every((p) => matchesPrivilege(user.privileges, p));
  }, [user, required, mode]);
}
