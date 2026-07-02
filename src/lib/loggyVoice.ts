/**
 * Voix de Loggy — synthèse vocale via la Web Speech API du webview (hors-ligne,
 * voix du système). Gère :
 *  - un interrupteur maître (afficher / autoriser la voix) ;
 *  - la lecture automatique des réponses ;
 *  - des profils de voix réglables (tonalité, vitesse, volume) ;
 *  - des profils propres à chaque utilisateur (la voix par défaut est partagée) ;
 *  - la création automatique d'un profil à partir d'un enregistrement (analyse
 *    de la hauteur, du débit et du volume — approximation paramétrique, pas un
 *    clonage neuronal).
 */

const ENABLED_KEY = "blin:loggy-voice-enabled";
const AUTO_KEY = "blin:loggy-voice-auto";
const PROFILES_KEY = "blin:loggy-voice-profiles";
const ACTIVE_PREFIX = "blin:loggy-voice-active:";
const CURRENT_USER_KEY = "blin:loggy-voice-current-user";

/** Identifiant de la voix par défaut partagée (non supprimable, non possédée). */
export const DEFAULT_VOICE_ID = "default";
/** Voix partagée — style asiatique féminin. */
export const ASIAN_VOICE_FEMALE_ID = "shared-asian-female";
/** Voix partagée — style asiatique masculin. */
export const ASIAN_VOICE_MALE_ID = "shared-asian-male";

const SHARED_VOICE_IDS: readonly string[] = [
  DEFAULT_VOICE_ID,
  ASIAN_VOICE_FEMALE_ID,
  ASIAN_VOICE_MALE_ID,
];

/** Profil partagé (non modifiable, non supprimable). */
export function isSharedVoiceId(id: string): boolean {
  return SHARED_VOICE_IDS.includes(id);
}

/** Événement émis à chaque changement de configuration vocale. */
export const LOGGY_VOICE_CHANGED_EVENT = "blin:loggy-voice-changed";

export interface VoiceProfile {
  id: string;
  /** Nom affiché du profil. */
  name: string;
  /** Voix système (voiceURI) ; null = voix par défaut du système. */
  voiceURI: string | null;
  /** Tonalité (0.1 → 3, 1 = normale ; filtre grave ↔ aiguë). */
  pitch: number;
  /** Vitesse (0.5 → 2, 1 = normale). */
  rate: number;
  /** Volume (0 → 1). */
  volume: number;
  /** Propriétaire (id utilisateur) ; null = voix par défaut partagée. */
  owner: string | null;
}

/** Tonalité minimale (très grave). */
export const PITCH_MIN = 0.1;
/** Tonalité maximale (très aiguë). */
export const PITCH_MAX = 3;
/** Tonalité neutre. */
export const PITCH_NEUTRAL = 1;

const DEFAULT_PROFILE: VoiceProfile = {
  id: DEFAULT_VOICE_ID,
  name: "Voix par défaut (Paul)",
  // Voix système préférée : Microsoft Paul (fr-FR), avec repli sur une voix
  // française si elle est absente sur le poste.
  voiceURI: "Microsoft Paul - French (France)",
  pitch: 1.3,
  rate: 1.2,
  volume: 0.7,
  owner: null,
};

const ASIAN_FEMALE_PROFILE: VoiceProfile = {
  id: ASIAN_VOICE_FEMALE_ID,
  name: "Style asiatique (Yuki)",
  voiceURI: "Microsoft Haruka - Japanese (Japan)",
  pitch: 1.45,
  rate: 1.05,
  volume: 0.85,
  owner: null,
};

const ASIAN_MALE_PROFILE: VoiceProfile = {
  id: ASIAN_VOICE_MALE_ID,
  name: "Style asiatique (Ken)",
  voiceURI: "Microsoft Ichiro - Japanese (Japan)",
  pitch: 0.95,
  rate: 1,
  volume: 0.85,
  owner: null,
};

const SHARED_PROFILES: VoiceProfile[] = [
  DEFAULT_PROFILE,
  ASIAN_FEMALE_PROFILE,
  ASIAN_MALE_PROFILE,
];

function emitChanged(): void {
  window.dispatchEvent(new CustomEvent(LOGGY_VOICE_CHANGED_EVENT));
}

function clamp(value: number, min: number, max: number, fallback: number): number {
  return Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;
}

/* ------------------------------------------------------------------ */
/* Disponibilité & interrupteurs                                       */
/* ------------------------------------------------------------------ */

