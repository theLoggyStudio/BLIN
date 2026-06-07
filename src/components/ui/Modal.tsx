import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "./Button";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  size?: "sm" | "md" | "lg" | "xl" | "2xl";
  footer?: ReactNode;
}

const sizeClasses = {
  sm: "max-w-md",
  md: "max-w-lg",
  lg: "max-w-2xl",
  xl: "max-w-5xl",
  "2xl": "max-w-[min(96vw,72rem)]",
};

/** Compteur global pour empiler les modales (sélection client/article dans un formulaire, etc.). */
let modalStackDepth = 0;

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
}: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const [stackLevel, setStackLevel] = useState(0);

  useEffect(() => {
    if (!open) return;
    modalStackDepth += 1;
    const level = modalStackDepth;
    setStackLevel(level);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && level === modalStackDepth) onClose();
    };
    document.addEventListener("keydown", onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = prev;
      modalStackDepth = Math.max(0, modalStackDepth - 1);
    };
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  const zBase = 200 + stackLevel * 20;

  return createPortal(
    <div
      className="fixed inset-0 flex items-center justify-center p-4"
      style={{ zIndex: zBase }}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
    >
      <button
        type="button"
        className="absolute inset-0 bg-black/78 cursor-default"
        aria-label="Fermer"
        onClick={onClose}
      />
      <div
        ref={panelRef}
        className={cn(
          "relative flex w-full max-h-[min(90dvh,calc(100vh-2rem))] flex-col overflow-hidden rounded-xl border border-border bg-card text-foreground shadow-2xl",
          sizeClasses[size],
        )}
        style={{ zIndex: zBase + 1 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex shrink-0 items-center justify-between border-b border-border px-6 py-4">
          <h2 id={titleId} className="text-lg font-semibold text-foreground">
            {title}
          </h2>
          <Button variant="ghost" size="sm" onClick={onClose} aria-label="Fermer">
            <X className="h-4 w-4" />
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">{children}</div>
        {footer && (
          <div className="flex shrink-0 justify-end gap-2 border-t border-border px-6 py-4">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
