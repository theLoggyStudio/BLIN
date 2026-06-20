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
    if text.chars().count() <= max_len {
        return text;
    }
    let partial: String = text.chars().take(max_len).collect();
    if let Some(pos) = partial.rfind("\n\n") {
        return partial[..pos].trim_end().to_string();
    }
    if let Some(pos) = partial.rfind(". ") {
        return partial[..pos + 1].to_string();
    }
    let mut out = partial;
    out.push_str("…");
    out
}

/// Une analyse incomplète (coupée par le modèle) ne doit pas remplacer le résumé déterministe.
fn is_complete_analysis(text: &str) -> bool {
    let t = text.trim();
    if t.chars().count() < 80 {
        return false;
    }
    if !t.contains("Mon avis final") {
        return false;
    }
    let lower = t.to_lowercase();
    for trailer in [
        " ce",
        " ce qui",
        " ce qui indique",
        " de ",
        " à ",
        " →",
        " indique",
        " autour de",
        " qui indique",
    ] {
        if lower.ends_with(trailer) {
            return false;
        }
    }
    let last = t.chars().last().unwrap_or('.');
    last == '.' || last == '…' || last == '!' || last == '?'
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

fn peak_point(points: &[StatsInterpretPoint]) -> Option<&StatsInterpretPoint> {
    points
        .iter()
        .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
}

fn advice_exploration(x_label: &str, y_label: &str) -> String {
    format!(
        "• Conseil — exploration : change l'abscisse (autre champ que « {x_label} ») ou l'ordonnée (« {y_label} » vs nombre d'enregistrements) pour confirmer si le signal se reproduit sous un autre angle."
    )
}

fn advice_vigilance_last_repères(points: &[StatsInterpretPoint]) -> String {
    let n = points.len();
    if n < 3 {
        return "• Conseil — vigilance : avec peu de repères, évite une conclusion trop forte — enrichis la série avant de décider.".into();
    }
    let last = points[n - 1].value;
    let prev = points[n - 2].value;
    if last < prev {
        format!(
            "• Conseil — vigilance : les derniers repères (« {} » puis « {} ») montrent un repli — vérifie si c'est un retournement ou un palier temporaire.",
            points[n - 2].label,
            points[n - 1].label
        )
    } else if last > prev {
        format!(
            "• Conseil — vigilance : la fin de courbe (« {} ») accélère encore — assure-toi que cette poussée est soutenable sur la durée.",
            points[n - 1].label
        )
    } else {
        format!(
            "• Conseil — vigilance : la courbe se stabilise sur les derniers repères — observe si un nouveau palier se forme avant d'agir."
        )
    }
}

fn advice_action_top_segments(points: &[StatsInterpretPoint], rising: bool) -> String {
    if rising {
        if let Some(peak) = peak_point(points) {
            if peak.label != points.last().map(|p| p.label.as_str()).unwrap_or("") {
                return format!(
                    "• Conseil — action : le pic est sur « {} » ({}) — cible ce segment en priorité et compare-le aux repères qui suivent pour capitaliser ou corriger.",
                    peak.label,
                    format_value(peak.value)
                );
            }
        }
        "• Conseil — action : identifie les repères où la croissance est la plus marquée et croise avec la liste filtrée sur ces valeurs pour agir concrètement.".into()
    } else {
        "• Conseil — action : repère le repère où la baisse s'amorce le plus nettement et isole ces enregistrements pour comprendre la cause (qualité, délai, saisonnalité…).".into()
    }
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

        let mut blocks = Vec::new();
        if leaders.len() >= 2 && leaders[0].1 > leaders[1].1 * 1.15 {
            blocks.push(format!(
                "• Mon avis final : « {} » prend l'avantage sur « {} » dans ce comparatif. \
                 L'écart est suffisant pour orienter l'analyse, mais les courbes peuvent encore converger ou diverger sur certains repères — ne généralise pas sans regarder les zones de croisement.",
                leaders[0].0,
                leaders[1].0
            ));
            blocks.push(format!(
                "• Conseil — comparaison : zoome sur les repères où « {} » et « {} » s'écartent le plus pour comprendre ce qui différencie les deux séries.",
                leaders[0].0,
                leaders[1].0
            ));
            blocks.push(format!(
                "• Conseil — exploration : ajoute une troisième entité ou change l'agrégat (somme vs nombre) pour voir si l'écart tient avec une autre mesure."
            ));
            blocks.push(format!(
                "• Conseil — action : exporte la liste des repères dominants de « {} » et vérifie en fiche si un facteur commun explique la performance.",
                leaders[0].0
            ));
        } else {
            blocks.push(
                "• Mon avis final : les séries restent proches ou se croisent — aucune ne s'impose clairement sur l'ensemble des repères. \
                 C'est souvent le signe que le regroupement actuel ne suffit pas à révéler un levier décisif, ou que les entités suivent la même dynamique."
                    .into(),
            );
            blocks.push(advice_exploration(&payload.x_label, &payload.y_label));
            blocks.push(
                "• Conseil — vigilance : un écart peut être masqué par l'agrégat global — teste un filtre date ou un champ catégoriel pour isoler un sous-groupe."
                    .into(),
            );
            blocks.push(
                "• Conseil — action : compare côte à côte les fiches des entités sur le repère le plus contrasté avant de choisir une priorité métier."
                    .into(),
            );
        }
        return blocks.join("\n\n");
    }

    let points = &payload.series[0].points;
    if points.len() < 2 {
        return format!(
            "• Mon avis final : un seul repère est visible sur cette courbe — je peux lire une valeur ponctuelle, pas encore une tendance ni une comparaison fiable entre segments.\n\n\
             • Conseil — exploration : ajoute des repères (autre champ en abscisse, période plus longue ou granularité plus fine).\n\n\
             • Conseil — vigilance : ne base pas une décision métier sur ce seul point — complète la série ou change le type de graphique (barres par catégorie).\n\n\
             • Conseil — action : une fois plus de données disponibles, réouvre l'analyse Loggy pour obtenir des paliers et variations exploitables."
        );
    }

    let first = points[0].value;
    let last = points[points.len() - 1].value;
    let delta = last - first;
    let mut blocks = Vec::new();

    if values_equal(delta, 0.0) {
        blocks.push(
            "• Mon avis final : la courbe est globalement plate entre le premier et le dernier repère — le volume ou la métrique reste stable sur cet axe. \
             Cela peut être sain (régularité) ou masquer des contrastes locaux entre repères intermédiaires."
                .into(),
        );
        blocks.push(advice_exploration(&payload.x_label, &payload.y_label));
        blocks.push(
            "• Conseil — vigilance : même sur une courbe plate, certains repères peuvent être atypiques — demande la liste des pics ou creux pour ne pas passer à côté d'un signal local."
                .into(),
        );
        blocks.push(
            "• Conseil — action : si la stabilité est attendue, documente ce palier ; sinon, teste un regroupement temporel (mois, trimestre) pour révéler une saisonnalité."
                .into(),
        );
    } else if delta > 0.0 {
        blocks.push(
            "• Mon avis final : la tendance globale est à la hausse — la métrique progresse entre le début et la fin de la série. \
                 Les paliers stables et les phases de croissance se combinent : l'essentiel est de savoir si la dynamique récente confirme ou infirme cette progression."
                .into(),
        );
        blocks.push(advice_vigilance_last_repères(points));
        blocks.push(advice_action_top_segments(points, true));
        blocks.push(advice_exploration(&payload.x_label, &payload.y_label));
    } else {
        blocks.push(
            "• Mon avis final : la tendance globale est à la baisse — la métrique recule entre le premier et le dernier repère. \
                 Identifie si ce recul est progressif ou concentré sur une zone précise : la réponse conditionne l'urgence de la réaction."
                .into(),
        );
        blocks.push(advice_vigilance_last_repères(points));
        blocks.push(advice_action_top_segments(points, false));
        blocks.push(
            "• Conseil — exploration : croise avec une autre ordonnée (somme, moyenne) ou une entité liée pour voir si la baisse est générale ou isolée à ce regroupement."
                .into(),
        );
    }

    blocks.join("\n\n")
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
    sanitize_text(&lines.join("\n\n"), 3000)
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
         - « • Mon avis final : … » — synthèse constructive en 2 à 4 phrases (lecture métier, nuance, ce que ça implique).\n\
         - Puis 2 à 3 lignes « • Conseil — exploration / vigilance / action / comparaison : … » avec des conseils PRATIQUES et DIVERS (changer abscisse, filtrer, exporter une liste, surveiller les derniers repères, comparer une autre entité…).\n\
         Règles : cite libellés et valeurs réels ; n'invente rien ; pas de JSON ; français parlé, première personne ; viser 10 à 18 lignes au total ; termine par un point."
    );
    let user = format!(
        "Données du graphique (repères numérotés dans l'ordre de l'abscisse) :\n{data_summary}\n\n\
         Rédige ton analyse : paliers + variations en puces •, puis un avis final constructif et développé, puis plusieurs conseils divers."
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
        1024,
    ) {
        Ok(raw) => {
            let out = sanitize_text(&raw, 3000);
            if is_complete_analysis(&out) {
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
    _initial_analysis: &str,
) -> String {
    try_deterministic_stats_chat(chart, question).unwrap_or_else(|| {
        format!(
            "Je n'ai pas de réponse précise pour « {} » sur cette courbe. \
             Essaie par exemple : « liste les repères stables », « où est la croissance ? » ou « quel est le pic ? ».",
            question.trim()
        )
    })
}

fn normalize_question(q: &str) -> String {
    q.to_lowercase()
        .replace('à', "a")
        .replace('á', "a")
        .replace('â', "a")
        .replace('ä', "a")
        .replace('é', "e")
        .replace('è', "e")
        .replace('ê', "e")
        .replace('ë', "e")
        .replace('ù', "u")
        .replace('û', "u")
        .replace('ü', "u")
        .replace('ô', "o")
        .replace('ö', "o")
        .replace('î', "i")
        .replace('ï', "i")
        .replace('ç', "c")
}

fn question_matches(q: &str, needles: &[&str]) -> bool {
    let n = normalize_question(q);
    needles.iter().any(|k| n.contains(k))
}

fn is_stable_plateau_bullet(b: &str) -> bool {
    b.contains("stable") || b.contains("se maintient") || b.contains("sans variation notable")
}

fn is_variation_bullet(b: &str) -> bool {
    b.contains("croissance") || b.contains("baisse")
}

fn series_bullets_filtered(
    chart: &StatsInterpretPayload,
    filter: impl Fn(&str) -> bool,
) -> Vec<String> {
    let multi = chart.series.len() > 1;
    let mut lines = Vec::new();
    for s in &chart.series {
        if s.points.is_empty() {
            continue;
        }
        let picked: Vec<String> = segment_curve_to_bullets(&s.points)
            .into_iter()
            .filter(|b| filter(b))
            .collect();
        if picked.is_empty() {
            continue;
        }
        if multi {
            lines.push(format!("• Courbe « {} » :", s.name));
        }
        lines.extend(picked);
    }
    lines
}

fn answer_stable_markers(chart: &StatsInterpretPayload) -> String {
    let lines = series_bullets_filtered(chart, is_stable_plateau_bullet);
    if lines.is_empty() {
        return "Je ne vois aucun palier stable net sur cette courbe — les valeurs varient entre la plupart des repères.".into();
    }
    let mut out = vec!["Voici les paliers stables que je repère sur ce graphique :".to_string()];
    out.extend(lines);
    out.join("\n\n")
}

fn answer_variation_markers(chart: &StatsInterpretPayload) -> String {
    let lines = series_bullets_filtered(chart, is_variation_bullet);
    if lines.is_empty() {
        return "Je ne distingue pas de tronçon de croissance ou de baisse marqué — la courbe est surtout stable.".into();
    }
    let mut out = vec!["Voici les variations notables sur ce graphique :".to_string()];
    out.extend(lines);
    out.join("\n\n")
}

fn answer_all_segments(chart: &StatsInterpretPayload) -> String {
    let multi = chart.series.len() > 1;
    let mut lines = vec!["Voici le découpage paliers / variations de cette courbe :".to_string()];
    for s in &chart.series {
        if s.points.is_empty() {
            continue;
        }
        if multi {
            lines.push(format!("• Courbe « {} » :", s.name));
        }
        lines.extend(segment_curve_to_bullets(&s.points));
    }
    if lines.len() == 1 {
        return "Je n'ai pas encore de repères à décrire sur ce graphique.".into();
    }
    lines.join("\n\n")
}

fn answer_peaks(chart: &StatsInterpretPayload, want_max: bool) -> String {
    let mut lines = Vec::new();
    for s in &chart.series {
        if s.points.is_empty() {
            continue;
        }
        let pick = if want_max {
            s.points
                .iter()
                .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
        } else {
            s.points
                .iter()
                .min_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
        };
        let Some(p) = pick else { continue };
        let label = if want_max { "pic" } else { "creux" };
        let prefix = if chart.series.len() > 1 {
            format!("• Courbe « {} » — ", s.name)
        } else {
            "• ".to_string()
        };
        lines.push(format!(
            "{prefix}Le {label} est « {} » avec {} sur l'ordonnée.",
            p.label,
            format_value(p.value)
        ));
    }
    if lines.is_empty() {
        return "Je n'ai pas de repère à comparer sur cette courbe.".into();
    }
    let lead = if want_max {
        "Voici les pics que je vois :"
    } else {
        "Voici les creux que je vois :"
    };
    std::iter::once(lead.to_string())
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Réponses instantanées pour questions courantes sur la courbe (sans LLM).
pub fn try_deterministic_stats_chat(
    chart: &StatsInterpretPayload,
    question: &str,
) -> Option<String> {
    let q = question.trim();
    if q.is_empty() {
        return None;
    }

    if question_matches(
        q,
        &[
            "stable",
            "palier",
            "plateau",
            "sans variation",
            "reperes stables",
            "repere stable",
            "rester",
        ],
    ) {
        return Some(answer_stable_markers(chart));
    }
    if question_matches(
        q,
        &[
            "variation",
            "croissance",
            "hausse",
            "monte",
            "baisse",
            "descend",
            "repli",
            "augment",
        ],
    ) {
        return Some(answer_variation_markers(chart));
    }
    if question_matches(
        q,
        &[
            "liste",
            "enumere",
            "rappelle",
            "resume",
            "quels repere",
            "tous les repere",
            "synthese",
            "decoupage",
        ],
    ) {
        return Some(answer_all_segments(chart));
    }
    if question_matches(q, &["pic", "maximum", "plus haut", "sommet", "max "]) {
        return Some(answer_peaks(chart, true));
    }
    if question_matches(q, &["creux", "minimum", "plus bas", "min "]) {
        return Some(answer_peaks(chart, false));
    }
    None
}

/// Point d'entrée chat stats : déterministe d'abord, puis LLM si besoin.
pub fn stats_chat_answer(
    chart: &StatsInterpretPayload,
    initial_analysis: &str,
    message: &str,
    history: &[StatsChatTurn],
    app_name: &str,
) -> String {
    if let Some(det) = try_deterministic_stats_chat(chart, message) {
        return det;
    }
    if LlamaServer::model_ready() {
        return stats_chat_with_llm(chart, initial_analysis, message, history, app_name);
    }
    fallback_stats_chat_answer(chart, message, initial_analysis)
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

    match LlamaServer::chat_with_options(None, messages, 0.45, 512) {
        Ok(raw) => {
            let out = sanitize_text(&raw, 2000);
            if out.chars().count() >= 20 && (out.ends_with('.') || out.ends_with('…') || out.ends_with('!') || out.ends_with('?')) {
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
        assert!(out.contains("Conseil"));
        assert!(!out.contains("repère 1") || out.contains("Du repère"), "should group, not enumerate each point");
    }

    #[test]
    fn incomplete_analysis_rejected() {
        assert!(!is_complete_analysis(
            "• Du repère 5 au repère 8, on constate une croissance de 0 à 414, ce"
        ));
        assert!(is_complete_analysis(
            "J'analyse un graphique en courbe avec plusieurs paliers stables et une variation nette sur la fin.\n\n\
             • Palier stable.\n\n\
             • Mon avis final : la tendance est à la hausse avec une dynamique encourageante sur la fin de série.\n\n\
             • Conseil — action : priorise les repères les plus contrastés pour orienter la suite."
        ));
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

    #[test]
    fn deterministic_lists_stable_markers() {
        let mut points = Vec::new();
        for i in 0..10 {
            points.push(StatsInterpretPoint {
                label: format!("REF-{:02}", i + 1),
                value: 1.0,
            });
        }
        points.push(StatsInterpretPoint {
            label: "REF-11".into(),
            value: 3.0,
        });
        let chart = StatsInterpretPayload {
            chart_type: "line".into(),
            x_label: "Reference".into(),
            y_label: "Nombre".into(),
            series: vec![StatsInterpretSeries {
                name: "Vehicule".into(),
                entity_key: "article".into(),
                aggregate: "count".into(),
                group_by: "reference".into(),
                value_field: None,
                points,
            }],
        };
        let out = try_deterministic_stats_chat(&chart, "liste les repere stable").unwrap();
        assert!(out.contains("paliers stables"));
        assert!(out.contains("repère 1"));
        assert!(out.contains("repère 10"));
        assert!(!out.contains("d'après mon analyse"));
    }
}
