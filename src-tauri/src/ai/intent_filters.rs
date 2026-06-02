//! Détection d'intention (listes, entités, statuts) — français, synonymes et fautes courantes.

use chrono::Datelike;

/// Normalise pour la correspondance : minuscules, sans accents ni ponctuation parasite.
pub fn normalize_message(text: &str) -> String {
    let mut s = text.to_lowercase();
    const REPLACEMENTS: &[(&str, &str)] = &[
        ("à", "a"),
        ("â", "a"),
        ("ä", "a"),
        ("é", "e"),
        ("è", "e"),
        ("ê", "e"),
        ("ë", "e"),
        ("ï", "i"),
        ("î", "i"),
        ("ô", "o"),
        ("ö", "o"),
        ("ù", "u"),
        ("û", "u"),
        ("ü", "u"),
        ("ç", "c"),
        ("œ", "oe"),
        ("æ", "ae"),
        ("'", " "),
        ("’", " "),
        ("-", " "),
        ("_", " "),
    ];
    for (from, to) in REPLACEMENTS {
        s = s.replace(from, to);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn matches_any(normalized: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|k| normalized.contains(k))
}

// --- Verbes / tournures « afficher une liste » ---
const LIST_VERBS: &[&str] = &[
    "liste",
    "lister",
    "listez",
    "listes",
    "listeloi",
    "list moi",
    "listes moi",
    "list ",
    "list",
    "listing",
    "affiche",
    "afficher",
    "affichez",
    "affichage",
    "montre",
    "montrer",
    "montrez",
    "montre moi",
    "montre-moi",
    "donne",
    "donner",
    "donnez",
    "donne moi",
    "donne-moi",
    "donnes",
    "donnez moi",
    "voir",
    "vois",
    "voyez",
    "visualise",
    "visualiser",
    "visualisation",
    "consulte",
    "consulter",
    "consultez",
    "affiche la liste",
    "donne la liste",
    "quels sont",
    "quelles sont",
    "qu est ce que",
    "est ce que tu as",
    "as tu des",
    "y a t il des",
    "il y a des",
    "enumere",
    "enumerer",
    "enumeres",
    "recense",
    "recenser",
    "inventorie",
    "inventorier",
    "extrait",
    "extraire",
    "extrait moi",
    "sortir",
    "sors",
    "sors moi",
    "tableau",
    "table",
    "tabulate",
    "export",
    "affiche tout",
    "montre tout",
    "tous les",
    "toutes les",
    "ensemble des",
    "recherche les",
    "cherche les",
    "trouve les",
    "trouver les",
    "get ",
    "show ",
    "display ",
];

// --- Entités métier (ordre de priorité : plus spécifique d'abord) ---
const KEYWORDS_HANGARS: &[&str] = &[
    "hangar",
    "hangars",
    "entrepot",
    "entrepots",
    "depot",
    "depots",
    "stockage",
    "logistique",
    "zone alpha",
    "zone beta",
    "h-alpha",
    "h-beta",
];

const KEYWORDS_FINANCES: &[&str] = &[
    "finance",
    "finances",
    "financier",
    "financiere",
    "financieres",
    "ecriture",
    "ecritures",
    "ecriture comptable",
    "compta",
    "comptabilite",
    "comptable",
    "loyer",
    "loyers",
    "paiement",
    "paiements",
    "facture",
    "factures",
    "depense",
    "depenses",
    "recette",
    "recettes",
    "charge",
    "charges",
    "reglement",
    "reglements",
    "encaissement",
    "decaissement",
    "fin-",
];

const KEYWORDS_CONTRATS: &[&str] = &[
    "contrat",
    "contrats",
    "bail",
    "baux",
    "location",
    "locations",
    "locataire",
    "locataires",
    "preavis",
    "loyer mensuel",
];

const KEYWORDS_BIENS: &[&str] = &[
    "bien",
    "biens",
    "propriete",
    "proprietes",
    "patrimoine",
    "logement",
    "logements",
    "appartement",
    "appartements",
    "maison",
    "maisons",
    "immeuble",
    "immeubles",
    "terrain",
    "terrains",
    "local commercial",
    "local",
    "lot",
    "lots",
    "bien-",
    "patrimoine immobilier",
];

// --- Statuts finances ---
const FINANCE_EN_ATTENTE: &[&str] = &[
    "en attente",
    "en-attente",
    "attente",
    "pending",
    "a valider",
    "à valider",
    "a-valider",
    "a valide", // faute « à valider »
    "pas encore valide",
    "pas valide",
    "non valide",
    "non validee",
    "non validees",
    "non valide",
    "pas encore validee",
    "a traiter",
    "à traiter",
    "en cours",
    "en cours de validation",
    "ouvert",
    "ouverte",
    "ouvertes",
    "provisoire",
    "provisoires",
    "brouillon", // demande utilisateur floue
    "en suspens",
    "non regle",
    "non réglé",
    "impaye", // parfois confondu
    "a approuver",
    "à approuver",
];

const FINANCE_VALIDE: &[&str] = &[
    "valide",
    "validé",
    "validee",
    "validees",
    "valides",
    "validées",
    "valider", // « finances valider » = validées
    "validees",
    "approuve",
    "approuvee",
    "approuvees",
    "approuvé",
    "approuvée",
    "approuvées",
    "confirme",
    "confirmee",
    "confirmees",
    "confirmé",
    "confirmée",
    "confirmées",
    "accepte",
    "acceptee",
    "acceptees",
    "accepté",
    "acceptée",
    "acceptées",
    "regle",
    "réglé",
    "reglee",
    "réglée",
    "reglees",
    "réglées",
    "paye",
    "payé",
    "payee",
    "payée",
    "payees",
    "payées",
    "cloture",
    "clôturé",
    "cloturee",
    "clôturée",
    "clos",
    "close",
    "finalise",
    "finalisé",
    "finalisee",
    "finalisée",
    "ok",
    "termine",
    "terminé",
    "terminee",
    "terminée",
    "statut valide",
    "deja valide",
    "déjà valide",
];

// --- Statuts biens (hors hangar) ---
const BIEN_DISPONIBLE: &[&str] = &[
    "disponible",
    "disponibles",
    "libre",
    "libres",
    "vacant",
    "vacants",
    "inoccupe",
    "inoccupé",
    "a louer",
    "à louer",
    "en vente",
    "sur le marche",
];

const BIEN_LOUE: &[&str] = &[
    "loue",
    "loué",
    "louee",
    "louée",
    "loues",
    "loués",
    "louees",
    "louées",
    "loues",
    "occupe",
    "occupé",
    "rente",
    "renté",
    "loue actuellement",
    "en location",
];

const BIEN_MAINTENANCE: &[&str] = &[
    "maintenance",
    "en travaux",
    "travaux",
    "renovation",
    "rénovation",
    "indisponible",
];

const BIEN_VENDU: &[&str] = &[
    "vendu",
    "vendue",
    "vendus",
    "vendues",
    "cede",
    "cédé",
    "cedee",
    "cédée",
];

// --- Statuts contrats ---
const CONTRAT_BROUILLON: &[&str] = &[
    "brouillon",
    "brouillons",
    "draft",
    "non signe",
    "non signé",
    "pas signe",
    "pas signé",
    "en preparation",
    "en préparation",
];

const CONTRAT_ACTIF: &[&str] = &[
    "actif",
    "actifs",
    "active",
    "actives",
    "en cours",
    "signe",
    "signé",
    "signee",
    "signée",
    "signes",
    "signés",
    "signees",
    "signées",
    "valide",
    "validé",
    "en vigueur",
    "vigueur",
    "en cours de location",
];

// --- Statuts hangars ---
const HANGAR_LIBRE: &[&str] = &[
    "libre",
    "libres",
    "disponible",
    "disponibles",
    "vacant",
    "inoccupe",
    "inoccupé",
    "vide",
    "vides",
];

const HANGAR_OCCUPE: &[&str] = &[
    "occupe",
    "occupé",
    "occupee",
    "occupée",
    "occupees",
    "occupées",
    "occupees",
    "plein",
    "pleine",
    "pleins",
    "rempli",
    "remplie",
    "utilise",
    "utilisé",
];

const HANGAR_MAINTENANCE: &[&str] = &[
    "maintenance",
    "en travaux",
    "travaux",
    "reparation",
    "réparation",
    "indisponible",
];

pub fn wants_list_intent(lower: &str) -> bool {
    let n = normalize_message(lower);
    matches_any(&n, LIST_VERBS)
}

// --- Création / écriture ---
const CREATE_VERBS: &[&str] = &[
    "cree",
    "creer",
    "crée",
    "créer",
    "creez",
    "créez",
    "crees",
    "ajoute",
    "ajouter",
    "ajoutez",
    "ajoutes",
    "enregistre",
    "enregistrer",
    "enregistrez",
    "insere",
    "insérer",
    "inserer",
    "insert",
    "nouveau",
    "nouvelle",
    "nouveaux",
    "nouvelles",
    "creer un",
    "créer un",
    "cree un",
    "crée un",
    "cree moi",
    "crée moi",
    "creer moi",
    "ajoute moi",
    "enregistre moi",
    "fabrique",
    "genere",
    "génère",
    "create",
    "add ",
];

/// Verbe de création sans filtre entité métier (tableau de bord, brouillon modal).
pub fn wants_create_verb(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, CREATE_VERBS)
}

