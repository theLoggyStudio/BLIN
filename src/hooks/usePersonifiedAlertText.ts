import { useEffect, useState } from "react";
import type { AlertVariant } from "@/items/Alert";
import { personifyAlertMessage, shouldPersonifyAlertText } from "@/lib/alertPersonify";

export interface PersonifiedAlertTextState {
  text: string | undefined;
  loading: boolean;
}

/** Affiche la version réécrite par Loggy (phrases naturelles). */
export function usePersonifiedAlertText(
  text: string | undefined,
  variant: AlertVariant,
  persona: "loggy" | false = "loggy",
): PersonifiedAlertTextState {
  const shouldRewrite = persona !== false && shouldPersonifyAlertText(text);
  const [display, setDisplay] = useState<string | undefined>(
    shouldRewrite ? undefined : text,
  );

  useEffect(() => {
    if (!shouldRewrite) {
      setDisplay(text);
      return;
    }

    let cancelled = false;
    setDisplay(undefined);

    void personifyAlertMessage(text!, variant).then((rewritten) => {
      if (!cancelled && rewritten.trim()) setDisplay(rewritten);
    });

    return () => {
      cancelled = true;
    };
  }, [text, variant, persona, shouldRewrite]);

  if (!shouldRewrite) {
    return { text: text ?? undefined, loading: false };
  }

  return {
    text: display ?? text,
    loading: display === undefined,
  };
}
