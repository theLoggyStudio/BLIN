import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import {
  currentModalStackDepth,
  modalZIndex,
  popModalStack,
  pushModalStack,
} from "@/lib/modalStack";
import { cn } from "@/lib/utils";
import { Button } from "./Button";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  size?: "sm" | "md" | "lg" | "xl" | "2xl";
  footer?: ReactNode;
  /** Bloque la fermeture et affiche un overlay de chargement. */
  busy?: boolean;
  busyLabel?: string;
  /** Bloque fermeture (Échap, backdrop, croix) sans overlay de chargement. */
  closeDisabled?: boolean;
}

const sizeClasses = {
  sm: "max-w-md",
  md: "max-w-lg",
  lg: "max-w-2xl",
  xl: "max-w-5xl",
  "2xl": "max-w-[min(96vw,72rem)]",
};

/**
 * Overlay div (pas de &lt;dialog showModal&gt;) — les pickers date/heure
 * et certains champs ne fonctionnent pas correctement dans un dialog natif sous WebView2/Tauri.
 * Rendu via portail sur document.body pour éviter les blocages dans un parent overflow.
 */
export function Modal({
  open,
  onClose,
  title,
  children,
  size = "md",
  footer,
  busy = false,
  busyLabel = "Chargement…",
  closeDisabled = false,
}: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const [stackLevel, setStackLevel] = useState(0);

  useLayoutEffect(() => {
    if (!open) return;

    const level = pushModalStack();
    setStackLevel(level);

    return () => {
      popModalStack();
      setStackLevel(0);
    };
  }, [open]);

  const closeBlocked = busy || closeDisabled;

  useEffect(() => {
    if (!open || stackLevel === 0) return;

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && stackLevel === currentModalStackDepth() && !closeBlocked) {
        onClose();
      }
    };
    document.addEventListener("keydown", onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = prev;
    };
  }, [open, onClose, closeBlocked, stackLevel]);

  const requestClose = () => {
    if (!closeBlocked) onClose();
  };

  if (!open) {
    return null;
  }

  const zBase = modalZIndex(stackLevel || currentModalStackDepth());
  const isInactive =
    stackLevel > 0 && stackLevel < currentModalStackDepth();

  return createPortal(
    <div
      className={cn(
        "fixed inset-0 flex items-center justify-center p-4 max-md:items-stretch max-md:justify-stretch max-md:p-0",
        isInactive && "pointer-events-none",
      )}
      style={{ zIndex: zBase }}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      aria-hidden={isInactive ? true : undefined}
    >
      <div
        role="presentation"
        className="absolute inset-0 cursor-default bg-black/78 max-md:bg-black/85"
        onMouseDown={isInactive ? undefined : requestClose}
      />
      <div
        ref={panelRef}
        className={cn(
          "relative flex w-full max-h-[min(90dvh,calc(100vh-2rem))] flex-col overflow-hidden rounded-xl border border-border bg-card text-foreground shadow-2xl",
          "max-md:h-[100dvh] max-md:max-h-[100dvh] max-md:rounded-none max-md:border-x-0 max-md:border-t-0 max-md:shadow-none",
          sizeClasses[size],
        )}
        style={{ zIndex: zBase + 1 }}
        onMouseDown={(e) => e.stopPropagation()}
        aria-busy={busy}
      >
        {busy && (
          <div
            className="absolute inset-0 z-20 flex flex-col items-center justify-center gap-3 bg-card/90 backdrop-blur-[2px]"
            role="status"
            aria-live="polite"
          >
            <div className="h-10 w-10 animate-spin rounded-full border-2 border-secondary border-t-transparent" />
            <p className="text-sm font-medium text-foreground">{busyLabel}</p>
          </div>
        )}
        <div className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-4 py-3 md:px-6 md:py-4">
          <h2
            id={titleId}
            className="min-w-0 text-base font-semibold leading-snug text-foreground md:text-lg"
          >
            {title}
          </h2>
          <Button variant="ghost" size="sm" onClick={requestClose} disabled={closeBlocked} aria-label="Fermer">
            <X className="h-4 w-4" />
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 py-3 md:px-6 md:py-4">
          {children}
        </div>
        {footer && (
          <div className="flex shrink-0 flex-col-reverse gap-2 border-t border-border px-4 py-3 max-md:pb-[max(0.75rem,env(safe-area-inset-bottom))] md:flex-row md:justify-end md:px-6 md:py-4 [&_button]:w-full md:[&_button]:w-auto">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
