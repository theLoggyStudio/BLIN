import { AlertTriangle } from "lucide-react";
import type { ValidationIssue } from "@/types/screen";

interface FieldMessagesProps {
  error?: ValidationIssue;
  warning?: ValidationIssue;
}

/** Messages sous chaque champ (erreur bloquante + avertissement). */
export function FieldMessages({ error, warning }: FieldMessagesProps) {
  return (
    <>
      {error && (
        <div className="mt-1" role="alert">
          <p className="text-xs text-primary font-medium">{error.message}</p>
          {error.fixHint && (
            <p className="text-xs text-muted mt-0.5">→ {error.fixHint}</p>
          )}
        </div>
      )}
      {warning && !error && (
        <div className="mt-1 flex gap-1.5 items-start text-amber-400/90" role="status">
          <AlertTriangle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
          <div>
            <p className="text-xs">{warning.message}</p>
            {warning.fixHint && (
              <p className="text-xs text-muted mt-0.5">→ {warning.fixHint}</p>
            )}
          </div>
        </div>
      )}
    </>
  );
}
