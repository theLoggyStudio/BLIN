import { useCallback, useId, useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

interface CollapsiblePanelProps {
  title: string;
  subtitle?: string;
  children: ReactNode;
  /** Ouvert par défaut (mode non contrôlé). */
  defaultOpen?: boolean;
  /** État contrôlé depuis le parent (ex. Paramètres). */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  headerAction?: ReactNode;
  className?: string;
}

/** Panneau rétractable — Paramètres et zones de configuration. */
export function CollapsiblePanel({
  title,
  subtitle,
  children,
  defaultOpen = false,
  open: openControlled,
  onOpenChange,
  headerAction,
  className,
}: CollapsiblePanelProps) {
  const [openInternal, setOpenInternal] = useState(defaultOpen);
  const titleId = useId();
  const bodyId = useId();
  const controlled = openControlled !== undefined;
  const open = controlled ? openControlled : openInternal;

  const setOpen = useCallback(
    (next: boolean) => {
      if (!controlled) setOpenInternal(next);
      onOpenChange?.(next);
    },
    [controlled, onOpenChange],
  );

  const toggle = () => setOpen(!open);

  return (
    <section
      className={cn("collapsible-panel overflow-hidden rounded-xl border border-border bg-card", className)}
      aria-labelledby={titleId}
    >
      <div
        className={cn(
          "collapsible-panel-header",
          open && "collapsible-panel-header--open",
        )}
      >
        <button
          type="button"
          className="collapsible-panel-toggle"
          onClick={toggle}
          aria-expanded={open}
          aria-controls={bodyId}
        >
          <ChevronDown
            className={cn("collapsible-panel-chevron", open && "collapsible-panel-chevron--open")}
            aria-hidden
          />
          <div className="min-w-0 flex-1">
            <h3 id={titleId} className="font-semibold text-foreground">
              {title}
            </h3>
            {subtitle && <p className="mt-0.5 text-sm text-muted">{subtitle}</p>}
          </div>
          <span className="collapsible-panel-hint">{open ? "Replier" : "Déplier"}</span>
        </button>
        {headerAction && (
          <div
            className="collapsible-panel-action"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
          >
            {headerAction}
          </div>
        )}
      </div>
      <div
        id={bodyId}
        role="region"
        aria-labelledby={titleId}
        className={cn("collapsible-panel-body-wrap", open && "collapsible-panel-body-wrap--open")}
      >
        <div className="collapsible-panel-body-inner">
          <div className="collapsible-panel-body">{children}</div>
        </div>
      </div>
    </section>
  );
}
