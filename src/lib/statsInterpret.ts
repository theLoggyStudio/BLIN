import { invoke } from "@tauri-apps/api/core";
import type { StatChartType } from "@/items/StatChart";
import type { StatAggregate } from "@/lib/entityStats";

export interface StatsInterpretPoint {
  label: string;
  value: number;
}

export interface StatsInterpretSeries {
  name: string;
  entity_key: string;
  aggregate: StatAggregate;
  group_by: string;
  value_field?: string | null;
  points: StatsInterpretPoint[];
}

export interface StatsInterpretPayload {
  chart_type: StatChartType;
  x_label: string;
  y_label: string;
  series: StatsInterpretSeries[];
}

const FLAT_EPS = 0.0001;

export function formatStatValue(v: number): string {
  if (Number.isInteger(v) || Math.abs(v - Math.round(v)) < 0.0001) {
    return String(Math.round(v));
  }
  return v.toFixed(2);
}

function valuesEqual(a: number, b: number): boolean {
  return Math.abs(a - b) < FLAT_EPS;
}

function interpretVariation(from: number, to: number, rising: boolean): string {
  if (valuesEqual(from, to)) return "une continuité sans écart significatif";
  const pct = from !== 0 ? Math.abs(((to - from) / from) * 100) : null;
  if (rising) {
    if (pct != null && pct >= 30) return "une dynamique en nette accélération sur cette période";
    return "une progression mesurée qu'il convient de surveiller";
  }
  if (pct != null && pct >= 30) return "un repli important méritant une analyse plus fine";
  return "un ajustement à la baisse, sans rupture brutale pour l'instant";
}

/** Regroupe la courbe en paliers stables et tronçons de variation (pas point par point). */
export function segmentCurveToBullets(points: StatsInterpretPoint[]): string[] {
  if (points.length === 0) return [];
  if (points.length === 1) {
    return [
      `• Au repère 1 (« ${points[0].label} »), la courbe se situe à ${formatStatValue(points[0].value)}, point de référence unique.`,
    ];
  }

  const allFlat = points.every((p) => valuesEqual(p.value, points[0].value));
  if (allFlat) {
    return [
      `• Du repère 1 au repère ${points.length}, la courbe reste stable à ${formatStatValue(points[0].value)}, sans variation notable sur l'ensemble des repères.`,
    ];
  }

  const bullets: string[] = [];
  let idx = 0;

  while (idx < points.length) {
    let end = idx;
    while (end + 1 < points.length && valuesEqual(points[end].value, points[end + 1].value)) {
      end++;
    }

    if (end > idx) {
      const r1 = idx + 1;
      const r2 = end + 1;
      const v = formatStatValue(points[idx].value);
      if (r1 === r2) {
        bullets.push(
          `• Au repère ${r1} (« ${points[idx].label} »), la courbe se maintient à ${v}.`,
        );
      } else {
        bullets.push(
          `• Du repère ${r1} au repère ${r2}, la courbe reste stable autour de ${v}, ce qui indique un palier sans variation notable.`,
        );
      }
      idx = end;
      if (idx >= points.length - 1) {
        idx++;
        break;
      }
      idx++;
      continue;
    }

    if (idx + 1 >= points.length) {
      break;
    }

    const rising = points[idx + 1].value > points[idx].value;
    const falling = points[idx + 1].value < points[idx].value;
    if (!rising && !falling) {
      idx++;
      continue;
    }

    let vEnd = idx + 1;
    while (vEnd + 1 < points.length) {
      const d = points[vEnd + 1].value - points[vEnd].value;
      if (valuesEqual(d, 0)) break;
      if (rising && d < 0) break;
      if (falling && d > 0) break;
      vEnd++;
    }

    const fromV = points[idx].value;
    const toV = points[vEnd].value;
    if (!valuesEqual(fromV, toV)) {
      const label = rising ? "croissance" : "baisse";
      const r1 = idx + 1;
      const r2 = vEnd + 1;
      bullets.push(
        `• Du repère ${r1} au repère ${r2} (« ${points[idx].label} » → « ${points[vEnd].label} »), on constate une ${label} de ${formatStatValue(fromV)} à ${formatStatValue(toV)}, ce qui indique ${interpretVariation(fromV, toV, rising)}.`,
      );
    }

    idx = vEnd;
    if (idx < points.length - 1) {
      idx++;
    } else {
      idx++;
    }
  }

  return bullets;
}

