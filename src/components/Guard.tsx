import type { ReactNode } from "react";
import type { Privilege } from "@/types/auth";
import { usePrivilege } from "@/hooks/usePrivilege";
import { cn } from "@/lib/utils";

interface GuardProps {
  privilege: Privilege | string;
  children: ReactNode;
  fallback?: ReactNode;
  mode?: "hide" | "disable";
  className?: string;
}

export function Guard({
  privilege,
  children,
  fallback = null,
  mode = "hide",
  className,
}: GuardProps) {
  const allowed = usePrivilege(privilege);

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
