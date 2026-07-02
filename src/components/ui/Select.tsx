import {
  forwardRef,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type ChangeEvent,
  type FocusEvent,
  type SelectHTMLAttributes,
} from "react";
import { createPortal } from "react-dom";
import { ChevronDown, ChevronLeft, ChevronRight } from "lucide-react";
import { Alert } from "@/items/Alert";
import {
  SELECT_OPTIONS_PAGE_SIZE,
  SELECT_OPTIONS_PAGINATE_THRESHOLD,
} from "@/constants/variable.constant";
import { floatingMenuZIndex } from "@/lib/modalStack";
import {
  computeFloatingMenuStyle,
  SELECT_MENU_MAX_HEIGHT,
} from "@/lib/floatingMenuPosition";
import { cn } from "@/lib/utils";

export interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "onChange"> {
  label?: string;
  error?: string;
  hint?: string;
  options: SelectOption[];
  placeholder?: string;
  onChange?: SelectHTMLAttributes<HTMLSelectElement>["onChange"];
}

export const Select = forwardRef<HTMLButtonElement, SelectProps>(
  (
    {
      className,
      label,
      error,
      hint,
      options,
      placeholder,
      id,
      value,
      disabled,
      onChange,
      onBlur,
      name,
      form,
    },
    ref,
  ) => {
    const selectId = id ?? label?.toLowerCase().replace(/\s+/g, "-");
    const listboxId = useId();
    const triggerRef = useRef<HTMLButtonElement>(null);
    const [open, setOpen] = useState(false);
    const [menuRect, setMenuRect] = useState<DOMRect | null>(null);
    const [filterQuery, setFilterQuery] = useState("");
    const [optionsPage, setOptionsPage] = useState(0);

    const strValue = value == null ? "" : String(value);
    const selected = options.find((o) => o.value === strValue);
    const displayLabel =
      selected?.label ??
      (placeholder && strValue === "" ? placeholder : options[0]?.label ?? "—");

    const paginateOptions = options.length > SELECT_OPTIONS_PAGINATE_THRESHOLD;

    const filteredOptions = useMemo(() => {
      if (!paginateOptions || !filterQuery.trim()) return options;
      const q = filterQuery.trim().toLowerCase();
      return options.filter(
        (o) => o.label.toLowerCase().includes(q) || o.value.toLowerCase().includes(q),
      );
    }, [options, filterQuery, paginateOptions]);

    const pageCount = Math.max(
      1,
      Math.ceil(filteredOptions.length / SELECT_OPTIONS_PAGE_SIZE),
    );
    const safePage = Math.min(optionsPage, pageCount - 1);
    const visibleOptions = paginateOptions
      ? filteredOptions.slice(
          safePage * SELECT_OPTIONS_PAGE_SIZE,
          (safePage + 1) * SELECT_OPTIONS_PAGE_SIZE,
        )
      : filteredOptions;

    useEffect(() => {
      if (!open) {
        setFilterQuery("");
        setOptionsPage(0);
      }
    }, [open]);

    useEffect(() => {
      setOptionsPage((p) => Math.min(p, Math.max(0, pageCount - 1)));
    }, [filteredOptions.length, pageCount]);

    const updateMenuRect = () => {
      const el = triggerRef.current;
      if (!el) return;
      setMenuRect(el.getBoundingClientRect());
    };

    useLayoutEffect(() => {
      if (!open) return;
      updateMenuRect();
      const onScrollOrResize = () => updateMenuRect();
      window.addEventListener("scroll", onScrollOrResize, true);
      window.addEventListener("resize", onScrollOrResize);
      return () => {
        window.removeEventListener("scroll", onScrollOrResize, true);
        window.removeEventListener("resize", onScrollOrResize);
      };
    }, [open]);

    useEffect(() => {
      if (!open) return;
      const onPointerDown = (e: MouseEvent) => {
        const t = e.target as Node;
        if (triggerRef.current?.contains(t)) return;
        const menu = document.getElementById(listboxId);
        if (menu?.contains(t)) return;
        setOpen(false);
        onBlur?.({ target: { value: strValue } } as FocusEvent<HTMLSelectElement>);
      };
      document.addEventListener("mousedown", onPointerDown);
      return () => document.removeEventListener("mousedown", onPointerDown);
    }, [open, listboxId, onBlur, strValue]);

    const pick = (next: string) => {
      setOpen(false);
      onChange?.({
        target: { value: next },
        currentTarget: { value: next },
      } as ChangeEvent<HTMLSelectElement>);
      onBlur?.({ target: { value: next } } as FocusEvent<HTMLSelectElement>);
    };

    const triggerProps: ButtonHTMLAttributes<HTMLButtonElement> = {
      id: selectId,
      type: "button",
      role: "combobox",
      "aria-expanded": open,
      "aria-haspopup": "listbox",
      "aria-controls": listboxId,
      disabled,
      "aria-invalid": !!error,
    };

    const menuStyle = useMemo(() => {
      if (!open || !menuRect) return null;
      const { placement: _p, ...style } = computeFloatingMenuStyle(
        menuRect,
        floatingMenuZIndex(),
        { preferredMaxHeight: SELECT_MENU_MAX_HEIGHT },
      );
      return style;
    }, [open, menuRect]);

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={selectId} className="text-sm font-medium text-muted">
            {label}
          </label>
        )}
        <div className="relative">
          <button
            ref={(node) => {
              triggerRef.current = node;
              if (typeof ref === "function") ref(node);
              else if (ref) ref.current = node;
            }}
            {...triggerProps}
            className={cn(
              "flex w-full items-center rounded-lg border bg-background px-3 py-2.5 pr-9 text-left text-sm",
              "transition-colors duration-200 cursor-pointer",
              "focus:border-secondary focus:ring-1 focus:ring-secondary focus:outline-none",
              "disabled:cursor-not-allowed disabled:opacity-60",
              selected || !placeholder || strValue !== ""
                ? "text-foreground"
                : "text-muted/60",
              error ? "border-primary" : "border-border",
              className,
            )}
            onClick={() => {
              if (disabled) return;
              setOpen((v) => !v);
            }}
            onKeyDown={(e) => {
              if (disabled) return;
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setOpen((v) => !v);
              }
              if (e.key === "Escape") setOpen(false);
            }}
          >
            <span className="min-w-0 truncate">{displayLabel}</span>
            <ChevronDown
              className={cn(
                "pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-foreground transition-transform",
                open && "rotate-180",
              )}
            />
          </button>

          {menuStyle &&
            createPortal(
              <ul
                id={listboxId}
                role="listbox"
                aria-labelledby={selectId}
                className="overflow-y-auto rounded-lg border border-border bg-card p-1 shadow-xl"
                style={menuStyle}
              >
                {placeholder && (
                  <li
                    role="option"
                    aria-selected={strValue === ""}
                    className={cn(
                      "cursor-pointer rounded-md px-3 py-2 text-sm text-muted hover:bg-surface-elevated",
                      strValue === "" && "bg-surface-elevated/80 text-foreground",
                    )}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => pick("")}
                  >
                    {placeholder}
                  </li>
                )}
                {paginateOptions && (
                  <li className="sticky top-0 z-10 border-b border-border bg-card p-2">
                    <input
                      type="search"
                      value={filterQuery}
                      placeholder="Filtrer…"
                      className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm text-foreground focus:border-secondary focus:outline-none"
                      onMouseDown={(e) => e.stopPropagation()}
                      onChange={(e) => {
                        setFilterQuery(e.target.value);
                        setOptionsPage(0);
                      }}
                    />
                  </li>
                )}
                {visibleOptions.length === 0 ? (
                  <li className="px-3 py-2 text-sm text-muted">Aucune option</li>
                ) : (
                  visibleOptions.map((opt) => (
                    <li
                      key={opt.value || `__empty_${opt.label}`}
                      role="option"
                      aria-selected={opt.value === strValue}
                      className={cn(
                        "cursor-pointer rounded-md px-3 py-2 text-sm text-foreground hover:bg-surface-elevated",
                        opt.value === strValue && "bg-surface-elevated/80",
                      )}
                      onMouseDown={(e) => e.preventDefault()}
                      onClick={() => pick(opt.value)}
                    >
                      {opt.label}
                    </li>
                  ))
                )}
                {paginateOptions && pageCount > 1 && (
                  <li className="sticky bottom-0 flex items-center justify-between gap-2 border-t border-border bg-card px-2 py-1.5">
                    <span className="text-xs text-muted">
                      {safePage + 1}/{pageCount} — {filteredOptions.length} opt.
                    </span>
                    <div className="flex gap-1">
                      <button
                        type="button"
                        className="rounded p-1 hover:bg-surface-elevated disabled:opacity-40"
                        disabled={safePage === 0}
                        aria-label="Page précédente"
                        onMouseDown={(e) => e.preventDefault()}
                        onClick={() => setOptionsPage((p) => Math.max(0, p - 1))}
                      >
                        <ChevronLeft className="h-4 w-4" />
                      </button>
                      <button
                        type="button"
                        className="rounded p-1 hover:bg-surface-elevated disabled:opacity-40"
                        disabled={safePage >= pageCount - 1}
                        aria-label="Page suivante"
                        onMouseDown={(e) => e.preventDefault()}
                        onClick={() =>
                          setOptionsPage((p) => Math.min(pageCount - 1, p + 1))
                        }
                      >
                        <ChevronRight className="h-4 w-4" />
                      </button>
                    </div>
                  </li>
                )}
              </ul>,
              document.body,
            )}
        </div>
        {name ? <input type="hidden" name={name} value={strValue} form={form} /> : null}
        {error && <Alert variant="danger" size="field" message={error} />}
        {hint && !error && <p className="text-xs text-muted">{hint}</p>}
      </div>
    );
  },
);

Select.displayName = "Select";
