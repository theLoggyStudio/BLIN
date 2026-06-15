import type { ReactNode } from "react";
import type { Privilege } from "@/types/auth";
import { usePrivilege, usePrivileges } from "@/hooks/usePrivilege";
import { cn } from "@/lib/utils";

interface GuardProps {
  privilege?: Privilege | string;
  anyOf?: (Privilege | string)[];
  children: ReactNode;
  fallback?: ReactNode;
  mode?: "hide" | "disable";
  className?: string;
}

export function Guard({
  privilege,
  anyOf,
  children,
  fallback = null,
  mode = "hide",
  className,
}: GuardProps) {
  const allowedSingle = usePrivilege(privilege ?? "");
  const allowedAny = usePrivileges(anyOf ?? [], "any");
  const allowed = anyOf?.length ? allowedAny : privilege ? allowedSingle : false;

  if (!allowed) {
    if (mode === "hide") return <>{fallback}</>;
    return (
      <div
        className={cn("pointer-events-none opacity-40 select-none", className)}
        aria-disabled="true"
      >
        {children}
      </div>
    );
  }

  return <>{children}</>;
}
