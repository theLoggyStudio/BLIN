import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Mic, Send } from "lucide-react";
import { sortEntitySuggestionsByPhrase } from "@/lib/entitySuggestions";
import { useSpeechInput } from "@/lib/useSpeechInput";
import { cn } from "@/lib/utils";
import type { EntitySuggestion } from "@/types/entity";

interface CommandBarProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  /** Clic sur une suggestion auto (entité du registre). */
  onSuggestionSelect?: (entityKey: string, phrase: string) => void;
  suggestionsRefreshToken?: number;
  placeholder?: string;
  className?: string;
  /** Désactive champ + envoi (legacy). Préférer sendDisabled / inputDisabled. */
  disabled?: boolean;
  /** Désactive uniquement le bouton d'envoi. */
  sendDisabled?: boolean;
  /** Désactive uniquement le champ texte. */
  inputDisabled?: boolean;
  /** Liste de suggestions au-dessus du champ (mode barre en bas). */
  suggestionsAbove?: boolean;
  /** Messages utilisateur (ordre chronologique) — ↑/↓ sans modificateur. */
  inputHistory?: string[];
  /** Réponses Loggy (ordre chronologique) — Ctrl+↑/↓. */
  responseHistory?: string[];
}

type HistoryLane = "user" | "assistant";

function messageAtIndex(history: string[], index: number): string {
  return history[history.length - 1 - index] ?? "";
}

/**
 * Barre « Que souhaitez-vous faire ? » + suggestions rattachées (trigger entités).
 * Les phrases sont chargées via `entity_list_manageable`, jamais codées en dur.
 */
