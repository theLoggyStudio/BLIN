import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  ChevronsLeft,
  ChevronsRight,
  ChevronsUpDown,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "./Button";

export interface Column<T> {
  key: string;
  header: string;
  sortable?: boolean;
  render?: (row: T) => ReactNode;
  className?: string;
}

export interface TablePaginationProps {
  /** Taille de page initiale (défaut 10). */
  pageSize?: number;
  /** Choix proposés dans le sélecteur (défaut [10, 25, 50, 100]). */
  pageSizeOptions?: number[];
  /** Afficher le sélecteur « lignes par page » (défaut true). */
  showPageSizeSelector?: boolean;
  /** Masquer toute la barre si une seule page (défaut false = toujours afficher le compteur). */
  hideWhenSinglePage?: boolean;
}

interface TableauProps<T> extends TablePaginationProps {
  columns: Column<T>[];
  data: T[];
  keyExtractor: (row: T) => string;
  emptyMessage?: string;
  onRowClick?: (row: T) => void;
}

type SortDir = "asc" | "desc";

const DEFAULT_PAGE_SIZES = [10, 25, 50, 100];

function buildPageList(current: number, total: number): (number | "ellipsis")[] {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i);
  }
  const pages: (number | "ellipsis")[] = [];
  const add = (n: number) => {
    if (n >= 0 && n < total && !pages.includes(n)) pages.push(n);
  };
  add(0);
  if (current > 2) pages.push("ellipsis");
  for (let i = current - 1; i <= current + 1; i++) add(i);
  if (current < total - 3) pages.push("ellipsis");
  add(total - 1);
  return pages;
}

