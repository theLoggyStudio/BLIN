import { forwardRef, type InputHTMLAttributes } from "react";
import { Alert } from "@/items/Alert";
import { cn } from "@/lib/utils";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  hint?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, label, error, hint, id, ...props }, ref) => {
    const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={inputId} className="text-sm font-medium text-muted">
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          className={cn(
            "w-full rounded-lg border bg-background px-3 py-2.5 text-sm text-foreground",
            "placeholder:text-muted/60 transition-colors duration-200",
            "focus:border-secondary focus:ring-1 focus:ring-secondary",
            error ? "border-primary" : "border-border",
            className,
          )}
          aria-invalid={!!error}
          aria-describedby={error ? `${inputId}-error` : undefined}
          {...props}
        />
        {error && (
          <Alert
            variant="danger"
            size="field"
            message={error}
            id={`${inputId}-error`}
          />
        )}
        {hint && !error && <p className="text-xs text-muted">{hint}</p>}
      </div>
    );
  },
);

Input.displayName = "Input";
