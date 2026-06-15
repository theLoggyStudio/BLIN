use serde::Deserialize;

use crate::ai::llama_server::{ChatMessage, LlamaServer};

#[derive(Debug, Clone, Deserialize)]
pub struct StatsInterpretPoint {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Deserialize)]
pub struct StatsInterpretSeries {
    pub name: String,
    pub entity_key: String,
    pub aggregate: String,
    pub group_by: String,
    pub value_field: Option<String>,
    pub points: Vec<StatsInterpretPoint>,
}

#[derive(Debug, Deserialize)]
pub struct StatsInterpretPayload {
    pub chart_type: String,
    pub x_label: String,
    pub y_label: String,
    pub series: Vec<StatsInterpretSeries>,
}

const FLAT_EPS: f64 = 0.0001;

fn sanitize_text(raw: &str, max_len: usize) -> String {
    let text: String = raw
        .trim()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if text.chars().count() > max_len {
        let mut out: String = text.chars().take(max_len).collect();
        out.push_str("…");
        out
    } else {
        text
    }
}

fn format_value(v: f64) -> String {
    if (v - v.round()).abs() < 0.0001 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
}

fn values_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < FLAT_EPS
}

fn interpret_variation(from: f64, to: f64, rising: bool) -> &'static str {
    if values_equal(from, to) {
        return "une continuité sans écart significatif";
    }
    let pct = if from != 0.0 {
        Some(((to - from) / from).abs() * 100.0)
    } else {
        None
    };
    if rising {
        if pct.is_some_and(|p| p >= 30.0) {
            "une dynamique en nette accélération sur cette période"
        } else {
            "une progression mesurée qu'il convient de surveiller"
        }
    } else if pct.is_some_and(|p| p >= 30.0) {
        "un repli important méritant une analyse plus fine"
    } else {
        "un ajustement à la baisse, sans rupture brutale pour l'instant"
    }
}

/// Paliers stables + tronçons de variation — pas point par point.
fn segment_curve_to_bullets(points: &[StatsInterpretPoint]) -> Vec<String> {
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![format!(
            "• Au repère 1 (« {} »), la courbe se situe à {}, point de référence unique.",
            points[0].label,
            format_value(points[0].value)
        )];
    }

    let all_flat = points.iter().all(|p| values_equal(p.value, points[0].value));
    if all_flat {
        return vec![format!(
            "• Du repère 1 au repère {}, la courbe reste stable à {}, sans variation notable sur l'ensemble des repères.",
            points.len(),
            format_value(points[0].value)
        )];
    }

    let mut bullets = Vec::new();
    let mut idx = 0usize;

    while idx < points.len() {
        let mut end = idx;
        while end + 1 < points.len() && values_equal(points[end].value, points[end + 1].value) {
            end += 1;
        }

        if end > idx {
            let r1 = idx + 1;
            let r2 = end + 1;
            let v = format_value(points[idx].value);
            if r1 == r2 {
                bullets.push(format!(
                    "• Au repère {r1} (« {} »), la courbe se maintient à {v}.",
                    points[idx].label
                ));
            } else {
                bullets.push(format!(
                    "• Du repère {r1} au repère {r2}, la courbe reste stable autour de {v}, ce qui indique un palier sans variation notable."
                ));
            }
            idx = end;
            if idx >= points.len() - 1 {
                idx += 1;
                break;
            }
            idx += 1;
            continue;
        }

        if idx + 1 >= points.len() {
            break;
        }

        let rising = points[idx + 1].value > points[idx].value;
        let falling = points[idx + 1].value < points[idx].value;
        if !rising && !falling {
            idx += 1;
            continue;
        }

        let mut v_end = idx + 1;
        while v_end + 1 < points.len() {
            let d = points[v_end + 1].value - points[v_end].value;
            if values_equal(d, 0.0) {
                break;
            }
            if rising && d < 0.0 {
                break;
            }
            if falling && d > 0.0 {
                break;
            }
            v_end += 1;
        }

        let from_v = points[idx].value;
        let to_v = points[v_end].value;
        if !values_equal(from_v, to_v) {
            let label = if rising { "croissance" } else { "baisse" };
            let r1 = idx + 1;
            let r2 = v_end + 1;
            bullets.push(format!(
                "• Du repère {r1} au repère {r2} (« {} » → « {} »), on constate une {label} de {} à {}, ce qui indique {}.",
                points[idx].label,
                points[v_end].label,
                format_value(from_v),
                format_value(to_v),
                interpret_variation(from_v, to_v, rising)
            ));
        }

        idx = v_end;
        if idx < points.len() - 1 {
            idx += 1;
        } else {
            idx += 1;
        }
    }

    bullets
}

