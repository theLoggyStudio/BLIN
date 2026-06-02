import { forwardRef, type TextareaHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
  hint?: string;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, label, error, hint, id, ...props }, ref) => {
    const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={inputId} className="text-sm font-medium text-muted">
            {label}
          </label>
        )}
        <textarea
          ref={ref}
          id={inputId}
          className={cn(
            "w-full min-h-[120px] rounded-lg border bg-background px-3 py-2.5 text-sm text-foreground font-mono",
            "placeholder:text-muted/60 transition-colors duration-200 resize-y",
            "focus:border-secondary focus:ring-1 focus:ring-secondary",
            error ? "border-primary" : "border-border",
            className,
          )}
          aria-invalid={!!error}
          {...props}
        />
        {error && (
          <p className="text-xs text-primary" role="alert">
            {error}
          </p>
        )}
        {hint && !error && <p className="text-xs text-muted">{hint}</p>}
      </div>
    );
  },
);

Textarea.displayName = "Textarea";
