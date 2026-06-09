import { AlertCircle, AlertTriangle, CheckCircle2, Info, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { usePersonifiedAlertText } from "@/hooks/usePersonifiedAlertText";
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

function AlertListItem({
  item,
  variant,
}: {
  item: AlertItemContent;
  variant: AlertVariant;
}) {
  const combined = [item.label, item.message].filter(Boolean).join(" — ");
  const display = usePersonifiedAlertText(combined, variant);
  const hint = usePersonifiedAlertText(item.fixHint, variant);

  return (
    <li>
      {display}
      {hint && (
        <span className="block text-xs text-muted mt-0.5">→ {hint}</span>
      )}
    </li>
  );
}

/** Alerte — tout texte affiché est réécrit par Loggy (ton collègue, 1re personne). */
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
  const hasList = Boolean(items?.length);
  const displayMessage = usePersonifiedAlertText(message, variant);
  const displayTitle = usePersonifiedAlertText(title, variant);
  const displayFixHint = usePersonifiedAlertText(fixHint, variant);

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
            {displayMessage && <p className="text-xs">{displayMessage}</p>}
            {displayFixHint && (
              <p className="text-xs text-muted mt-0.5">→ {displayFixHint}</p>
            )}
          </div>
        </div>
      );
    }

    return (
      <div className={cn(className)} role={liveRole} id={id}>
        {displayMessage && (
          <p className={cn("text-xs font-medium", cfg.accent)}>{displayMessage}</p>
        )}
        {displayFixHint && (
          <p className="text-xs text-muted mt-0.5">→ {displayFixHint}</p>
        )}
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
        {displayMessage}
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
        {children ?? displayMessage}
      </div>
    );
  }

  const resolvedTitle = displayTitle ?? (!hasList ? displayMessage : undefined);

  return (
    <div
      className={cn("rounded-xl border p-4", cfg.border, cfg.bg, className)}
      role={liveRole}
      aria-live={liveRole === "alert" ? "polite" : undefined}
      id={id}
    >
      {resolvedTitle && (
        <div className={cn("flex items-start gap-2", hasList && "mb-2")}>
          <Icon className={cn("h-5 w-5 shrink-0 mt-0.5", cfg.accent)} />
          <p className={cn("text-sm font-semibold", cfg.accent)}>{resolvedTitle}</p>
        </div>
      )}
      {hasList && items && (
        <ul className="space-y-2 pl-7 text-sm text-foreground list-disc">
          {items.map((item) => (
            <AlertListItem
              key={`${item.label ?? ""}-${item.message}-${item.fixHint ?? ""}`}
              item={item}
              variant={variant}
            />
          ))}
        </ul>
      )}
      {!hasList && displayMessage && title && (
        <p className={cn("text-sm pl-7", cfg.text)}>{displayMessage}</p>
      )}
      {!hasList && displayFixHint && (
        <p className="text-xs text-muted mt-1 pl-7">→ {displayFixHint}</p>
      )}
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

const ALERT_AUTHOR_LABEL = "Loggy";

interface AlertBubbleProps {
  message: string;
  variant?: AlertVariant;
  personify?: boolean;
  time?: string;
  entering?: boolean;
  exiting?: boolean;
  persistent?: boolean;
  actionLabel?: string;
  onAction?: () => void;
  onClose?: () => void;
}

/** Bulle toast Loggy pour notifications. */
export function AlertBubble({
  message,
  variant = "info",
  personify = true,
  time,
  entering,
  exiting,
  persistent,
  actionLabel,
  onAction,
  onClose,
}: AlertBubbleProps) {
  const style = VARIANT_CONFIG[variant];
  const displayMessage = usePersonifiedAlertText(
    message,
    variant,
    personify ? "loggy" : false,
  );

  return (
    <div
      className={cn(
        "loggy-chat-bubble loggy-alert-bubble pointer-events-auto !w-max !max-w-[min(28vw,calc(100vw-2rem))]",
        persistent && "loggy-alert-bubble-persistent",
        style.border,
        style.bg,
        entering && "loggy-alert-enter",
        exiting && "loggy-alert-exit",
      )}
      role="status"
      aria-label={`Message de ${ALERT_AUTHOR_LABEL}`}
    >
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className={cn("loggy-chat-author !mb-0 normal-case", style.accent)}>
          {ALERT_AUTHOR_LABEL}
        </span>
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
      <p className={cn("loggy-chat-text whitespace-pre-wrap", style.text)}>{displayMessage}</p>
      {actionLabel && onAction && (
        <button
          type="button"
          onClick={onAction}
          className={cn(
            "mt-2 text-left text-sm font-medium underline underline-offset-2 transition-opacity hover:opacity-80",
            style.accent,
          )}
        >
          {actionLabel}
        </button>
      )}
    </div>
  );
}