fn final_opinion(payload: &StatsInterpretPayload) -> String {
    let multi = payload.series.len() > 1;
    if multi {
        let mut leaders: Vec<(String, f64)> = payload
            .series
            .iter()
            .map(|s| {
                let total: f64 = s.points.iter().map(|p| p.value).sum();
                (s.name.clone(), total)
            })
            .collect();
        leaders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if leaders.len() >= 2 && leaders[0].1 > leaders[1].1 * 1.15 {
            return format!(
                "• Mon avis final : « {} » domine le comparatif — creuse surtout les repères où les courbes divergent le plus.",
                leaders[0].0
            );
        }
        return "• Mon avis final : les séries restent proches — affine le regroupement (abscisse) ou compare un champ numérique précis pour faire ressortir un écart net.".into();
    }

    let points = &payload.series[0].points;
    if points.len() < 2 {
        return "• Mon avis final : un seul repère visible — ajoute de la profondeur temporelle ou catégorielle pour lire une tendance.".into();
    }
    let first = points[0].value;
    let last = points[points.len() - 1].value;
    let delta = last - first;
    if values_equal(delta, 0.0) {
        return "• Mon avis final : la courbe est globalement plate — cherche un autre découpage ou une autre ordonnée pour révéler un signal.".into();
    }
    if delta > 0.0 {
        "• Mon avis final : la tendance globale est à la hausse — vérifie si cette progression se maintient sur les derniers repères ou si un ralentissement apparaît.".into()
    } else {
        "• Mon avis final : la tendance globale est à la baisse — identifie à quel repère le recul s'accélère pour prioriser une action.".into()
    }
}

/// Résumé déterministe si le modèle local est indisponible.
pub fn fallback_interpretation(payload: &StatsInterpretPayload) -> String {
    if payload.series.is_empty() || payload.series.iter().all(|s| s.points.is_empty()) {
        return "Je n'ai pas encore de données à commenter sur ce graphique.".into();
    }

    let multi = payload.series.len() > 1;
    let mut lines = vec![format!(
        "J'analyse un graphique en {} : « {} » en abscisse, « {} » en ordonnée.",
        payload.chart_type, payload.x_label, payload.y_label
    )];

    for s in &payload.series {
        if s.points.is_empty() {
            lines.push(format!("• Pour « {} », je ne vois aucun repère sur la courbe.", s.name));
            continue;
        }

        if multi {
            lines.push(format!("• Courbe « {} » :", s.name));
        }

        lines.extend(segment_curve_to_bullets(&s.points));
    }

    lines.push(final_opinion(payload));
    sanitize_text(&lines.join("\n\n"), 1400)
}

fn build_data_summary(payload: &StatsInterpretPayload) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Type : {}\nAbscisse : {}\nOrdonnée : {}",
        payload.chart_type, payload.x_label, payload.y_label
    ));
    for (si, s) in payload.series.iter().enumerate() {
        lines.push(format!("\nSérie {} « {} » ({})", si + 1, s.name, s.entity_key));
        lines.push(format!(
            "Agrégat : {} | Regroupement : {}",
            s.aggregate, s.group_by
        ));
        if let Some(vf) = &s.value_field {
            lines.push(format!("Champ valeur : {vf}"));
        }
        for (i, p) in s.points.iter().enumerate() {
            lines.push(format!(
                "Repère {} : {} → {}",
                i + 1,
                p.label,
                format_value(p.value)
            ));
        }
    }
    lines.join("\n")
}

