import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Textarea } from "@/items/Textarea";
import {
  applyAttributSuggestion,
  getAttributSuggestions,
  type AttributSuggestion,
  type EntityAttributCatalog,
} from "@/lib/print/templateAttributes";
import { cn } from "@/lib/utils";

const SUGGESTION_MENU_MAX_HEIGHT = 192;

interface TemplateAttributeTextareaProps {
  label: string;
  hint?: string;
  value: string;
  onChange: (value: string) => void;
  catalog: EntityAttributCatalog;
  className?: string;
}

/** Zone de texte avec suggestions « table.attribut » après {{ */
export function TemplateAttributeTextarea({
  label,
  hint,
  value,
  onChange,
  catalog,
  className,
}: TemplateAttributeTextareaProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [suggestions, setSuggestions] = useState<AttributSuggestion[]>([]);
  const [replaceStart, setReplaceStart] = useState(0);
  const [activeIndex, setActiveIndex] = useState(0);
  const [open, setOpen] = useState(false);
  const [menuAbove, setMenuAbove] = useState(false);

  const updateMenuPlacement = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom - 8;
    const spaceAbove = rect.top - 8;
    setMenuAbove(
      spaceBelow < SUGGESTION_MENU_MAX_HEIGHT
      && spaceAbove > spaceBelow,
    );
  }, []);

  const refreshSuggestions = useCallback(() => {
    const el = textareaRef.current;
    if (!el) {
      setOpen(false);
      return;
    }
    const hit = getAttributSuggestions(value, el.selectionStart, catalog);
    if (!hit) {
      setOpen(false);
      return;
    }
    setReplaceStart(hit.replaceStart);
    setSuggestions(hit.suggestions);
    setActiveIndex(0);
    setOpen(true);
  }, [value, catalog]);

  useEffect(() => {
    refreshSuggestions();
  }, [refreshSuggestions]);

  useLayoutEffect(() => {
    if (!open || suggestions.length === 0) return;
    updateMenuPlacement();
    const onScrollOrResize = () => updateMenuPlacement();
    window.addEventListener("scroll", onScrollOrResize, true);
    window.addEventListener("resize", onScrollOrResize);
    return () => {
      window.removeEventListener("scroll", onScrollOrResize, true);
      window.removeEventListener("resize", onScrollOrResize);
    };
  }, [open, suggestions.length, updateMenuPlacement]);

  const pick = (item: AttributSuggestion) => {
    const el = textareaRef.current;
    if (!el) return;
    const { text, cursor } = applyAttributSuggestion(
      value,
      el.selectionStart,
      replaceStart,
      item.insert,
    );
    onChange(text);
    setOpen(false);
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(cursor, cursor);
    });
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (!open || suggestions.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => (i + 1) % suggestions.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => (i - 1 + suggestions.length) % suggestions.length);
    } else if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      pick(suggestions[activeIndex]);
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  };

  return (
    <div className="relative">
      <Textarea
        ref={textareaRef}
        label={label}
        hint={hint}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyUp={refreshSuggestions}
        onClick={refreshSuggestions}
        onKeyDown={onKeyDown}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        className={className}
      />
      {open && suggestions.length > 0 && (
        <ul
          className={cn(
            "absolute z-20 w-full overflow-y-auto rounded-lg border border-border bg-card py-1 shadow-lg",
            menuAbove ? "bottom-full mb-1" : "top-full mt-1",
          )}
          style={{ maxHeight: SUGGESTION_MENU_MAX_HEIGHT }}
          role="listbox"
        >
          {suggestions.map((s, i) => (
            <li key={`${s.insert}-${i}`}>
              <button
                type="button"
                role="option"
                aria-selected={i === activeIndex}
                className={`flex w-full flex-col px-3 py-2 text-left text-sm hover:bg-surface-elevated ${
                  i === activeIndex ? "bg-surface-elevated" : ""
                }`}
                onMouseDown={(ev) => {
                  ev.preventDefault();
                  pick(s);
                }}
              >
                <span className="font-mono text-secondary">{`{{${s.insert}`}</span>
                <span className="text-xs text-muted">
                  {s.label}
                  {s.detail ? ` (${s.detail})` : ""}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