pub fn wants_create_intent(message: &str) -> bool {
    let n = normalize_message(message);
    if !matches_any(&n, CREATE_VERBS) {
        return false;
    }
    n.contains("bien")
        || n.contains("immeuble")
        || n.contains("appartement")
        || n.contains("maison")
        || n.contains("hangar")
        || n.contains("contrat")
        || n.contains("reference")
        || n.contains("ref ")
        || message.contains('{')
}

const EXPORT_KEYWORDS: &[&str] = &[
    "pdf",
    "export",
    "exporte",
    "exporter",
    "exportez",
    "rapport",
    "fiche",
    "document",
    "imprime",
    "imprimer",
    "impression",
    "telecharge",
    "telecharger",
    "telechargez",
    "genere un pdf",
    "génère un pdf",
    "cree un pdf",
    "crée un pdf",
    "fais un pdf",
    "fait un pdf",
    "html",
    "latex",
    "print",
];

const GENERATE_LOYERS_KEYWORDS: &[&str] = &[
    "generer les loyers",
    "générer les loyers",
    "genere les loyers",
    "génère les loyers",
    "loyers du mois",
    "paiements mensuels",
    "paiement mensuel",
    "echeances du mois",
    "échéances du mois",
    "facturer le mois",
    "facturation mensuelle",
];

