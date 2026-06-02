import { AlertCircle, AlertTriangle } from "lucide-react";
import type { ValidationIssue } from "@/types/screen";

interface ValidationBannerProps {
  errors: ValidationIssue[];
  warnings: ValidationIssue[];
}

export function ValidationBanner({ errors, warnings }: ValidationBannerProps) {
  if (errors.length === 0 && warnings.length === 0) return null;

  return (
    <div className="space-y-3">
      {errors.length > 0 && (
        <div
          className="rounded-xl border border-primary/40 bg-primary/10 p-4"
          role="alert"
          aria-live="polite"
        >
          <div className="flex items-start gap-2 mb-2">
            <AlertCircle className="h-5 w-5 text-primary shrink-0 mt-0.5" />
            <p className="text-sm font-semibold text-primary">
              {errors.length === 1
                ? "1 erreur à corriger avant enregistrement"
                : `${errors.length} erreurs à corriger avant enregistrement`}
            </p>
          </div>
          <ul className="space-y-2 pl-7 text-sm text-foreground list-disc">
            {errors.map((e) => (
              <li key={`${e.field}-${e.code}`}>
                <span className="font-medium">{e.label}</span> — {e.message}
                {e.fixHint && (
                  <span className="block text-xs text-muted mt-0.5">
                    → {e.fixHint}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      {warnings.length > 0 && (
        <div
          className="rounded-xl border border-amber-500/40 bg-amber-500/10 p-4"
          role="status"
        >
          <div className="flex items-start gap-2 mb-2">
            <AlertTriangle className="h-5 w-5 text-amber-400 shrink-0 mt-0.5" />
            <p className="text-sm font-semibold text-amber-200">
              {warnings.length === 1
                ? "1 avertissement"
                : `${warnings.length} avertissements`}
            </p>
          </div>
          <ul className="space-y-2 pl-7 text-sm text-foreground/90 list-disc">
            {warnings.map((w) => (
              <li key={`${w.field}-${w.code}`}>
                <span className="font-medium">{w.label}</span> — {w.message}
                {w.fixHint && (
                  <span className="block text-xs text-muted mt-0.5">
                    → {w.fixHint}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
