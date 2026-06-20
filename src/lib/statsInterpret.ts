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

function peakPoint(points: StatsInterpretPoint[]): StatsInterpretPoint | undefined {
  if (points.length === 0) return undefined;
  return points.reduce((best, p) => (p.value > best.value ? p : best), points[0]);
}

function adviceExploration(xLabel: string, yLabel: string): string {
  return `• Conseil — exploration : change l'abscisse (autre champ que « ${xLabel} ») ou l'ordonnée (« ${yLabel} » vs nombre d'enregistrements) pour confirmer si le signal se reproduit sous un autre angle.`;
}

function adviceVigilanceLastRepères(points: StatsInterpretPoint[]): string {
  const n = points.length;
  if (n < 3) {
    return "• Conseil — vigilance : avec peu de repères, évite une conclusion trop forte — enrichis la série avant de décider.";
  }
  const last = points[n - 1].value;
  const prev = points[n - 2].value;
  if (last < prev) {
    return `• Conseil — vigilance : les derniers repères (« ${points[n - 2].label} » puis « ${points[n - 1].label} ») montrent un repli — vérifie si c'est un retournement ou un palier temporaire.`;
  }
  if (last > prev) {
    return `• Conseil — vigilance : la fin de courbe (« ${points[n - 1].label} ») accélère encore — assure-toi que cette poussée est soutenable sur la durée.`;
  }
  return "• Conseil — vigilance : la courbe se stabilise sur les derniers repères — observe si un nouveau palier se forme avant d'agir.";
}

function adviceActionTopSegments(points: StatsInterpretPoint[], rising: boolean): string {
  if (rising) {
    const peak = peakPoint(points);
    const lastLabel = points[points.length - 1]?.label;
    if (peak && peak.label !== lastLabel) {
      return `• Conseil — action : le pic est sur « ${peak.label} » (${formatStatValue(peak.value)}) — cible ce segment en priorité et compare-le aux repères qui suivent pour capitaliser ou corriger.`;
    }
    return "• Conseil — action : identifie les repères où la croissance est la plus marquée et croise avec la liste filtrée sur ces valeurs pour agir concrètement.";
  }
  return "• Conseil — action : repère le repère où la baisse s'amorce le plus nettement et isole ces enregistrements pour comprendre la cause (qualité, délai, saisonnalité…).";
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
    const blocks: string[] = [];
    if (second && first.total > second.total * 1.15) {
      blocks.push(
        `• Mon avis final : « ${first.name} » prend l'avantage sur « ${second.name} » dans ce comparatif. L'écart est suffisant pour orienter l'analyse, mais les courbes peuvent encore converger ou diverger sur certains repères — ne généralise pas sans regarder les zones de croisement.`,
      );
      blocks.push(
        `• Conseil — comparaison : zoome sur les repères où « ${first.name} » et « ${second.name} » s'écartent le plus pour comprendre ce qui différencie les deux séries.`,
      );
      blocks.push(
        "• Conseil — exploration : ajoute une troisième entité ou change l'agrégat (somme vs nombre) pour voir si l'écart tient avec une autre mesure.",
      );
      blocks.push(
        `• Conseil — action : exporte la liste des repères dominants de « ${first.name} » et vérifie en fiche si un facteur commun explique la performance.`,
      );
    } else {
      blocks.push(
        "• Mon avis final : les séries restent proches ou se croisent — aucune ne s'impose clairement sur l'ensemble des repères. C'est souvent le signe que le regroupement actuel ne suffit pas à révéler un levier décisif, ou que les entités suivent la même dynamique.",
      );
      blocks.push(adviceExploration(payload.x_label, payload.y_label));
      blocks.push(
        "• Conseil — vigilance : un écart peut être masqué par l'agrégat global — teste un filtre date ou un champ catégoriel pour isoler un sous-groupe.",
      );
      blocks.push(
        "• Conseil — action : compare côte à côte les fiches des entités sur le repère le plus contrasté avant de choisir une priorité métier.",
      );
    }
    return blocks.join("\n\n");
  }

  const points = payload.series[0]?.points ?? [];
  if (points.length < 2) {
    return [
      "• Mon avis final : un seul repère est visible sur cette courbe — je peux lire une valeur ponctuelle, pas encore une tendance ni une comparaison fiable entre segments.",
      "• Conseil — exploration : ajoute des repères (autre champ en abscisse, période plus longue ou granularité plus fine).",
      "• Conseil — vigilance : ne base pas une décision métier sur ce seul point — complète la série ou change le type de graphique (barres par catégorie).",
      "• Conseil — action : une fois plus de données disponibles, réouvre l'analyse Loggy pour obtenir des paliers et variations exploitables.",
    ].join("\n\n");
  }

  const first = points[0].value;
  const last = points[points.length - 1].value;
  const delta = last - first;
  const blocks: string[] = [];

  if (valuesEqual(delta, 0)) {
    blocks.push(
      "• Mon avis final : la courbe est globalement plate entre le premier et le dernier repère — le volume ou la métrique reste stable sur cet axe. Cela peut être sain (régularité) ou masquer des contrastes locaux entre repères intermédiaires.",
    );
    blocks.push(adviceExploration(payload.x_label, payload.y_label));
    blocks.push(
      "• Conseil — vigilance : même sur une courbe plate, certains repères peuvent être atypiques — demande la liste des pics ou creux pour ne pas passer à côté d'un signal local.",
    );
    blocks.push(
      "• Conseil — action : si la stabilité est attendue, documente ce palier ; sinon, teste un regroupement temporel (mois, trimestre) pour révéler une saisonnalité.",
    );
  } else if (delta > 0) {
    blocks.push(
      "• Mon avis final : la tendance globale est à la hausse — la métrique progresse entre le début et la fin de la série. Les paliers stables et les phases de croissance se combinent : l'essentiel est de savoir si la dynamique récente confirme ou infirme cette progression.",
    );
    blocks.push(adviceVigilanceLastRepères(points));
    blocks.push(adviceActionTopSegments(points, true));
    blocks.push(adviceExploration(payload.x_label, payload.y_label));
  } else {
    blocks.push(
      "• Mon avis final : la tendance globale est à la baisse — la métrique recule entre le premier et le dernier repère. Identifie si ce recul est progressif ou concentré sur une zone précise : la réponse conditionne l'urgence de la réaction.",
    );
    blocks.push(adviceVigilanceLastRepères(points));
    blocks.push(adviceActionTopSegments(points, false));
    blocks.push(
      "• Conseil — exploration : croise avec une autre ordonnée (somme, moyenne) ou une entité liée pour voir si la baisse est générale ou isolée à ce regroupement.",
    );
  }

  return blocks.join("\n\n");
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

/** Analyse IA acceptée seulement si complète (sinon on garde le résumé déterministe). */
function isCompleteStatsAnalysis(text: string): boolean {
  const t = text.trim();
  if (t.length < 80 || !t.includes("Mon avis final")) return false;
  const lower = t.toLowerCase();
  const badTrailers = [
    " ce",
    " ce qui",
    " ce qui indique",
    " de ",
    " à ",
    " →",
    " indique",
    " autour de",
    " qui indique",
  ];
  if (badTrailers.some((s) => lower.endsWith(s))) return false;
  const last = t.slice(-1);
  return last === "." || last === "…" || last === "!" || last === "?";
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
    return isCompleteStatsAnalysis(enriched) ? enriched : null;
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