export function Tableau<T extends Record<string, unknown>>({
  columns,
  data,
  keyExtractor,
  pageSize: initialPageSize = 10,
  pageSizeOptions = DEFAULT_PAGE_SIZES,
  showPageSizeSelector = true,
  hideWhenSinglePage = false,
  emptyMessage = "Aucune donnée",
  onRowClick,
}: TableauProps<T>) {
  const [sortKey, setSortKey] = useState<string | null>(null);
  const [sortDir, setSortDir] = useState<SortDir>("asc");
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(initialPageSize);

  const sorted = useMemo(() => {
    if (!sortKey) return data;
    return [...data].sort((a, b) => {
      const av = a[sortKey];
      const bv = b[sortKey];
      if (av === bv) return 0;
      if (av == null) return 1;
      if (bv == null) return -1;
      const cmp = String(av).localeCompare(String(bv), "fr", { numeric: true });
      return sortDir === "asc" ? cmp : -cmp;
    });
  }, [data, sortKey, sortDir]);

  const totalPages = Math.max(1, Math.ceil(sorted.length / pageSize));
  const safePage = Math.min(page, totalPages - 1);
  const start = safePage * pageSize;
  const end = Math.min(start + pageSize, sorted.length);
  const paginated = sorted.slice(start, end);

  useEffect(() => {
    setPage(0);
  }, [data.length, pageSize, sortKey, sortDir]);

  useEffect(() => {
    if (page >= totalPages) {
      setPage(Math.max(0, totalPages - 1));
    }
  }, [page, totalPages]);

  const toggleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
    setPage(0);
  };

  const showPagination =
    sorted.length > 0 && (!hideWhenSinglePage || totalPages > 1);

  const pageItems = buildPageList(safePage, totalPages);

  const SortIcon = ({ colKey }: { colKey: string }) => {
    if (sortKey !== colKey)
      return <ChevronsUpDown className="h-3.5 w-3.5 text-muted" />;
    return sortDir === "asc" ? (
      <ChevronUp className="h-3.5 w-3.5 text-secondary" />
    ) : (
      <ChevronDown className="h-3.5 w-3.5 text-secondary" />
    );
  };

  return (
    <div className="overflow-hidden rounded-xl border border-border">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-surface-elevated/50">
              {columns.map((col) => (
                <th
                  key={col.key}
                  className={cn(
                    "px-4 py-3 text-left font-medium text-muted",
                    col.className,
                  )}
                >
                  {col.sortable ? (
                    <button
                      type="button"
                      className="inline-flex items-center gap-1 hover:text-foreground transition-colors"
                      onClick={() => toggleSort(col.key)}
                    >
                      {col.header}
                      <SortIcon colKey={col.key} />
                    </button>
                  ) : (
                    col.header
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {paginated.length === 0 ? (
              <tr>
                <td
                  colSpan={columns.length}
                  className="px-4 py-12 text-center text-muted"
                >
                  {emptyMessage}
                </td>
              </tr>
            ) : (
              paginated.map((row) => (
                <tr
                  key={keyExtractor(row)}
                  className={cn(
                    "border-b border-border/50 last:border-0 transition-colors",
                    onRowClick && "cursor-pointer hover:bg-surface-elevated/30",
                  )}
                  onClick={() => onRowClick?.(row)}
                >
                  {columns.map((col) => (
                    <td key={col.key} className={cn("px-4 py-3", col.className)}>
                      {col.render
                        ? col.render(row)
                        : String(row[col.key] ?? "—")}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {showPagination && (
        <div
          className="flex flex-col gap-3 border-t border-border px-4 py-3 bg-surface sm:flex-row sm:items-center sm:justify-between"
          role="navigation"
          aria-label="Pagination du tableau"
        >
          <div className="flex flex-wrap items-center gap-3 text-xs text-muted">
            <span>
              {sorted.length === 0
                ? "0 résultat"
                : `${start + 1}–${end} sur ${sorted.length}`}
            </span>
            {showPageSizeSelector && (
              <label className="inline-flex items-center gap-2">
                <span className="text-muted">Lignes par page</span>
                <select
                  value={pageSize}
                  onChange={(e) => {
                    setPageSize(Number(e.target.value));
                    setPage(0);
                  }}
                  className="rounded-md border border-border bg-background px-2 py-1 text-foreground text-xs focus:border-secondary focus:outline-none focus:ring-1 focus:ring-secondary"
                  aria-label="Nombre de lignes par page"
                >
                  {pageSizeOptions.map((n) => (
                    <option key={n} value={n}>
                      {n}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>

          {totalPages > 1 && (
            <div className="flex flex-wrap items-center gap-1">
              <Button
                variant="outline"
                size="sm"
                disabled={safePage === 0}
                onClick={() => setPage(0)}
                aria-label="Première page"
                className="px-2"
              >
                <ChevronsLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={safePage === 0}
                onClick={() => setPage((p) => Math.max(0, p - 1))}
                aria-label="Page précédente"
                className="px-2"
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>

              <div className="flex items-center gap-0.5 mx-1">
                {pageItems.map((item, idx) =>
                  item === "ellipsis" ? (
                    <span
                      key={`ellipsis-${idx}`}
                      className="px-2 text-muted select-none"
                    >
                      …
                    </span>
                  ) : (
                    <button
                      key={item}
                      type="button"
                      onClick={() => setPage(item)}
                      aria-label={`Page ${item + 1}`}
                      aria-current={item === safePage ? "page" : undefined}
                      className={cn(
                        "min-w-[2rem] rounded-md px-2 py-1.5 text-xs font-medium transition-colors",
                        item === safePage
                          ? "bg-secondary text-background"
                          : "text-muted hover:bg-surface-elevated hover:text-foreground",
                      )}
                    >
                      {item + 1}
                    </button>
                  ),
                )}
              </div>

              <Button
                variant="outline"
                size="sm"
                disabled={safePage >= totalPages - 1}
                onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
                aria-label="Page suivante"
                className="px-2"
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={safePage >= totalPages - 1}
                onClick={() => setPage(totalPages - 1)}
                aria-label="Dernière page"
                className="px-2"
              >
                <ChevronsRight className="h-4 w-4" />
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
