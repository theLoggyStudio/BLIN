import { forwardRef, type InputHTMLAttributes } from "react";
import { Search } from "lucide-react";
import { cn } from "@/lib/utils";

interface SearchInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "onSearch"> {
  label?: string;
  loading?: boolean;
  onSearch: () => void;
}

/** Champ texte + loupe : la recherche ne part qu'au clic sur l'icône. */
export const SearchInput = forwardRef<HTMLInputElement, SearchInputProps>(
  ({ className, label, loading, onSearch, id, disabled, ...props }, ref) => {
    const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={inputId} className="text-sm font-medium text-muted">
            {label}
          </label>
        )}
        <div className="relative">
          <input
            ref={ref}
            id={inputId}
            type="search"
            disabled={disabled}
            autoComplete="off"
            className={cn(
              "w-full rounded-lg border border-border bg-background py-2.5 pl-3 pr-11 text-sm text-foreground select-text",
              "placeholder:text-muted/60 transition-colors duration-200",
              "focus:border-secondary focus:ring-1 focus:ring-secondary focus:outline-none",
              "disabled:cursor-not-allowed disabled:opacity-60",
              className,
            )}
            {...props}
          />
          <button
            type="button"
            className={cn(
              "absolute right-2 top-1/2 flex h-8 w-8 -translate-y-1/2 items-center justify-center rounded-md",
              "text-foreground transition-colors hover:bg-surface-elevated hover:text-secondary",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-secondary",
              "disabled:cursor-not-allowed disabled:opacity-60",
              loading && "pointer-events-none opacity-60",
            )}
            aria-label="Lancer la recherche"
            disabled={disabled || loading}
            onClick={onSearch}
          >
            <Search className="h-4 w-4" aria-hidden />
          </button>
        </div>
      </div>
    );
  },
);

SearchInput.displayName = "SearchInput";
