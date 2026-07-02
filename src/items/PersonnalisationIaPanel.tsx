import { useEffect, useReducer, useRef, useState } from "react";
import { Mic, Play, Plus, Square, Trash2, Volume2 } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { Text } from "@/items/Text";
import { useAuth } from "@/hooks/useAuth";
import {
  DEFAULT_VOICE_ID,
  LOGGY_VOICE_CHANGED_EVENT,
  analyzeVoiceRecording,
  deleteVoiceProfile,
  getActiveVoiceId,
  getVoiceProfile,
  getVoiceProfilesForUser,
  isLoggyVoiceAutoEnabled,
  isLoggyVoiceEnabled,
  isLoggyVoiceSupported,
  isSharedVoiceId,
  listSystemVoices,
  newVoiceProfileId,
  saveVoiceProfile,
  setActiveVoiceId,
  setLoggyVoiceAutoEnabled,
  setLoggyVoiceEnabled,
  speakWithProfile,
  formatPitchLabel,
  PITCH_MIN,
  PITCH_MAX,
  type VoiceProfile,
} from "@/lib/loggyVoice";

const TEST_PHRASE = "Bonjour, je suis Loggy. Voici un aperçu de cette voix.";

function useVoiceVersion(): number {
  const [version, bump] = useReducer((x: number) => x + 1, 0);
  useEffect(() => {
    const handler = () => bump();
    window.addEventListener(LOGGY_VOICE_CHANGED_EVENT, handler);
    return () => window.removeEventListener(LOGGY_VOICE_CHANGED_EVENT, handler);
  }, []);
  return version;
}

function useSystemVoices(): SpeechSynthesisVoice[] {
  const [voices, setVoices] = useState<SpeechSynthesisVoice[]>(() => listSystemVoices());
  useEffect(() => {
    if (!isLoggyVoiceSupported()) return;
    const update = () => setVoices(listSystemVoices());
    update();
    window.speechSynthesis.addEventListener?.("voiceschanged", update);
    return () => window.speechSynthesis.removeEventListener?.("voiceschanged", update);
  }, []);
  return voices;
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  display,
  disabled,
  minLabel,
  maxLabel,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  display: string;
  disabled?: boolean;
  minLabel?: string;
  maxLabel?: string;
  onChange: (value: number) => void;
}) {
  return (
    <label className="block">
      <span className="mb-1 flex items-center justify-between text-sm text-foreground">
        <span>{label}</span>
        <span className="font-mono text-xs text-muted">{display}</span>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        className="h-2 w-full cursor-pointer appearance-none rounded-full bg-surface-elevated accent-secondary disabled:cursor-not-allowed disabled:opacity-50"
        onChange={(e) => onChange(Number(e.target.value))}
      />
      {(minLabel || maxLabel) && (
        <span className="mt-0.5 flex justify-between text-[11px] uppercase tracking-wide text-muted">
          <span>{minLabel}</span>
          <span>{maxLabel}</span>
        </span>
      )}
    </label>
  );
}

