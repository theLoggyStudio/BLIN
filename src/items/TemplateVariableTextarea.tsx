import { useCallback, useEffect, useRef, useState } from "react";
import { Textarea } from "@/items/Textarea";
import {
  applyVariableSuggestion,
  getVariableSuggestions,
  type EntityVariableCatalog,
  type VariableSuggestion,
} from "@/lib/print/templateVariables";

interface TemplateVariableTextareaProps {
  label: string;
  hint?: string;
  value: string;
  onChange: (value: string) => void;
  catalog: EntityVariableCatalog;
  className?: string;
}

/** Zone de texte avec suggestions « table.champ » après {{ */
export function TemplateVariableTextarea({
  label,
  hint,
  value,
  onChange,
  catalog,
  className,
}: TemplateVariableTextareaProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [suggestions, setSuggestions] = useState<VariableSuggestion[]>([]);
  const [replaceStart, setReplaceStart] = useState(0);
  const [activeIndex, setActiveIndex] = useState(0);
  const [open, setOpen] = useState(false);

  const refreshSuggestions = useCallback(() => {
    const el = textareaRef.current;
    if (!el) {
      setOpen(false);
      return;
    }
    const hit = getVariableSuggestions(value, el.selectionStart, catalog);
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

  const pick = (item: VariableSuggestion) => {
    const el = textareaRef.current;
    if (!el) return;
    const { text, cursor } = applyVariableSuggestion(
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
          className="absolute z-20 mt-1 max-h-48 w-full overflow-y-auto rounded-lg border border-border bg-card py-1 shadow-lg"
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
