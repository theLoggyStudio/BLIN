import { X } from "lucide-react";
import { cn } from "@/lib/utils";

export type AlertVariant = "success" | "danger" | "info" | "warning";

const VARIANT_STYLES: Record<
  AlertVariant,
  { border: string; accent: string; bg: string; text: string }
> = {
  success: {
    border: "border-emerald-500/50",
    accent: "text-emerald-400",
    bg: "bg-emerald-500/10",
    text: "text-emerald-100",
  },
  danger: {
    border: "border-primary/50",
    accent: "text-primary",
    bg: "bg-primary/10",
    text: "text-red-100",
  },
  info: {
    border: "border-secondary/50",
    accent: "text-secondary",
    bg: "bg-secondary/10",
    text: "text-foreground",
  },
  warning: {
    border: "border-amber-500/50",
    accent: "text-amber-400",
    bg: "bg-amber-500/10",
    text: "text-amber-100",
  },
};

interface AlertBubbleProps {
  message: string;
  variant?: AlertVariant;
  time?: string;
  entering?: boolean;
  exiting?: boolean;
  onClose?: () => void;
}

/** Bulle Loggy (style chat IA) pour notifications toast. */
export function AlertBubble({
  message,
  variant = "info",
  time,
  entering,
  exiting,
  onClose,
}: AlertBubbleProps) {
  const style = VARIANT_STYLES[variant];

  return (
    <div
      className={cn(
        "loggy-chat-bubble loggy-alert-bubble pointer-events-auto w-80 max-w-[calc(100vw-2rem)]",
        style.border,
        style.bg,
        entering && "loggy-alert-enter",
        exiting && "loggy-alert-exit",
      )}
      role="status"
      aria-label="Message de Loggy"
    >
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className={cn("loggy-chat-author !mb-0", style.accent)}>Loggy</span>
        <div className="flex items-center gap-2">
          {time && (
            <span className="text-[10px] text-muted tabular-nums">{time}</span>
          )}
          {onClose && (
            <button
              type="button"
              onClick={onClose}
              className="rounded p-0.5 text-muted transition-colors hover:bg-surface-elevated hover:text-foreground"
              aria-label="Fermer le message"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      </div>
      <p className={cn("loggy-chat-text whitespace-pre-wrap", style.text)}>{message}</p>
    </div>
  );
}