/// Appel LLM uniquement — sans verrou SQLite (runtime déjà profilé).
pub fn interpret_stats_with_llm(
    payload: &StatsInterpretPayload,
    fallback: &str,
    app_name: &str,
) -> String {
    let data_summary = build_data_summary(payload);
    let system = format!(
        "Tu es Loggy, l'assistant IA de {app_name}. Tu commentes un graphique statistique pour ton collègue.\n\
         Règle essentielle : ne décris PAS chaque repère un par un. Parle uniquement des PALIERS (valeur stable sur plusieurs repères) \
         et des VARIATIONS (croissance ou baisse entre deux zones).\n\
         Format OBLIGATOIRE :\n\
         - Une phrase d'introduction (sans puce).\n\
         - Chaque étape d'analyse sur sa propre ligne, précédée d'un saut de ligne puis « • » (puce).\n\
         - Palier : « • Du repère X au repère Y, la courbe reste stable autour de [valeur], ce qui indique … »\n\
         - Variation : « • Du repère X au repère Y, on constate une croissance/baisse de [valeur départ] à [valeur fin], ce qui indique … »\n\
         - Dernière ligne : « • Mon avis final : … » (synthèse globale, comparaison des séries si plusieurs courbes).\n\
         Exemple : « • Du repère 1 au repère 8, la courbe reste stable à 56… » puis « • Du repère 9 au repère 15, on constate une croissance… »\n\
         Règles : cite libellés et valeurs réels ; n'invente rien ; pas de JSON ; français parlé, première personne ; 5 à 12 lignes."
    );
    let user = format!(
        "Données du graphique (repères numérotés dans l'ordre de l'abscisse) :\n{data_summary}\n\n\
         Rédige ton analyse : paliers + variations seulement, chaque étape en puce •, puis Mon avis final."
    );

    match LlamaServer::chat_with_options(
        None,
        vec![
            ChatMessage {
                role: "system".into(),
                content: system,
            },
            ChatMessage {
                role: "user".into(),
                content: user,
            },
        ],
        0.55,
        520,
    ) {
        Ok(raw) => {
            let out = sanitize_text(&raw, 1400);
            if out.len() >= 80 {
                out
            } else {
                fallback.to_string()
            }
        }
        Err(_) => fallback.to_string(),
    }
}

#[derive(Debug, Deserialize)]
pub struct StatsChatTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct StatsChatPayload {
    pub chart: StatsInterpretPayload,
    pub initial_analysis: String,
    pub message: String,
    #[serde(default)]
    pub history: Vec<StatsChatTurn>,
}

pub fn fallback_stats_chat_answer(
    chart: &StatsInterpretPayload,
    question: &str,
    initial_analysis: &str,
) -> String {
    let snippet: String = initial_analysis.chars().take(160).collect();
    format!(
        "Je ne réponds qu'aux questions liées à ce graphique (« {} » × « {} »). \
         Concernant « {} » : d'après mon analyse — {snippet}… \
         Peux-tu préciser ta question en restant sur cette courbe ?",
        chart.x_label, chart.y_label, question.trim()
    )
}

