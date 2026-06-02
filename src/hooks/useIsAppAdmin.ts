import { useAuth } from "@/hooks/useAuth";

/** Compte administrateur (privilège * ou rôle Administrateur). */
export function useIsAppAdmin(): boolean {
  const { user, hasPrivilege } = useAuth();
  if (!user) return false;
  if (hasPrivilege("*")) return true;
  const role = user.role.trim().toLowerCase();
  return role === "administrateur" || role === "admin";
}