function finalOpinion(payload: StatsInterpretPayload): string {
  const multi = payload.series.length > 1;
  if (multi) {
    const leaders = payload.series
      .map((s) => {
        const total = s.points.reduce((sum, p) => sum + p.value, 0);
        return { name: s.name, total };
      })
      .sort((a, b) => b.total - a.total);
    const first = leaders[0];
    const second = leaders[1];
    if (second && first.total > second.total * 1.15) {
      return `• Mon avis final : « ${first.name} » domine le comparatif — creuse surtout les repères où les courbes divergent le plus.`;
    }
    return "• Mon avis final : les séries restent proches — affine le regroupement (abscisse) ou compare un champ numérique précis pour faire ressortir un écart net.";
  }

  const points = payload.series[0]?.points ?? [];
  if (points.length < 2) {
    return "• Mon avis final : un seul repère visible — ajoute de la profondeur temporelle ou catégorielle pour lire une tendance.";
  }
  const first = points[0].value;
  const last = points[points.length - 1].value;
  const delta = last - first;
  if (valuesEqual(delta, 0)) {
    return "• Mon avis final : la courbe est globalement plate — cherche un autre découpage ou une autre ordonnée pour révéler un signal.";
  }
  if (delta > 0) {
    return "• Mon avis final : la tendance globale est à la hausse — vérifie si cette progression se maintient sur les derniers repères ou si un ralentissement apparaît.";
  }
  return "• Mon avis final : la tendance globale est à la baisse — identifie à quel repère le recul s'accélère pour prioriser une action.";
}

/** Résumé instantané : paliers + variations uniquement, chaque étape en puce •. */
export function fallbackStatsInterpretation(payload: StatsInterpretPayload): string {
  if (payload.series.length === 0 || payload.series.every((s) => s.points.length === 0)) {
    return "Je n'ai pas encore de données à commenter sur ce graphique.";
  }

  const multi = payload.series.length > 1;
  const lines: string[] = [
    `J'analyse un graphique en ${payload.chart_type} : « ${payload.x_label} » en abscisse, « ${payload.y_label} » en ordonnée.`,
  ];

  for (const s of payload.series) {
    if (s.points.length === 0) {
      lines.push(`• Pour « ${s.name} », je ne vois aucun repère sur la courbe.`);
      continue;
    }

    if (multi) {
      lines.push(`• Courbe « ${s.name} » :`);
    }

    lines.push(...segmentCurveToBullets(s.points));
  }

  lines.push(finalOpinion(payload));
  return lines.join("\n\n");
}

/** Enrichissement IA en arrière-plan — thread Rust dédié, n'attend pas le graphique. */
export async function enrichStatsInterpretationWithAi(
  payload: StatsInterpretPayload,
): Promise<string | null> {
  await new Promise<void>((resolve) => {
    window.setTimeout(resolve, 0);
  });
  try {
    const text = await invoke<string>("ai_stats_interpret", { payload });
    const enriched = text.trim();
    return enriched.length >= 80 ? enriched : null;
  } catch {
    return null;
  }
}

export interface StatsChatTurn {
  role: "user" | "assistant";
  content: string;
}

/** Question de suivi sur la courbe affichée (contexte graphique exclusif). */
export async function askStatsLoggyQuestion(
  chart: StatsInterpretPayload,
  initialAnalysis: string,
  message: string,
  history: StatsChatTurn[],
): Promise<string> {
  return invoke<string>("ai_stats_chat", {
    payload: {
      chart,
      initial_analysis: initialAnalysis,
      message,
      history,
    },
  });
}
