import { useEffect, useState } from "react";
import { Square, Volume2 } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  LOGGY_VOICE_CHANGED_EVENT,
  isLoggyVoiceEnabled,
  isLoggyVoiceSupported,
  speakLoggy,
  stopLoggyVoice,
} from "@/lib/loggyVoice";

interface LoggySpeakButtonProps {
  text: string;
  className?: string;
}

/** Bouton haut-parleur : lit (ou arrête) la réponse de Loggy à voix haute. */
export function LoggySpeakButton({ text, className }: LoggySpeakButtonProps) {
  const [speaking, setSpeaking] = useState(false);
  const [enabled, setEnabled] = useState(isLoggyVoiceEnabled);

  useEffect(() => {
    const onChanged = () => setEnabled(isLoggyVoiceEnabled());
    window.addEventListener(LOGGY_VOICE_CHANGED_EVENT, onChanged);
    return () => window.removeEventListener(LOGGY_VOICE_CHANGED_EVENT, onChanged);
  }, []);

  useEffect(() => {
    return () => {
      if (speaking) stopLoggyVoice();
    };
  }, [speaking]);

  if (!isLoggyVoiceSupported() || !enabled || !text.trim()) return null;

  const toggle = () => {
    if (speaking) {
      stopLoggyVoice();
      setSpeaking(false);
      return;
    }
    const started = speakLoggy(text, () => setSpeaking(false));
    setSpeaking(started);
  };

  return (
    <button
      type="button"
      onClick={toggle}
      className={cn("loggy-speak-btn", speaking && "loggy-speak-btn--active", className)}
      aria-label={speaking ? "Arrêter la lecture" : "Écouter la réponse"}
      title={speaking ? "Arrêter la lecture" : "Écouter la réponse"}
    >
      {speaking ? <Square className="h-3.5 w-3.5" /> : <Volume2 className="h-3.5 w-3.5" />}
    </button>
  );
}