const DETAIL_KEYWORDS: &[&str] = &[
    "dernier bien",
    "derniere bien",
    "dernier immeuble",
    "fiche bien",
    "detail bien",
    "détail bien",
    "infos sur le bien",
    "information sur le bien",
    "qu est ce que le bien",
];

const PAY_KEYWORDS: &[&str] = &[
    "marquer paye",
    "marquer payé",
    "marque paye",
    "valider le loyer",
    "valider la finance",
    "encaisser",
    "paiement recu",
    "paiement reçu",
];

const SEARCH_KEYWORDS: &[&str] = &[
    "cherche",
    "chercher",
    "recherche",
    "rechercher",
    "trouve",
    "trouver",
    "ou est",
    "où est",
];

pub fn wants_export_intent(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, EXPORT_KEYWORDS)
        || (n.contains("pdf") && (n.contains("bien") || n.contains("contrat") || n.contains("loyer")))
}

pub fn wants_generate_loyers_intent(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, GENERATE_LOYERS_KEYWORDS)
        || (n.contains("gener") && n.contains("loyer"))
        || (n.contains("génér") && n.contains("loyer"))
}

pub fn wants_detail_intent(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, DETAIL_KEYWORDS)
        || (n.contains("dernier") && n.contains("bien"))
        || (n.contains("derniere") && n.contains("bien"))
}

pub fn wants_pay_finance_intent(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, PAY_KEYWORDS)
        || (n.contains("marquer") && n.contains("pay"))
}

