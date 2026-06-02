import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { NavLink } from "react-router-dom";
import { cn } from "@/lib/utils";

interface MenuItemProps {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
  collapsed?: boolean;
}

/** Bouton de navigation sidebar — vert émeraude arrondi (charte Blin). */
export function MenuItem({ to, label, icon: Icon, end, collapsed }: MenuItemProps) {
  return (
    <NavLink
      to={to}
      end={end}
      title={collapsed ? label : undefined}
      className={({ isActive }) =>
        cn(
          "nav-app-btn w-full",
          isActive && "nav-app-btn-active",
          collapsed && "justify-center px-2",
        )
      }
    >
      <Icon className="h-4 w-4 shrink-0" />
      {!collapsed && <span className="truncate">{label}</span>}
    </NavLink>
  );
}

interface MenuGroupProps {
  label: string;
  icon: LucideIcon;
  open: boolean;
  onToggle: () => void;
  collapsed?: boolean;
  children: ReactNode;
}

/** Groupe déroulant sidebar (ex. Admin). */
export function MenuGroup({
  label,
  icon: Icon,
  open,
  onToggle,
  collapsed,
  children,
}: MenuGroupProps) {
  if (collapsed) {
    return (
      <button
        type="button"
        title={label}
        onClick={onToggle}
        className="nav-app-btn w-full justify-center px-2"
      >
        <Icon className="h-4 w-4" />
      </button>
    );
  }

  return (
    <div className="space-y-1">
      <button type="button" onClick={onToggle} className="nav-app-btn w-full justify-between">
        <span className="flex items-center gap-3">
          <Icon className="h-4 w-4 shrink-0" />
          <span>{label}</span>
        </span>
        <span className={cn("text-xs transition-transform", open && "rotate-180")}>▾</span>
      </button>
      {open && (
        <div className="ml-3 space-y-1 border-l border-border pl-2">{children}</div>
      )}
    </div>
  );
}
