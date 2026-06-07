import { AlertCircle, AlertTriangle, CheckCircle2, Info, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import type { ValidationIssue } from "@/types/screen";

export type AlertVariant = "success" | "danger" | "info" | "warning";
export type AlertSize = "field" | "inline" | "box" | "banner";

interface AlertItemContent {
  label?: string;
  message: string;
  fixHint?: string;
}

interface AlertProps {
  variant?: AlertVariant;
  message?: string;
  title?: string;
  fixHint?: string;
  size?: AlertSize;
  className?: string;
  id?: string;
  centered?: boolean;
  withIcon?: boolean;
  items?: AlertItemContent[];
  role?: "alert" | "status";
  children?: ReactNode;
}

const VARIANT_CONFIG: Record<
  AlertVariant,
  { border: string; accent: string; bg: string; text: string; icon: LucideIcon }
> = {
  success: {
    border: "border-emerald-500/50",
    accent: "text-emerald-400",
    bg: "bg-emerald-500/10",
    text: "text-emerald-100",
    icon: CheckCircle2,
  },
  danger: {
    border: "border-primary/40",
    accent: "text-primary",
    bg: "bg-primary/10",
    text: "text-red-100",
    icon: AlertCircle,
  },
  info: {
    border: "border-secondary/50",
    accent: "text-secondary",
    bg: "bg-secondary/10",
    text: "text-foreground",
    icon: Info,
  },
  warning: {
    border: "border-amber-500/40",
    accent: "text-amber-400",
    bg: "bg-amber-500/10",
    text: "text-amber-100",
    icon: AlertTriangle,
  },
};

/** Alerte inline réutilisable (champ, formulaire, bannière). */
export function Alert({
  variant = "danger",
  message,
  title,
  fixHint,
  size = "inline",
  className,
  id,
  centered,
  withIcon,
  items,
  role,
  children,
}: AlertProps) {
  const cfg = VARIANT_CONFIG[variant];
  const Icon = cfg.icon;
  const liveRole = role ?? (variant === "warning" || variant === "success" ? "status" : "alert");

  if (size === "field") {
    if (withIcon) {
      return (
        <div
          className={cn("flex gap-1.5 items-start", cfg.accent, className)}
          role={liveRole}
          id={id}
        >
          <Icon className="h-3.5 w-3.5 shrink-0 mt-0.5" />
          <div>
            {message && <p className="text-xs">{message}</p>}
            {fixHint && <p className="text-xs text-muted mt-0.5">→ {fixHint}</p>}
          </div>
        </div>
      );
    }

    return (
      <div className={cn(className)} role={liveRole} id={id}>
        {message && <p className={cn("text-xs font-medium", cfg.accent)}>{message}</p>}
        {fixHint && <p className="text-xs text-muted mt-0.5">→ {fixHint}</p>}
      </div>
    );
  }

  if (size === "inline") {
    return (
      <p
        className={cn(
          "text-sm",
          centered && "text-center",
          variant === "info" && centered ? "text-muted" : cfg.accent,
          className,
        )}
        role={liveRole}
        id={id}
      >
        {message}
      </p>
    );
  }

  if (size === "box") {
    return (
      <div
        className={cn(
          "rounded-lg border px-3 py-2 text-sm",
          cfg.border,
          cfg.bg,
          !children && cfg.accent,
          centered && "text-center",
          className,
        )}
        role={liveRole}
        id={id}
      >
        {children ?? message}
      </div>
    );
  }

  const displayTitle = title ?? (!items?.length ? message : undefined);
  const hasList = Boolean(items?.length);

  return (
    <div
      className={cn("rounded-xl border p-4", cfg.border, cfg.bg, className)}
      role={liveRole}
      aria-live={liveRole === "alert" ? "polite" : undefined}
      id={id}
    >
      {displayTitle && (
        <div className={cn("flex items-start gap-2", hasList && "mb-2")}>
          <Icon className={cn("h-5 w-5 shrink-0 mt-0.5", cfg.accent)} />
          <p className={cn("text-sm font-semibold", cfg.accent)}>{displayTitle}</p>
        </div>
      )}
      {hasList && items && (
        <ul className="space-y-2 pl-7 text-sm text-foreground list-disc">
          {items.map((item) => (
            <li key={`${item.label ?? ""}-${item.message}`}>
              {item.label && <span className="font-medium">{item.label}</span>}
              {item.label && " — "}
              {item.message}
              {item.fixHint && (
                <span className="block text-xs text-muted mt-0.5">→ {item.fixHint}</span>
              )}
            </li>
          ))}
        </ul>
      )}
      {!hasList && message && title && (
        <p className={cn("text-sm pl-7", cfg.text)}>{message}</p>
      )}
      {!hasList && fixHint && <p className="text-xs text-muted mt-1 pl-7">→ {fixHint}</p>}
    </div>
  );
}

interface ValidationBannerProps {
  errors: ValidationIssue[];
  warnings: ValidationIssue[];
}

/** Bannière de validation formulaire (erreurs + avertissements). */
export function ValidationBanner({ errors, warnings }: ValidationBannerProps) {
  if (errors.length === 0 && warnings.length === 0) return null;

  return (
    <div className="space-y-3">
      {errors.length > 0 && (
        <Alert
          variant="danger"
          size="banner"
          title={
            errors.length === 1
              ? "1 erreur à corriger avant enregistrement"
              : `${errors.length} erreurs à corriger avant enregistrement`
          }
          items={errors.map((e) => ({
            label: e.label,
            message: e.message,
            fixHint: e.fixHint,
          }))}
        />
      )}
      {warnings.length > 0 && (
        <Alert
          variant="warning"
          size="banner"
          role="status"
          title={
            warnings.length === 1
              ? "1 avertissement"
              : `${warnings.length} avertissements`
          }
          items={warnings.map((w) => ({
            label: w.label,
            message: w.message,
            fixHint: w.fixHint,
          }))}
        />
      )}
    </div>
  );
}

interface FieldMessagesProps {
  error?: ValidationIssue;
  warning?: ValidationIssue;
}

/** Messages sous chaque champ (erreur bloquante + avertissement). */
export function FieldMessages({ error, warning }: FieldMessagesProps) {
  return (
    <>
      {error && (
        <Alert
          variant="danger"
          size="field"
          className="mt-1"
          message={error.message}
          fixHint={error.fixHint}
        />
      )}
      {warning && !error && (
        <Alert
          variant="warning"
          size="field"
          className="mt-1 text-amber-400/90"
          withIcon
          message={warning.message}
          fixHint={warning.fixHint}
        />
      )}
    </>
  );
}

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
  const style = VARIANT_CONFIG[variant];

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
