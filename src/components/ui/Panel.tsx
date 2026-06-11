import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface PanelProps {
  title?: string;
  subtitle?: string;
  children: ReactNode;
  className?: string;
  headerAction?: ReactNode;
  variant?: "default" | "accent";
}

export function Panel({
  title,
  subtitle,
  children,
  className,
  headerAction,
  variant = "default",
}: PanelProps) {
  return (
    <section
      className={cn(
        "rounded-xl border bg-card overflow-hidden",
        variant === "accent"
          ? "border-secondary/30 shadow-lg shadow-secondary/5"
          : "border-border",
        className,
      )}
    >
      {(title || headerAction) && (
        <header className="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
          <div>
            {title && <h3 className="font-semibold text-foreground">{title}</h3>}
            {subtitle && <p className="text-sm text-muted mt-0.5">{subtitle}</p>}
          </div>
          {headerAction}
        </header>
      )}
      <div className="p-5">{children}</div>
    </section>
  );
}