pub fn wants_search_intent(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, SEARCH_KEYWORDS) && (n.contains("bien") || n.contains("adresse") || n.contains("reference"))
}

const INTERNET_RESEARCH_KEYWORDS: &[&str] = &[
    "sur internet",
    "sur le web",
    "sur le net",
    "en ligne",
    "recherche web",
    "recherche internet",
    "cherche sur internet",
    "cherche sur le web",
    "trouve sur internet",
    "google",
    "wikipedia",
    "actualite",
    "actualites",
    "meteo",
    "news",
    "qui est ",
    "qu est ce que",
    "c est quoi",
    "definition de",
    "combien font",
    "capitale de",
    "quelle heure",
    "quelle date",
    "heure actuelle",
    "donne l heure",
    "donne moi l heure",
    "peut tu me donner l heure",
    "peux tu me donner l heure",
];

const DATETIME_LOCAL_KEYWORDS: &[&str] = &[
    "quelle heure",
    "donne l heure",
    "donne moi l heure",
    "peut tu me donner l heure",
    "peux tu me donner l heure",
    "heure actuelle",
    "quelle date",
    "date du jour",
    "quel jour",
];

/// Heure / date : réponse locale instantanée (pas Internet).
pub fn wants_local_datetime_intent(message: &str) -> bool {
    let n = normalize_message(message);
    DATETIME_LOCAL_KEYWORDS.iter().any(|k| n.contains(k))
}

/// Question nécessitant une recherche Internet (hors base locale / entités).
pub fn wants_internet_research_intent(message: &str) -> bool {
    if wants_search_intent(message) {
        return false;
    }
    if wants_local_datetime_intent(message) {
        return false;
    }
    let n = normalize_message(message);
    if matches_any(&n, INTERNET_RESEARCH_KEYWORDS) {
        return true;
    }
    // Questions générales courtes sans mot-clé métier local
    let local_hints = [
        "bien", "hangar", "contrat", "finance", "loyer", "entite", "ecole", "classe", "eleve",
        "dda_", "liste les", "gerer ",
    ];
    if n.contains('?')
        && !local_hints.iter().any(|h| n.contains(h))
        && n.split_whitespace().count() >= 4
    {
        return true;
    }
    false
}

/// Extrait la requête web (sans préfixes « cherche sur internet »).
pub fn extract_web_search_query(message: &str) -> Option<String> {
    let mut s = message.trim().to_string();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_lowercase();
    const STRIP: &[&str] = &[
        "cherche sur internet",
        "cherche sur le web",
        "recherche sur internet",
        "recherche web",
        "recherche internet",
        "trouve sur internet",
        "peux tu chercher",
        "peux-tu chercher",
        "loggy cherche",
        "sur internet",
        "sur le web",
        "en ligne",
    ];
    for prefix in STRIP {
        if let Some(rest) = lower.strip_prefix(prefix) {
            s = rest.trim().to_string();
            break;
        }
        if let Some(pos) = lower.find(prefix) {
            let rest = message[pos + prefix.len()..].trim();
            if !rest.is_empty() {
                s = rest.to_string();
                break;
            }
        }
    }
    let s = s.trim_matches(|c| c == '?' || c == '!' || c == '.').trim();
    if s.len() < 2 {
        None
    } else {
        Some(s.to_string())
    }
}

pub fn wants_action_intent(message: &str) -> bool {
    wants_list_intent(message)
        || wants_create_intent(message)
        || wants_update_intent(message)
        || wants_delete_intent(message)
        || wants_export_intent(message)
        || wants_generate_loyers_intent(message)
        || wants_detail_intent(message)
        || wants_pay_finance_intent(message)
        || wants_search_intent(message)
}

pub fn infer_export_tool(message: &str) -> Option<&'static str> {
    let n = normalize_message(message);
    if matches_any(&n, KEYWORDS_FINANCES) || n.contains("loyer") && n.contains("mois") {
        return Some("export_finances_month");
    }
    if matches_any(&n, KEYWORDS_CONTRATS) {
        return Some("export_bien_report");
    }
    Some("export_bien_report")
}