export function isLoggyVoiceSupported(): boolean {
  return typeof window !== "undefined" && "speechSynthesis" in window;
}

/** Voix activée sur ce poste (interrupteur maître). Activé par défaut. */
export function isLoggyVoiceEnabled(): boolean {
  try {
    return localStorage.getItem(ENABLED_KEY) !== "0";
  } catch {
    return true;
  }
}

export function setLoggyVoiceEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(ENABLED_KEY, enabled ? "1" : "0");
  } catch {
    /* noop */
  }
  if (!enabled) stopLoggyVoice();
  emitChanged();
}

/** Lecture automatique des nouvelles réponses (nécessite la voix activée). */
export function isLoggyVoiceAutoEnabled(): boolean {
  if (!isLoggyVoiceEnabled()) return false;
  try {
    return localStorage.getItem(AUTO_KEY) === "1";
  } catch {
    return false;
  }
}

export function setLoggyVoiceAutoEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(AUTO_KEY, enabled ? "1" : "0");
  } catch {
    /* noop */
  }
  if (!enabled) stopLoggyVoice();
  emitChanged();
}

/* ------------------------------------------------------------------ */
/* Utilisateur courant                                                 */
/* ------------------------------------------------------------------ */

/** Définit l'utilisateur dont les voix s'appliquent (appelé à la connexion). */
export function setLoggyVoiceCurrentUser(userId: string | null): void {
  try {
    if (userId) localStorage.setItem(CURRENT_USER_KEY, userId);
    else localStorage.removeItem(CURRENT_USER_KEY);
  } catch {
    /* noop */
  }
  emitChanged();
}

export function getLoggyVoiceCurrentUser(): string | null {
  try {
    return localStorage.getItem(CURRENT_USER_KEY);
  } catch {
    return null;
  }
}

/* ------------------------------------------------------------------ */
/* Profils de voix                                                     */
/* ------------------------------------------------------------------ */