export function CommandBar({
  value,
  onChange,
  onSubmit,
  onSuggestionSelect,
  suggestionsRefreshToken = 0,
  placeholder = "Que souhaitez-vous faire ?",
  className,
  disabled,
  sendDisabled = false,
  inputDisabled = false,
  suggestionsAbove = false,
  inputHistory = [],
  responseHistory = [],
}: CommandBarProps) {
  const [suggestions, setSuggestions] = useState<EntitySuggestion[]>([]);
  const [listOpen, setListOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  /** -1 = pas encore parcouru dans cette voie ; 0 = le plus récent… */
  const [userHistIndex, setUserHistIndex] = useState(-1);
  const [assistantHistIndex, setAssistantHistIndex] = useState(-1);
  const [activeLane, setActiveLane] = useState<HistoryLane | null>(null);
  const draftRef = useRef("");

  const loadSuggestions = useCallback(async () => {
    try {
      const rows = await invoke<EntitySuggestion[]>("entity_list_manageable");
      setSuggestions(sortEntitySuggestionsByPhrase(rows));
    } catch {
      setSuggestions([]);
    }
  }, []);

  useEffect(() => {
    void loadSuggestions();
  }, [loadSuggestions, suggestionsRefreshToken]);

  useEffect(() => {
    const onDocClick = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) {
        setListOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, []);

  const filtered = useMemo(() => {
    const q = value.trim().toLowerCase();
    const base = q
      ? suggestions.filter(
          (s) =>
            s.phrase.toLowerCase().includes(q) ||
            s.label.toLowerCase().includes(q) ||
            s.key.toLowerCase().includes(q),
        )
      : suggestions;
    return sortEntitySuggestionsByPhrase(base);
  }, [suggestions, value]);

  const blockInput = disabled || inputDisabled;
  const blockSend = disabled || sendDisabled;
  const showList = listOpen && filtered.length > 0 && !blockInput;

  const speech = useSpeechInput(value, onChange);

  useEffect(() => {
    if (!speech.listening) return;
    const onVis = () => {
      if (document.visibilityState === "hidden") speech.stop();
    };
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, [speech.listening, speech.stop]);

  const userHistory = useMemo(
    () => inputHistory.map((m) => m.trim()).filter((m) => m.length > 0),
    [inputHistory],
  );

  const assistantHistory = useMemo(
    () => responseHistory.map((m) => m.trim()).filter((m) => m.length > 0),
    [responseHistory],
  );

  useEffect(() => {
    setUserHistIndex(-1);
    setActiveLane((lane) => {
      if (lane === "user") {
        draftRef.current = "";
        return null;
      }
      return lane;
    });
  }, [userHistory.length]);

  useEffect(() => {
    setAssistantHistIndex(-1);
    setActiveLane((lane) => {
      if (lane === "assistant") {
        draftRef.current = "";
        return null;
      }
      return lane;
    });
  }, [assistantHistory.length]);

  const resetToDraft = useCallback(() => {
    setUserHistIndex(-1);
    setAssistantHistIndex(-1);
    setActiveLane(null);
    draftRef.current = "";
  }, []);

  const navigateLane = useCallback(
    (lane: HistoryLane, direction: "older" | "newer") => {
      const history = lane === "user" ? userHistory : assistantHistory;
      if (history.length === 0 || blockInput) return;

      const histIndex = lane === "user" ? userHistIndex : assistantHistIndex;
      const setHistIndex = lane === "user" ? setUserHistIndex : setAssistantHistIndex;

      if (direction === "older") {
        if (histIndex === -1) {
          if (activeLane === null) {
            draftRef.current = value;
          }
          setHistIndex(0);
          setActiveLane(lane);
          onChange(messageAtIndex(history, 0));
          return;
        }
        if (histIndex < history.length - 1) {
          const next = histIndex + 1;
          setHistIndex(next);
          setActiveLane(lane);
          onChange(messageAtIndex(history, next));
        }
        return;
      }

      if (histIndex === -1) return;
      if (histIndex === 0) {
        setHistIndex(-1);
        if (activeLane === lane) {
          setActiveLane(null);
          onChange(draftRef.current);
        }
        return;
      }
      const next = histIndex - 1;
      setHistIndex(next);
      setActiveLane(lane);
      onChange(messageAtIndex(history, next));
    },
    [
      activeLane,
      assistantHistIndex,
      assistantHistory,
      blockInput,
      onChange,
      userHistIndex,
      userHistory,
      value,
    ],
  );

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;

    const lane: HistoryLane = e.ctrlKey ? "assistant" : "user";
    const history = lane === "user" ? userHistory : assistantHistory;
    if (history.length === 0) return;

    e.preventDefault();
    setListOpen(false);
    navigateLane(lane, e.key === "ArrowUp" ? "older" : "newer");
  };

  const pickSuggestion = (item: EntitySuggestion) => {
    onChange(item.phrase);
    setListOpen(false);
    onSuggestionSelect?.(item.key, item.phrase);
  };

  return (
    <div ref={wrapRef} className={cn("command-bar-wrap", className)}>
      <form
        className="command-bar"
        onSubmit={(e) => {
          e.preventDefault();
          setListOpen(false);
          if (speech.listening) speech.stop();
          if (value.trim()) onSubmit();
        }}
      >
        <input
          type="text"
          value={value}
          readOnly={blockInput}
          aria-disabled={blockInput}
          onChange={(e) => {
            if (blockInput) return;
            if (speech.listening) speech.stop();
            if (activeLane !== null || userHistIndex !== -1 || assistantHistIndex !== -1) {
              resetToDraft();
            }
            onChange(e.target.value);
            setListOpen(true);
          }}
          onKeyDown={handleInputKeyDown}
          onFocus={() => setListOpen(true)}
          placeholder={placeholder}
          className={cn("command-bar-input", blockInput && "cursor-not-allowed opacity-50")}
          aria-label={placeholder}
          aria-expanded={showList}
          aria-controls="command-bar-suggestions"
          aria-autocomplete="list"
          role="combobox"
        />
        {speech.supported && (
          <button
            type="button"
            disabled={blockInput}
            className={cn(
              "command-bar-mic",
              speech.listening && "command-bar-mic--active",
              blockInput && "cursor-not-allowed opacity-50",
            )}
            aria-label={speech.listening ? "Arrêter la dictée" : "Dicter un message"}
            aria-pressed={speech.listening}
            onClick={() => {
              if (blockInput) return;
              setListOpen(false);
              speech.toggle();
            }}
          >
            <Mic className="h-4 w-4" />
          </button>
        )}
        <button
          type="submit"
          disabled={blockSend || !value.trim()}
          className="command-bar-send"
          aria-label="Envoyer"
        >
          <Send className="h-4 w-4" />
        </button>
      </form>

      {showList && (
        <ul
          id="command-bar-suggestions"
          className={cn(
            "command-bar-suggestions",
            suggestionsAbove && "command-bar-suggestions--above",
          )}
          role="listbox"
          aria-label="Suggestions automatiques"
        >
          {filtered.map((item) => (
            <li key={item.key} role="option" aria-selected={value === item.phrase}>
              <button
                type="button"
                className="command-bar-suggestion-item"
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => pickSuggestion(item)}
              >
                {item.phrase}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
