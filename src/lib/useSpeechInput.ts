import { useCallback, useEffect, useRef } from "react";
import SpeechRecognition, { useSpeechRecognition } from "react-speech-recognition";

const DEFAULT_LANG = "fr-FR";

/** Dictée vocale via Web Speech API (react-speech-recognition). Compatible WebView2 / Tauri. */
export function useSpeechInput(
  value: string,
  onChange: (next: string) => void,
  language = DEFAULT_LANG,
) {
  const prefixRef = useRef("");
  const {
    transcript,
    listening,
    resetTranscript,
    browserSupportsSpeechRecognition,
    isMicrophoneAvailable,
  } = useSpeechRecognition();

  useEffect(() => {
    if (!listening) return;
    const prefix = prefixRef.current;
    const spoken = transcript.trimStart();
    if (!spoken) return;
    onChange(prefix ? `${prefix} ${spoken}` : spoken);
  }, [transcript, listening, onChange]);

  const stop = useCallback(() => {
    void SpeechRecognition.stopListening();
  }, []);

  const start = useCallback(() => {
    prefixRef.current = value.trimEnd();
    resetTranscript();
    void SpeechRecognition.startListening({ continuous: true, language });
  }, [value, resetTranscript, language]);

  const toggle = useCallback(() => {
    if (listening) stop();
    else start();
  }, [listening, start, stop]);

  const supported =
    browserSupportsSpeechRecognition && isMicrophoneAvailable !== false;

  return { supported, listening, toggle, stop, start };
}