function readStoredProfiles(): VoiceProfile[] {
  try {
    const raw = localStorage.getItem(PROFILES_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as VoiceProfile[];
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((p) => p && typeof p.id === "string" && !isSharedVoiceId(p.id))
      .map((p) => ({
        id: p.id,
        name: String(p.name || "Voix"),
        voiceURI: p.voiceURI ?? null,
        pitch: clamp(Number(p.pitch), PITCH_MIN, PITCH_MAX, PITCH_NEUTRAL),
        rate: clamp(Number(p.rate), 0.5, 2, 1),
        volume: clamp(Number(p.volume), 0, 1, 1),
        owner: p.owner ?? null,
      }));
  } catch {
    return [];
  }
}

function writeStoredProfiles(profiles: VoiceProfile[]): void {
  try {
    localStorage.setItem(
      PROFILES_KEY,
      JSON.stringify(profiles.filter((p) => !isSharedVoiceId(p.id))),
    );
  } catch {
    /* noop */
  }
}

/** Tous les profils (voix par défaut en tête). */
export function getAllVoiceProfiles(): VoiceProfile[] {
  return [...SHARED_PROFILES, ...readStoredProfiles()];
}

/** Profils accessibles à un utilisateur : voix partagées + ses voix. */
export function getVoiceProfilesForUser(userId: string | null): VoiceProfile[] {
  const owned = readStoredProfiles().filter((p) => p.owner === userId);
  return [...SHARED_PROFILES, ...owned];
}

export function getVoiceProfile(id: string): VoiceProfile | null {
  const shared = SHARED_PROFILES.find((p) => p.id === id);
  if (shared) return shared;
  return readStoredProfiles().find((p) => p.id === id) ?? null;
}

/** Crée ou met à jour un profil (les voix partagées ne sont pas modifiables). */
export function saveVoiceProfile(profile: VoiceProfile): void {
  if (isSharedVoiceId(profile.id) || profile.owner === null) return;
  const profiles = readStoredProfiles();
  const idx = profiles.findIndex((p) => p.id === profile.id);
  const normalized: VoiceProfile = {
    ...profile,
    pitch: clamp(profile.pitch, PITCH_MIN, PITCH_MAX, PITCH_NEUTRAL),
    rate: clamp(profile.rate, 0.5, 2, 1),
    volume: clamp(profile.volume, 0, 1, 1),
  };
  if (idx >= 0) profiles[idx] = normalized;
  else profiles.push(normalized);
  writeStoredProfiles(profiles);
  emitChanged();
}

export function deleteVoiceProfile(id: string): void {
  if (isSharedVoiceId(id)) return;
  writeStoredProfiles(readStoredProfiles().filter((p) => p.id !== id));
  emitChanged();
}

export function newVoiceProfileId(): string {
  return `voice-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

/* ------------------------------------------------------------------ */
/* Voix active par utilisateur                                         */
/* ------------------------------------------------------------------ */

export function getActiveVoiceId(userId: string | null): string {
  try {
    const stored = localStorage.getItem(ACTIVE_PREFIX + (userId ?? "_"));
    if (stored && getVoiceProfile(stored)) return stored;
  } catch {
    /* noop */
  }
  return DEFAULT_VOICE_ID;
}

export function setActiveVoiceId(userId: string | null, id: string): void {
  try {
    localStorage.setItem(ACTIVE_PREFIX + (userId ?? "_"), id);
  } catch {
    /* noop */
  }
  emitChanged();
}

/** Profil de voix effectif pour l'utilisateur courant. */
function resolveActiveProfile(): VoiceProfile {
  const userId = getLoggyVoiceCurrentUser();
  const activeId = getActiveVoiceId(userId);
  const profile = getVoiceProfile(activeId);
  if (!profile) return DEFAULT_PROFILE;
  if (profile.owner !== null && profile.owner !== userId) return DEFAULT_PROFILE;
  return profile;
}

/* ------------------------------------------------------------------ */
/* Voix système                                                        */
/* ------------------------------------------------------------------ */

export function listSystemVoices(): SpeechSynthesisVoice[] {
  if (!isLoggyVoiceSupported()) return [];
  return window.speechSynthesis.getVoices();
}

function pickAsianVoice(preferredURI: string | null, female: boolean): SpeechSynthesisVoice | null {
  const voices = listSystemVoices();
  if (voices.length === 0) return null;

  if (preferredURI) {
    const exact = voices.find((v) => v.voiceURI === preferredURI || v.name === preferredURI);
    if (exact) return exact;
    const hint = preferredURI.replace(/^Microsoft\s+/i, "").split(" - ")[0]?.trim();
    if (hint) {
      const byHint = voices.find((v) => v.name.includes(hint));
      if (byHint) return byHint;
    }
  }

  const asian = voices.filter((v) => /^(ja|zh|ko|vi|th)/i.test(v.lang ?? ""));
  if (asian.length === 0) return null;

  const femaleRe =
    /haruka|ayumi|huihui|yaoyao|heami|nanami|yunxia|tracy|xiaoxiao|xiaoyi|female|femme/i;
  const maleRe = /ichiro|kangkang|yunxi|injoon|hyunsu|yunjian|male|homme/i;

  if (female) {
    return (
      asian.find((v) => femaleRe.test(v.name)) ??
      asian.find((v) => !maleRe.test(v.name)) ??
      asian[0]
    );
  }
  return asian.find((v) => maleRe.test(v.name)) ?? asian[asian.length - 1];
}

function pickVoice(voiceURI: string | null, profileId?: string): SpeechSynthesisVoice | null {
  const voices = listSystemVoices();
  if (voices.length === 0) return null;

  if (profileId === ASIAN_VOICE_FEMALE_ID) {
    return pickAsianVoice(voiceURI, true) ?? pickVoice(voiceURI);
  }
  if (profileId === ASIAN_VOICE_MALE_ID) {
    return pickAsianVoice(voiceURI, false) ?? pickVoice(voiceURI);
  }

  if (voiceURI) {
    const exact = voices.find((v) => v.voiceURI === voiceURI || v.name === voiceURI);
    if (exact) return exact;
  }
  const fr = voices.filter((v) => v.lang?.toLowerCase().startsWith("fr"));
  if (fr.length > 0) {
    return fr.find((v) => v.lang?.toLowerCase() === "fr-fr") ?? fr[0];
  }
  return null;
}

/* ------------------------------------------------------------------ */
/* Lecture                                                             */
/* ------------------------------------------------------------------ */

function sanitizeForSpeech(text: string): string {
  return text
    .replace(/https?:\/\/[^\s]+/g, "")
    .replace(/`{1,3}[^`]*`{1,3}/g, " ")
    .replace(/[*_#>|]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function stopLoggyVoice(): void {
  if (!isLoggyVoiceSupported()) return;
  window.speechSynthesis.cancel();
}

/** Libellé lisible pour une valeur de tonalité. */
export function formatPitchLabel(pitch: number): string {
  const p = clamp(pitch, PITCH_MIN, PITCH_MAX, PITCH_NEUTRAL);
  if (p <= 0.45) return "Très grave";
  if (p <= 0.8) return "Grave";
  if (p >= 2.2) return "Très aiguë";
  if (p >= 1.45) return "Aiguë";
  return "Neutre";
}

/** Courbe agressive : pousse davantage vers les extrêmes perceptifs. */
function pitchCurve(t: number): number {
  return Math.pow(clamp(t, 0, 1, 0), 2);
}

/** Tonalité profil → pitch / rate / voix système pour la synthèse. */
function pitchToSpeechParams(profile: VoiceProfile): {
  utterPitch: number;
  utterRate: number;
  preferGraveVoice: boolean;
  preferAiguVoice: boolean;
} {
  const p = clamp(profile.pitch, PITCH_MIN, PITCH_MAX, PITCH_NEUTRAL);
  const baseRate = clamp(profile.rate, 0.5, 2, 1);

  if (Math.abs(p - PITCH_NEUTRAL) < 0.04) {
    return {
      utterPitch: PITCH_NEUTRAL,
      utterRate: baseRate,
      preferGraveVoice: false,
      preferAiguVoice: false,
    };
  }

  if (p < PITCH_NEUTRAL) {
    const t = pitchCurve((PITCH_NEUTRAL - p) / (PITCH_NEUTRAL - PITCH_MIN));
    return {
      utterPitch: PITCH_NEUTRAL - t * PITCH_NEUTRAL,
      utterRate: baseRate * (1 - t * 0.38),
      preferGraveVoice: t > 0.35,
      preferAiguVoice: false,
    };
  }

  const t = pitchCurve((p - PITCH_NEUTRAL) / (PITCH_MAX - PITCH_NEUTRAL));
  return {
    utterPitch: PITCH_NEUTRAL + t,
    utterRate: baseRate * (1 + t * 0.28),
    preferGraveVoice: false,
    preferAiguVoice: t > 0.35,
  };
}

function pickVoiceForPitch(
  profile: VoiceProfile,
  preferGrave: boolean,
  preferAigu: boolean,
): SpeechSynthesisVoice | null {
  if (isSharedVoiceId(profile.id)) {
    return pickVoice(profile.voiceURI, profile.id);
  }
  if (!preferGrave && !preferAigu) return pickVoice(profile.voiceURI, profile.id);

  const fr = listSystemVoices().filter((v) => v.lang?.toLowerCase().startsWith("fr"));
  if (fr.length === 0) return pickVoice(profile.voiceURI, profile.id);

  if (preferGrave) {
    const grave =
      fr.find((v) => /paul|henri|claude|gilles|marcel|yves|homme|male|david/i.test(v.name)) ??
      fr.find((v) => !/julie|hortense|denise|celeste|caroline|femme|female/i.test(v.name)) ??
      fr[0];
    return grave;
  }

  const aigu =
    fr.find((v) =>
      /julie|hortense|denise|celeste|caroline|claire|femme|female|elodie/i.test(v.name),
    ) ?? fr[fr.length - 1];
  return aigu;
}

function buildUtterance(text: string, profile: VoiceProfile): SpeechSynthesisUtterance | null {
  const clean = sanitizeForSpeech(text);
  if (!clean) return null;

  const tonal = pitchToSpeechParams(profile);

  const utter = new SpeechSynthesisUtterance(clean);
  utter.lang = "fr-FR";
  utter.pitch = clamp(tonal.utterPitch, 0, 2, PITCH_NEUTRAL);
  utter.rate = clamp(tonal.utterRate, 0.1, 10, 1);
  utter.volume = clamp(profile.volume, 0, 1, 1);
  const voice = pickVoiceForPitch(profile, tonal.preferGraveVoice, tonal.preferAiguVoice);
  if (voice) utter.voice = voice;
  return utter;
}

/** Lit le texte avec la voix active de l'utilisateur courant. */
export function speakLoggy(text: string, onEnd?: () => void): boolean {
  if (!isLoggyVoiceSupported() || !isLoggyVoiceEnabled()) {
    onEnd?.();
    return false;
  }
  return speakWithProfile(text, resolveActiveProfile(), onEnd);
}

/** Lit le texte avec un profil donné (prévisualisation / test). */
export function speakWithProfile(
  text: string,
  profile: VoiceProfile,
  onEnd?: () => void,
): boolean {
  if (!isLoggyVoiceSupported()) {
    onEnd?.();
    return false;
  }
  stopLoggyVoice();
  const utter = buildUtterance(text, profile);
  if (!utter) {
    onEnd?.();
    return false;
  }
  utter.onend = () => onEnd?.();
  utter.onerror = () => onEnd?.();
  window.speechSynthesis.speak(utter);
  return true;
}

export function warmUpLoggyVoices(): void {
  if (!isLoggyVoiceSupported()) return;
  window.speechSynthesis.getVoices();
}

/* ------------------------------------------------------------------ */
/* Analyse d'un enregistrement → paramètres de voix                    */
/* ------------------------------------------------------------------ */

export interface VoiceAnalysis {
  /** Hauteur estimée (Hz) ou null si indéterminée. */
  fundamentalHz: number | null;
  pitch: number;
  rate: number;
  volume: number;
}

/** Estime la fréquence fondamentale d'une trame par auto-corrélation. */
function detectPitchHz(frame: Float32Array, sampleRate: number): number | null {
  const size = frame.length;
  let rms = 0;
  for (let i = 0; i < size; i++) rms += frame[i] * frame[i];
  rms = Math.sqrt(rms / size);
  if (rms < 0.01) return null; // trame trop silencieuse

  const minLag = Math.floor(sampleRate / 400); // 400 Hz max
  const maxLag = Math.floor(sampleRate / 75); // 75 Hz min
  let bestLag = -1;
  let bestCorr = 0;
  for (let lag = minLag; lag <= maxLag; lag++) {
    let corr = 0;
    for (let i = 0; i < size - lag; i++) corr += frame[i] * frame[i + lag];
    if (corr > bestCorr) {
      bestCorr = corr;
      bestLag = lag;
    }
  }
  if (bestLag <= 0) return null;
  return sampleRate / bestLag;
}

/**
 * Analyse un enregistrement audio et en déduit des paramètres de voix
 * (hauteur, débit, volume). Tout est calculé localement via l'API Web Audio.
 */
export async function analyzeVoiceRecording(blob: Blob): Promise<VoiceAnalysis> {
  const AudioCtx =
    window.AudioContext ?? (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
  const ctx = new AudioCtx();
  try {
    const buffer = await ctx.decodeAudioData(await blob.arrayBuffer());
    const data = buffer.getChannelData(0);
    const sr = buffer.sampleRate;

    // Volume global (RMS) → mappe vers 0.6 → 1.
    let rms = 0;
    for (let i = 0; i < data.length; i++) rms += data[i] * data[i];
    rms = Math.sqrt(rms / Math.max(1, data.length));
    const volume = clamp(0.6 + rms * 4, 0.6, 1, 1);

    // Hauteur : médiane des F0 des trames voisées.
    const frameSize = 2048;
    const hop = 1024;
    const f0s: number[] = [];
    for (let start = 0; start + frameSize <= data.length; start += hop) {
      const hz = detectPitchHz(data.subarray(start, start + frameSize), sr);
      if (hz && hz >= 75 && hz <= 400) f0s.push(hz);
    }
    let fundamentalHz: number | null = null;
    let pitch = 1;
    if (f0s.length > 0) {
      f0s.sort((a, b) => a - b);
      fundamentalHz = f0s[Math.floor(f0s.length / 2)];
      // 170 Hz ≈ neutre. Voix grave → pitch < 1, voix aiguë → pitch > 1.
      pitch = clamp(fundamentalHz / 170, PITCH_MIN, PITCH_MAX, PITCH_NEUTRAL);
    }

    // Débit : nombre de pics d'énergie (≈ syllabes) par seconde.
    const envHop = Math.floor(sr * 0.02); // fenêtres de 20 ms
    const env: number[] = [];
    for (let start = 0; start + envHop <= data.length; start += envHop) {
      let e = 0;
      for (let i = start; i < start + envHop; i++) e += data[i] * data[i];
      env.push(Math.sqrt(e / envHop));
    }
    const envMax = env.reduce((m, v) => Math.max(m, v), 0) || 1;
    const threshold = envMax * 0.35;
    let peaks = 0;
    for (let i = 1; i < env.length - 1; i++) {
      if (env[i] > threshold && env[i] >= env[i - 1] && env[i] > env[i + 1]) peaks++;
    }
    const durationSec = buffer.duration || env.length * 0.02;
    let rate = 1;
    if (durationSec > 0.3 && peaks > 0) {
      const syllPerSec = peaks / durationSec;
      // ~4 syllabes/s ≈ débit normal (rate 1).
      rate = clamp(syllPerSec / 4, 0.7, 1.4, 1);
    }

    return { fundamentalHz, pitch, rate, volume };
  } finally {
    void ctx.close();
  }
}