/// Réponses de suivi — uniquement sur les données du graphique affiché.
pub fn stats_chat_with_llm(
    chart: &StatsInterpretPayload,
    initial_analysis: &str,
    message: &str,
    history: &[StatsChatTurn],
    app_name: &str,
) -> String {
    let q = message.trim();
    if q.is_empty() {
        return "Pose-moi une question sur cette courbe (tendance, pic, comparaison des séries…).".into();
    }

    if !LlamaServer::model_ready() {
        return fallback_stats_chat_answer(chart, q, initial_analysis);
    }

    let data_summary = build_data_summary(chart);
    let system = format!(
        "Tu es Loggy, l'assistant IA de {app_name}. L'utilisateur consulte un graphique statistique \
         et te pose des questions de suivi.\n\
         Règles STRICTES :\n\
         - Réponds UNIQUEMENT à partir des données du graphique ci-dessous et de ton analyse initiale.\n\
         - Refuse poliment toute question hors sujet (autre entité, autre écran, configuration, blague…).\n\
         - Français parlé, première personne, concis (2 à 6 phrases).\n\
         - Pas de JSON ; tu peux utiliser des puces « • » sur des lignes séparées si utile.\n\
         - Cite des libellés et valeurs réels ; n'invente rien."
    );

    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: system,
    }];

    let context = format!(
        "Données du graphique :\n{data_summary}\n\n\
         Mon analyse initiale de cette courbe :\n{initial_analysis}"
    );
    messages.push(ChatMessage {
        role: "user".into(),
        content: context,
    });
    messages.push(ChatMessage {
        role: "assistant".into(),
        content: "J'ai bien noté les données et mon analyse. Pose ta question sur cette courbe.".into(),
    });

    for turn in history {
        let role = if turn.role == "assistant" {
            "assistant"
        } else {
            "user"
        };
        if turn.content.trim().is_empty() {
            continue;
        }
        messages.push(ChatMessage {
            role: role.into(),
            content: turn.content.clone(),
        });
    }

    messages.push(ChatMessage {
        role: "user".into(),
        content: q.into(),
    });

    match LlamaServer::chat_with_options(None, messages, 0.45, 380) {
        Ok(raw) => {
            let out = sanitize_text(&raw, 900);
            if out.len() >= 20 {
                out
            } else {
                fallback_stats_chat_answer(chart, q, initial_analysis)
            }
        }
        Err(_) => fallback_stats_chat_answer(chart, q, initial_analysis),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_uses_bullets_and_variations() {
        let payload = StatsInterpretPayload {
            chart_type: "line".into(),
            x_label: "Date".into(),
            y_label: "Nombre".into(),
            series: vec![StatsInterpretSeries {
                name: "Clients".into(),
                entity_key: "clients".into(),
                aggregate: "count".into(),
                group_by: "ville".into(),
                value_field: None,
                points: vec![
                    StatsInterpretPoint {
                        label: "Jan".into(),
                        value: 56.0,
                    },
                    StatsInterpretPoint {
                        label: "Fév".into(),
                        value: 56.0,
                    },
                    StatsInterpretPoint {
                        label: "Mar".into(),
                        value: 56.0,
                    },
                    StatsInterpretPoint {
                        label: "Avr".into(),
                        value: 70.0,
                    },
                    StatsInterpretPoint {
                        label: "Mai".into(),
                        value: 85.0,
                    },
                ],
            }],
        };
        let out = fallback_interpretation(&payload);
        assert!(out.contains('•'), "expected bullet: {out}");
        assert!(out.contains("stable"), "expected plateau: {out}");
        assert!(out.contains("croissance") || out.contains("baisse"), "expected variation: {out}");
        assert!(out.contains("Mon avis final"));
        assert!(!out.contains("repère 1") || out.contains("Du repère"), "should group, not enumerate each point");
    }

    #[test]
    fn flat_curve_single_bullet() {
        let points: Vec<StatsInterpretPoint> = (0..5)
            .map(|i| StatsInterpretPoint {
                label: format!("R{}", i + 1),
                value: 42.0,
            })
            .collect();
        let bullets = segment_curve_to_bullets(&points);
        assert_eq!(bullets.len(), 1);
        assert!(bullets[0].contains("stable"));
    }
}