pub fn wants_last_bien(message: &str) -> bool {
    let n = normalize_message(message);
    n.contains("dernier") || n.contains("derniere") || n.contains("last") || n.contains("recent")
}

pub fn infer_period_from_message(message: &str) -> Option<(i32, u32)> {
    let n = normalize_message(message);
    let now = chrono::Local::now();
    let mut year = now.year();
    let mut month = now.month();

    const MOIS: [(&str, u32); 12] = [
        ("janvier", 1),
        ("fevrier", 2),
        ("mars", 3),
        ("avril", 4),
        ("mai", 5),
        ("juin", 6),
        ("juillet", 7),
        ("aout", 8),
        ("septembre", 9),
        ("octobre", 10),
        ("novembre", 11),
        ("decembre", 12),
    ];
    for (name, m) in MOIS {
        if n.contains(name) {
            month = m;
            break;
        }
    }
    if let Some(y) = extract_year_token(&n) {
        year = y;
    }
    Some((year, month))
}

fn extract_year_token(n: &str) -> Option<i32> {
    for word in n.split_whitespace() {
        if word.len() == 4 {
            if let Ok(y) = word.parse::<i32>() {
                if (2020..=2100).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    None
}

pub fn mois_label_fr(mois: u32) -> &'static str {
    match mois {
        1 => "janvier",
        2 => "février",
        3 => "mars",
        4 => "avril",
        5 => "mai",
        6 => "juin",
        7 => "juillet",
        8 => "août",
        9 => "septembre",
        10 => "octobre",
        11 => "novembre",
        12 => "décembre",
        _ => "mois",
    }
}

const DELETE_VERBS: &[&str] = &[
    "supprime",
    "supprimer",
    "supprimez",
    "efface",
    "effacer",
    "effacez",
    "delete",
    "retire",
    "retirer",
    "retirez",
    "enleve",
    "enlever",
    "enlevez",
    "detruit",
    "detruire",
    "detruisez",
    "virer",
    "vire",
    "retirer le",
    "supprimer le",
];

pub fn wants_delete_intent(message: &str) -> bool {
    let n = normalize_message(message);
    if !matches_any(&n, DELETE_VERBS) {
        return false;
    }
    n.contains("bien")
        || n.contains("hangar")
        || n.contains("immeuble")
        || n.contains("appartement")
        || n.contains("reference")
        || message.contains('{')
}

const DELETE_ALL_MARKERS: &[&str] = &[
    "tout",
    "tous",
    "toutes",
    "all",
    "ensemble",
    "integral",
    "integralement",
    "globalement",
];

/// « supprime tous les biens », « efface tout les bien », etc.
pub fn wants_delete_all_biens_intent(message: &str) -> bool {
    if !wants_delete_intent(message) || message_targets_hangar(message) {
        return false;
    }
    let n = normalize_message(message);
    if !matches_any(&n, DELETE_ALL_MARKERS) {
        return false;
    }
    n.contains("bien")
        || n.contains("immeuble")
        || n.contains("appartement")
        || n.contains("logement")
}

pub fn message_targets_hangar(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, KEYWORDS_HANGARS)
}

pub fn message_targets_contrat(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, KEYWORDS_CONTRATS)
}

pub fn message_targets_finance(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, KEYWORDS_FINANCES)
}

const UPDATE_VERBS: &[&str] = &[
    "modifie",
    "modifier",
    "modifiez",
    "met a jour",
    "mettre a jour",
    "mets a jour",
    "maj",
    "update",
    "change",
    "changer",
    "changez",
    "corrige",
    "corriger",
    "edite",
    "editer",
    "édite",
    "éditer",
    "actualise",
    "actualiser",
];

pub fn wants_update_intent(message: &str) -> bool {
    let n = normalize_message(message);
    matches_any(&n, UPDATE_VERBS)
}

pub fn infer_list_tool(lower: &str) -> Option<&'static str> {
    let n = normalize_message(lower);
    if matches_any(&n, KEYWORDS_HANGARS) {
        return Some("list_hangars");
    }
    if matches_any(&n, KEYWORDS_FINANCES) {
        return Some("list_finances");
    }
    if matches_any(&n, KEYWORDS_CONTRATS) {
        return Some("list_contrats");
    }
    if matches_any(&n, KEYWORDS_BIENS) {
        return Some("list_biens");
    }
    None
}

