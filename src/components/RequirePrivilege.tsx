import { Navigate } from "react-router-dom";
import type { ReactNode } from "react";
import type { Privilege } from "@/types/auth";
import { useAuth } from "@/hooks/useAuth";
import { usePrivileges } from "@/hooks/usePrivilege";

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

interface RequireAnyPrivilegeProps {
  privileges: (Privilege | string)[];
  children: ReactNode;
}

export function RequireAnyPrivilege({ privileges, children }: RequireAnyPrivilegeProps) {
  const { user, loading } = useAuth();
  const allowed = usePrivileges(privileges, "any");

  if (loading) {
    return (
      <div className="min-h-[40vh] flex items-center justify-center">
        <div className="h-8 w-8 rounded-full border-2 border-secondary border-t-transparent animate-spin" />
      </div>
    );
  }

  if (!user || !allowed) {
    return <Navigate to="/interdit" replace />;
  }

  return <>{children}</>;
}
