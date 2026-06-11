import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/items/Button";
import { LIST_PAGE_SIZE } from "@/constants/variable.constant";
import { cn } from "@/lib/utils";

interface PaginatedListProps<T> {
  items: T[];
  /** Taille de page (défaut : LIST_PAGE_SIZE). */
  pageSize?: number;
  renderItem: (item: T, index: number) => ReactNode;
  /** Affiché quand `items` est vide. */
  empty?: ReactNode;
  /** Classes du conteneur des éléments. */
  className?: string;
}

/**
 * Liste paginée côté client pour les items hors tableaux (privilèges, rôles…).
 * Affiche LIST_PAGE_SIZE éléments par page avec navigation Précédent / Suivant.
 */
export function PaginatedList<T>({
  items,
  pageSize = LIST_PAGE_SIZE,
  renderItem,
  empty,
  className,
}: PaginatedListProps<T>) {
  const [page, setPage] = useState(0);
  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));

  // Retour à une page valide quand la liste change (filtre, suppression…).
  useEffect(() => {
    setPage((p) => Math.min(p, Math.max(0, Math.ceil(items.length / pageSize) - 1)));
  }, [items.length, pageSize]);

  const visible = useMemo(
    () => items.slice(page * pageSize, (page + 1) * pageSize),
    [items, page, pageSize],
  );

  if (items.length === 0) {
    return <>{empty ?? null}</>;
  }

  return (
    <div className="space-y-2">
      <div className={cn(className)}>
        {visible.map((item, idx) => renderItem(item, page * pageSize + idx))}
      </div>
      {pageCount > 1 && (
        <div className="flex items-center justify-between gap-2 px-1">
          <span className="text-xs text-muted">
            Page {page + 1} / {pageCount} — {items.length} élément{items.length > 1 ? "s" : ""}
          </span>
          <div className="flex gap-1">
            <Button
              size="sm"
              variant="ghost"
              type="button"
              disabled={page === 0}
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              aria-label="Page précédente"
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              type="button"
              disabled={page >= pageCount - 1}
              onClick={() => setPage((p) => Math.min(pageCount - 1, p + 1))}
              aria-label="Page suivante"
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