pub fn infer_statut_param(tool: &str, lower: &str) -> Option<String> {
    let n = normalize_message(lower);
    match tool {
        "list_finances" => infer_finance_statut(&n),
        "list_biens" => infer_bien_statut(&n),
        "list_contrats" => infer_contrat_statut(&n),
        "list_hangars" => infer_hangar_statut(&n),
        _ => None,
    }
}

fn infer_finance_statut(n: &str) -> Option<String> {
    let pending = matches_any(n, FINANCE_EN_ATTENTE);
    let validated = matches_any(n, FINANCE_VALIDE);

    if pending && !validated {
        return Some("en_attente".into());
    }
    if validated && !pending {
        return Some("valide".into());
    }
    if validated && pending {
        // « non valide » déjà dans pending ; « valide » seul gagne si « validé » explicite
        if n.contains("non valide") || n.contains("pas valide") || n.contains("a valider") {
            return Some("en_attente".into());
        }
        return Some("valide".into());
    }
    // « finances valider » sans « a valider »
    if n.contains("finance") && n.contains("valider") && !n.contains("a valider") {
        return Some("valide".into());
    }
    None
}

fn infer_bien_statut(n: &str) -> Option<String> {
    if matches_any(n, BIEN_MAINTENANCE) {
        return Some("maintenance".into());
    }
    if matches_any(n, BIEN_VENDU) {
        return Some("vendu".into());
    }
    if matches_any(n, BIEN_LOUE) {
        return Some("loué".into());
    }
    if matches_any(n, BIEN_DISPONIBLE) {
        return Some("disponible".into());
    }
    None
}

fn infer_contrat_statut(n: &str) -> Option<String> {
    if matches_any(n, CONTRAT_BROUILLON) {
        return Some("brouillon".into());
    }
    if matches_any(n, CONTRAT_ACTIF) {
        return Some("actif".into());
    }
    None
}

fn infer_hangar_statut(n: &str) -> Option<String> {
    if matches_any(n, HANGAR_MAINTENANCE) {
        return Some("maintenance".into());
    }
    if matches_any(n, HANGAR_OCCUPE) {
        return Some("occupé".into());
    }
    if matches_any(n, HANGAR_LIBRE) {
        return Some("libre".into());
    }
    None
}

pub fn statut_label(statut: &str) -> &str {
    match statut {
        "valide" => "validées",
        "en_attente" => "en attente",
        "disponible" => "disponibles",
        "loué" => "loués",
        "maintenance" => "en maintenance",
        "vendu" => "vendus",
        "actif" => "actifs",
        "brouillon" => "brouillons",
        "libre" => "libres",
        "occupé" => "occupés",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listeloi_finances_valider() {
        let n = normalize_message("listeloi les finance valider");
        assert!(wants_list_intent(&n));
        assert_eq!(infer_list_tool(&n), Some("list_finances"));
        assert_eq!(infer_statut_param("list_finances", &n), Some("valide".into()));
    }

    #[test]
    fn create_immeuble_json() {
        let msg = r#"cree moi ce bien { "reference": "IMM-TEST-02", "adresse": "5 place Bellecour, Lyon", "type_bien": "immeuble" }"#;
        assert!(wants_create_intent(msg));
    }

    #[test]
    fn finances_en_attente() {
        let n = normalize_message("montre les écritures en attente");
        assert_eq!(infer_statut_param("list_finances", &n), Some("en_attente".into()));
    }

    #[test]
    fn greeting_bonjour_pure() {
        assert!(crate::ai::greetings::wants_greeting_intent("Bonjour !"));
        assert!(!crate::ai::greetings::wants_greeting_intent(
            "Bonjour, liste les biens disponibles"
        ));
    }

    #[test]
    fn hangars_libres() {
        let n = normalize_message("quels hangars sont vacants ?");
        assert_eq!(infer_list_tool(&n), Some("list_hangars"));
        assert_eq!(infer_statut_param("list_hangars", &n), Some("libre".into()));
    }
}
