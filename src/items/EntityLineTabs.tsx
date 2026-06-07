import { Plus, X } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface EntityLineTabsProps {
  /** Nom de l'entité mère (ex. demande d'achat) — titre des onglets Ligne N. */
  entityLabel: string;
  lineCount: number;
  activeIndex: number;
  onSelect: (index: number) => void;
  onAdd?: () => void;
  onRemove?: (index: number) => void;
  readOnly?: boolean;
  /** Fiche signée : navigation entre lignes sans chrome de formulaire. */
  displayOnly?: boolean;
  children: ReactNode;
  className?: string;
}

/** Onglets « Ligne N » + bouton d'ajout pour objets embarqués (écran d'ajout). */
export function EntityLineTabs({
  entityLabel,
  lineCount,
  activeIndex,
  onSelect,
  onAdd,
  onRemove,
  readOnly,
  displayOnly,
  children,
  className,
}: EntityLineTabsProps) {
  const consult = displayOnly || readOnly;

  return (
    <div
      className={cn(
        consult
          ? "space-y-3"
          : "overflow-hidden rounded-xl border border-border bg-surface text-center",
        className,
      )}
    >
      <div
        className={cn(
          consult ? "border-b border-border/60 pb-2" : "border-b border-border bg-surface-elevated/40 px-2 pt-2",
        )}
      >
        <ul
          className="flex flex-wrap items-end gap-1"
          role="tablist"
          aria-label={`Lignes ${entityLabel}`}
        >
          {Array.from({ length: lineCount }, (_, index) => {
            const active = index === activeIndex;
            const lineNumber = index + 1;
            const canRemove = !readOnly && onRemove && lineCount > 1;
            return (
              <li key={`embed-line-tab-${index}`} role="presentation" className="flex items-end">
                <div
                  className={cn(
                    consult
                      ? cn(
                          "border-b-2 pb-1 text-sm font-medium transition-colors",
                          active
                            ? "border-secondary text-foreground"
                            : "border-transparent text-muted hover:text-foreground",
                        )
                      : cn(
                          "flex items-stretch rounded-t-lg border border-b-0 transition-colors",
                          active
                            ? "border-border bg-surface text-foreground"
                            : "border-transparent bg-transparent text-muted",
                        ),
                  )}
                >
                  <button
                    type="button"
                    role="tab"
                    id={`embed-line-tab-${index}`}
                    aria-selected={active}
                    aria-controls={`embed-line-panel-${index}`}
                    title={`${lineNumber}${lineNumber === 1 ? "re" : "e"} ligne de ${entityLabel}`}
                    className={cn(
                      consult
                        ? "px-3 py-1"
                        : cn(
                            "px-4 py-2 text-sm font-medium transition-colors",
                            !active && "hover:text-foreground",
                          ),
                    )}
                    onClick={() => onSelect(index)}
                  >
                    Ligne{lineNumber}
                  </button>
                  {canRemove && (
                    <button
                      type="button"
                      aria-label={`Supprimer la ligne ${lineNumber}`}
                      title={`Supprimer la ligne ${lineNumber}`}
                      className="flex items-center border-l border-border/60 px-2 text-muted transition-colors hover:bg-primary/10 hover:text-primary"
                      onClick={(e) => {
                        e.stopPropagation();
                        onRemove(index);
                      }}
                    >
                      <X className="h-3.5 w-3.5" />
                    </button>
                  )}
                </div>
              </li>
            );
          })}
          {!readOnly && onAdd && (
            <li role="presentation">
              <button
                type="button"
                role="tab"
                aria-label="Ajouter une ligne"
                title={`Ajouter une ligne à ${entityLabel}`}
                className="mb-px flex h-9 w-9 items-center justify-center rounded-t-lg border border-b-0 border-transparent text-muted transition-colors hover:border-border hover:bg-surface hover:text-secondary"
                onClick={onAdd}
              >
                <Plus className="h-4 w-4" />
              </button>
            </li>
          )}
        </ul>
      </div>
      <div
        role="tabpanel"
        id={`embed-line-panel-${activeIndex}`}
        aria-labelledby={`embed-line-tab-${activeIndex}`}
        className={cn(consult ? "space-y-3 text-left" : "space-y-3 p-4 text-left")}
      >
        {children}
      </div>
    </div>
  );
}