/** Panneau Paramètres — personnalisation IA : voix de Loggy (réglages + profils). */
export function PersonnalisationIaPanel() {
  const { user } = useAuth();
  const userId = user?.id ?? null;
  useVoiceVersion();
  const voices = useSystemVoices();

  const supported = isLoggyVoiceSupported();
  const enabled = isLoggyVoiceEnabled();
  const autoEnabled = isLoggyVoiceAutoEnabled();
  const profiles = getVoiceProfilesForUser(userId);
  const activeId = getActiveVoiceId(userId);
  const selected = getVoiceProfile(activeId) ?? profiles[0];
  const isShared = isSharedVoiceId(selected.id);
  const ownedCount = profiles.filter((p) => p.owner !== null).length;

  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const [recording, setRecording] = useState(false);
  const [recordedBlob, setRecordedBlob] = useState<Blob | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [recError, setRecError] = useState<string | null>(null);
  const [analysisNote, setAnalysisNote] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      try {
        mediaRecorderRef.current?.stream?.getTracks().forEach((t) => t.stop());
      } catch {
        /* noop */
      }
    };
  }, []);

  if (!supported) {
    return (
      <Alert
        variant="info"
        size="box"
        className="px-4 py-3"
        message="La synthèse vocale n'est pas disponible dans cet environnement : la voix de Loggy ne peut pas être activée sur ce poste."
      />
    );
  }

  const voiceOptions = [
    { value: "", label: "Voix par défaut du système" },
    ...voices.map((v) => ({ value: v.voiceURI, label: `${v.name} (${v.lang})` })),
  ];

  const selectedVoiceValue = selected.voiceURI
    ? voices.find((v) => v.voiceURI === selected.voiceURI || v.name === selected.voiceURI)
        ?.voiceURI ?? ""
    : "";

  const updateSelected = (patch: Partial<VoiceProfile>) => {
    if (isShared || selected.owner === null) return;
    saveVoiceProfile({ ...selected, ...patch });
  };

  const createBlankVoice = () => {
    if (!userId) return;
    const frVoice = voices.find((v) => v.lang?.toLowerCase().startsWith("fr"));
    const profile: VoiceProfile = {
      id: newVoiceProfileId(),
      name: `Ma voix ${ownedCount + 1}`,
      voiceURI: frVoice?.voiceURI ?? null,
      pitch: 1,
      rate: 1,
      volume: 1,
      owner: userId,
    };
    saveVoiceProfile(profile);
    setActiveVoiceId(userId, profile.id);
  };

  const removeSelected = () => {
    if (isShared) return;
    deleteVoiceProfile(selected.id);
    setActiveVoiceId(userId, DEFAULT_VOICE_ID);
  };

  const startRecording = async () => {
    setRecError(null);
    setAnalysisNote(null);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const recorder = new MediaRecorder(stream);
      chunksRef.current = [];
      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunksRef.current.push(e.data);
      };
      recorder.onstop = () => {
        stream.getTracks().forEach((t) => t.stop());
        setRecordedBlob(
          new Blob(chunksRef.current, { type: recorder.mimeType || "audio/webm" }),
        );
      };
      mediaRecorderRef.current = recorder;
      recorder.start();
      setRecording(true);
    } catch (e) {
      setRecError(`Micro indisponible ou accès refusé : ${String(e)}`);
    }
  };

  const stopRecording = () => {
    try {
      mediaRecorderRef.current?.stop();
    } catch {
      /* noop */
    }
    setRecording(false);
  };

  const createVoiceFromRecording = async () => {
    if (!recordedBlob || !userId) return;
    setAnalyzing(true);
    setRecError(null);
    try {
      const analysis = await analyzeVoiceRecording(recordedBlob);
      const frVoice = voices.find((v) => v.lang?.toLowerCase().startsWith("fr"));
      const profile: VoiceProfile = {
        id: newVoiceProfileId(),
        name: `Ma voix ${ownedCount + 1}`,
        voiceURI: frVoice?.voiceURI ?? null,
        pitch: analysis.pitch,
        rate: analysis.rate,
        volume: analysis.volume,
        owner: userId,
      };
      saveVoiceProfile(profile);
      setActiveVoiceId(userId, profile.id);
      setRecordedBlob(null);
      setAnalysisNote(
        analysis.fundamentalHz
          ? `Voix créée — hauteur détectée ≈ ${Math.round(analysis.fundamentalHz)} Hz, ` +
              `tonalité ${profile.pitch.toFixed(2)}, vitesse ${profile.rate.toFixed(2)}.`
          : "Voix créée à partir de l'enregistrement (hauteur indéterminée, réglages par défaut appliqués).",
      );
      speakWithProfile("Voici votre nouvelle voix, générée depuis votre enregistrement.", profile);
    } catch (e) {
      setRecError(`Analyse impossible : ${String(e)}`);
    } finally {
      setAnalyzing(false);
    }
  };

  return (
    <div className="space-y-5">
      <Text variant="muted" className="text-sm">
        Donnez une voix à Loggy : il lit ses réponses à voix haute (voix du système, hors-ligne).
        Réglages et voix personnelles sont enregistrés sur ce poste.
      </Text>

      <label className="flex cursor-pointer items-center gap-3 rounded-lg border border-border px-3 py-2.5">
        <input
          type="checkbox"
          checked={enabled}
          className="h-4 w-4 rounded border-border accent-secondary"
          onChange={(e) => setLoggyVoiceEnabled(e.target.checked)}
        />
        <span className="flex-1 text-sm text-foreground">
          Activer la voix de Loggy (affiche le bouton « Écouter » et autorise la lecture)
        </span>
      </label>

      {enabled && (
        <>
          <label className="flex cursor-pointer items-center gap-3 rounded-lg border border-border px-3 py-2.5">
            <input
              type="checkbox"
              checked={autoEnabled}
              className="h-4 w-4 rounded border-border accent-secondary"
              onChange={(e) => setLoggyVoiceAutoEnabled(e.target.checked)}
            />
            <span className="flex-1 text-sm text-foreground">
              Lire automatiquement chaque nouvelle réponse de Loggy
            </span>
          </label>

          <div className="space-y-4 rounded-lg border border-border bg-surface-elevated/30 p-4">
            <div className="flex flex-wrap items-end justify-between gap-3">
              <div className="min-w-[12rem] flex-1">
                <Select
                  label="Voix active"
                  options={profiles.map((p) => ({
                    value: p.id,
                    label: p.owner === null ? `${p.name} (partagée)` : p.name,
                  }))}
                  value={activeId}
                  onChange={(e) => setActiveVoiceId(userId, e.target.value)}
                />
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => speakWithProfile(TEST_PHRASE, selected)}
              >
                <Play className="h-4 w-4" />
                Tester
              </Button>
            </div>

            <div className="space-y-4">
              {isShared ? (
                <Text variant="muted" className="text-xs">
                  Cette voix est partagée par tous les utilisateurs et n&apos;est pas modifiable
                  (réglages affichés ci-dessous). Créez votre propre voix pour la personnaliser.
                  {selected.id !== DEFAULT_VOICE_ID && (
                    <>
                      {" "}
                      Les voix asiatiques utilisent les voix japonaises/chinoises installées sur
                      Windows (Paramètres → Heure et langue → Voix).
                    </>
                  )}
                </Text>
              ) : (
                <Input
                  aria-label="Nom de la voix"
                  value={selected.name}
                  placeholder="Nom de la voix"
                  onChange={(e) => updateSelected({ name: e.target.value })}
                />
              )}
              <Select
                label="Voix système de base"
                options={voiceOptions}
                value={selectedVoiceValue}
                disabled={isShared}
                onChange={(e) => updateSelected({ voiceURI: e.target.value || null })}
              />
              <Slider
                label="Tonalité (filtre grave ↔ aiguë)"
                value={selected.pitch}
                min={PITCH_MIN}
                max={PITCH_MAX}
                step={0.05}
                display={`${formatPitchLabel(selected.pitch)} (${selected.pitch.toFixed(2)})`}
                disabled={isShared}
                minLabel="Très grave"
                maxLabel="Très aiguë"
                onChange={(v) => updateSelected({ pitch: v })}
              />
              <Slider
                label="Vitesse"
                value={selected.rate}
                min={0.5}
                max={2}
                step={0.05}
                display={selected.rate.toFixed(2)}
                disabled={isShared}
                minLabel="Lente"
                maxLabel="Rapide"
                onChange={(v) => updateSelected({ rate: v })}
              />
              <Slider
                label="Volume (amplitude)"
                value={selected.volume}
                min={0}
                max={1}
                step={0.05}
                display={`${Math.round(selected.volume * 100)} %`}
                disabled={isShared}
                minLabel="Faible"
                maxLabel="Fort"
                onChange={(v) => updateSelected({ volume: v })}
              />
              {!isShared && (
                <div className="flex justify-end">
                  <Button type="button" variant="ghost" size="sm" onClick={removeSelected}>
                    <Trash2 className="h-4 w-4" />
                    Supprimer cette voix
                  </Button>
                </div>
              )}
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="secondary"
              size="sm"
              disabled={!userId}
              onClick={createBlankVoice}
            >
              <Plus className="h-4 w-4" />
              Créer une voix
            </Button>
          </div>

          <div className="space-y-3 rounded-lg border border-border bg-surface-elevated/30 p-4">
            <div className="flex items-center gap-2">
              <Volume2 className="h-4 w-4 text-secondary" aria-hidden />
              <Text variant="label">Créer une voix depuis un enregistrement</Text>
            </div>
            <Text variant="muted" className="text-xs">
              Enregistrez quelques secondes de votre voix : Loggy analyse la hauteur, le débit et
              le volume, puis crée automatiquement un profil calé sur votre voix (approximation
              paramétrique, basée sur une voix système).
            </Text>

            <div className="flex flex-wrap items-center gap-2">
              {recording ? (
                <Button type="button" variant="danger" size="sm" onClick={stopRecording}>
                  <Square className="h-4 w-4" />
                  Arrêter l&apos;enregistrement
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  disabled={!userId || analyzing}
                  onClick={() => void startRecording()}
                >
                  <Mic className="h-4 w-4" />
                  {recordedBlob ? "Réenregistrer" : "Enregistrer ma voix"}
                </Button>
              )}
              {recordedBlob && !recording && (
                <Button
                  type="button"
                  size="sm"
                  disabled={analyzing}
                  onClick={() => void createVoiceFromRecording()}
                >
                  {analyzing ? "Analyse…" : "Analyser et créer la voix"}
                </Button>
              )}
              {recording && (
                <span className="flex items-center gap-1.5 text-xs text-primary">
                  <span className="loggy-rec-dot" />
                  Enregistrement en cours…
                </span>
              )}
            </div>

            {!userId && (
              <Text variant="muted" className="text-xs">
                Connectez-vous pour créer des voix personnelles.
              </Text>
            )}
            {recError && <Alert variant="danger" size="field" message={recError} />}
            {analysisNote && <Alert variant="info" size="field" message={analysisNote} />}
          </div>
        </>
      )}
    </div>
  );
}
