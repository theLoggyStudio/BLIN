import { Navigate } from "react-router-dom";
import type { ReactNode } from "react";
import type { Privilege } from "@/types/auth";
import { useAuth } from "@/hooks/useAuth";

interface RequirePrivilegeProps {
  privilege: Privilege;
  children: ReactNode;
}

export function RequirePrivilege({ privilege, children }: RequirePrivilegeProps) {
  const { user, loading, hasPrivilege } = useAuth();

  if (loading) {
    return (
      <div className="min-h-[40vh] flex items-center justify-center">
        <div className="h-8 w-8 rounded-full border-2 border-secondary border-t-transparent animate-spin" />
      </div>
    );
  }

  if (!user || !hasPrivilege(privilege)) {
    return <Navigate to="/interdit" replace />;
  }

  return <>{children}</>;
}
