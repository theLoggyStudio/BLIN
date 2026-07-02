//! Analyse d'images (vision) — extraction JSON entités ou modèle d'impression.
//! Fournisseurs : OpenRouter (gratuit) ou Google Gemini. Rien n'est enregistré automatiquement.

use std::fs;
use std::path::Path;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::entity::registry;
use crate::entity::attr_types::is_reserved_attribute;

const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.0-flash";
const DEFAULT_OPENROUTER_MODEL: &str = "qwen/qwen2.5-vl-72b-instruct:free";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisionProvider {
    OpenRouter,
    Gemini,
}

impl VisionProvider {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "gemini" | "google" => Self::Gemini,
            _ => Self::OpenRouter,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::Gemini => "gemini",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::OpenRouter => "OpenRouter (gratuit)",
            Self::Gemini => "Google Gemini",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    /// Rétrocompatibilité anciennes configs.
    #[serde(default)]
    pub gemini_api_key: String,
    #[serde(default)]
    pub model: String,
}

fn default_provider() -> String {
    "openrouter".to_string()
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: String::new(),
            gemini_api_key: String::new(),
            model: DEFAULT_OPENROUTER_MODEL.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionConfigPublic {
    pub configured: bool,
    pub provider: String,
    pub provider_label: String,
    pub model: String,
    pub key_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisionMode {
    Registry,
    Print,
}

pub fn config_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("ai").join("vision.json")
}

fn resolved_api_key(cfg: &VisionConfig) -> String {
    let key = cfg.api_key.trim();
    if !key.is_empty() {
        return key.to_string();
    }
    cfg.gemini_api_key.trim().to_string()
}

fn resolved_model(cfg: &VisionConfig, provider: VisionProvider) -> String {
    let m = cfg.model.trim();
    if !m.is_empty() {
        return m.to_string();
    }
    match provider {
        VisionProvider::Gemini => DEFAULT_GEMINI_MODEL.to_string(),
        VisionProvider::OpenRouter => DEFAULT_OPENROUTER_MODEL.to_string(),
    }
}

pub fn load_config(data_dir: &Path) -> VisionConfig {
    let path = config_path(data_dir);
    if !path.is_file() {
        return VisionConfig::default();
    }
    let mut cfg: VisionConfig = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    if cfg.api_key.trim().is_empty() && !cfg.gemini_api_key.trim().is_empty() {
        cfg.api_key = cfg.gemini_api_key.trim().to_string();
        if cfg.provider.is_empty() {
            cfg.provider = "gemini".to_string();
        }
    }
    cfg
}

pub fn save_config(data_dir: &Path, cfg: &VisionConfig) -> Result<(), String> {
    let dir = data_dir.join("ai");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(data_dir), json).map_err(|e| e.to_string())
}

pub fn public_config(data_dir: &Path) -> VisionConfigPublic {
    let cfg = load_config(data_dir);
    let provider = VisionProvider::parse(&cfg.provider);
    let key = resolved_api_key(&cfg);
    VisionConfigPublic {
        configured: !key.is_empty(),
        provider: provider.as_str().to_string(),
        provider_label: provider.label().to_string(),
        model: resolved_model(&cfg, provider),
        key_hint: if key.len() >= 4 {
            Some(format!("…{}", &key[key.len().saturating_sub(4)..]))
        } else {
            None
        },
    }
}

fn detect_mode(message: &str) -> VisionMode {
    let m = message.to_lowercase();
    let print_score = [
        "impression", "imprimer", "print", "modèle d'impression", "modele d'impression",
        "html", "css", "template", "modèle html", "modele html",
    ]
    .iter()
    .filter(|k| m.contains(*k))
    .count();
    let registry_score = [
        "entité", "entite", "entités", "entites", "registry", "registre", "json",
        "écosystème", "ecosysteme", "attribut", "schema", "schéma", "paramètres", "parametres",
        "facture", "créer", "creer", "cre moi", "nouvelle entité", "nouvelle entite",
    ]
    .iter()
    .filter(|k| m.contains(*k))
    .count();
    if print_score > registry_score {
        VisionMode::Print
    } else {
        VisionMode::Registry
    }
}

fn decode_image(image_base64: &str) -> Result<(String, String), String> {
    let raw = image_base64.trim();
    let (mime, b64) = if raw.starts_with("data:") {
        let rest = raw.strip_prefix("data:").unwrap_or(raw);
        let (meta, data) = rest
            .split_once(',')
            .ok_or_else(|| "Data-URI image invalide.".to_string())?;
        let mime = meta
            .split(';')
            .next()
            .unwrap_or("image/png")
            .trim()
            .to_string();
        (mime, data.trim().to_string())
    } else {
        ("image/png".to_string(), raw.to_string())
    };
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.replace('\n', "").replace('\r', ""))
        .map_err(|e| format!("Image base64 invalide : {e}"))?;
    if bytes.is_empty() {
        return Err("Fichier image vide.".into());
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "Image trop volumineuse (max {} Mo).",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    let data_url = if raw.starts_with("data:") {
        raw.to_string()
    } else {
        format!("data:{mime};base64,{b64}")
    };
    Ok((mime, data_url))
}

fn format_attr_for_vision(a: &registry::EntityAttribute) -> String {
    if is_reserved_attribute(a) {
        return String::new();
    }
    let label = a.label.as_deref().unwrap_or(&a.nom);
    let mut parts = vec![
        a.nom.clone(),
        a.attr_type.clone(),
        format!("label={label}"),
    ];
    if a.required {
        parts.push("required".into());
    }
    if a.attr_type == "entity" {
        if let Some(r) = a.r#ref.as_deref().filter(|s| !s.trim().is_empty()) {
            parts.push(format!("ref={r}"));
        }
        if a.relation_multiple {
            parts.push("relation_multiple=true (liste embarquée — entité fille)".into());
        } else {
            parts.push("relation_multiple=false (liaison 1-1 embarquée)".into());
        }
    }
    if let Some(opts) = &a.enum_options {
        if !opts.is_empty() {
            parts.push(format!("enum=[{}]", opts.join(",")));
        }
    }
    if let Some(m) = a.matricule_ref.as_deref().filter(|s| !s.trim().is_empty()) {
        parts.push(format!("matricule_ref={m}"));
    }
    parts.join(" · ")
}

fn registry_context(data_dir: &Path) -> String {
    let Ok(reg) = registry::load(data_dir) else {
        return String::from(
            "## Registre actuel\n\n(Aucun registre chargé — propose un écosystème cohérent.)\n",
        );
    };
    let eco = reg.ecosysteme.as_deref().unwrap_or("").trim();
    let slogan = reg.slogan.as_deref().unwrap_or("").trim();
    let mut lines = vec![
        "## Registre Blin actuel — À RÉUTILISER EN PRIORITÉ".to_string(),
        String::new(),
        "Ne recrée pas des champs déjà couverts par une entité existante : préfère une **liaison entity** vers l'entité fille (catalogue) plutôt que de dupliquer les attributs sur le parent.".to_string(),
        String::new(),
    ];
    if !eco.is_empty() {
        lines.push(format!("- **ecosysteme** : {eco}"));
    }
    if !slogan.is_empty() {
        lines.push(format!("- **slogan** : {slogan}"));
    }
    lines.push(String::new());
    lines.push("### Entités existantes (nom technique = clé exacte)".to_string());

    for ent in &reg.entities {
        let label = ent.label.as_deref().unwrap_or(&ent.nom);
        let desc = ent.description.as_deref().unwrap_or("").trim();
        lines.push(String::new());
        lines.push(format!("#### `{nom}` — {label}", nom = ent.nom));
        if !desc.is_empty() {
            lines.push(format!("_{desc}_"));
        }
        let mut flags = Vec::new();
        if ent.requires_signature {
            flags.push("requires_signature");
        }
        if ent.is_session {
            flags.push("is_session");
        }
        if !flags.is_empty() {
            lines.push(format!("Flags : {}", flags.join(", ")));
        }
        lines.push("Attributs :".to_string());
        let mut has_entity_link = false;
        for a in &ent.attributs {
            let formatted = format_attr_for_vision(a);
            if formatted.is_empty() {
                continue;
            }
            if a.attr_type == "entity" {
                has_entity_link = true;
            }
            lines.push(format!("  - {formatted}"));
        }
        if has_entity_link {
            lines.push("  → Cette entité **consomme déjà des entités filles** — imite ce modèle.".to_string());
        }
    }

    lines.push(String::new());
    lines.push("### Graphe des liaisons filles (parent → ref)".to_string());
    let mut any_link = false;
    for ent in &reg.entities {
        for a in &ent.attributs {
            if a.attr_type != "entity" {
                continue;
            }
            let Some(child) = a.r#ref.as_deref().filter(|s| !s.trim().is_empty()) else {
                continue;
            };
            any_link = true;
            let mode = if a.relation_multiple {
                "liste embarquée (plusieurs lignes)"
            } else {
                "1-1 embarqué"
            };
            lines.push(format!(
                "  - `{parent}`.{attr} → `{child}` ({mode})",
                parent = ent.nom,
                attr = a.nom,
            ));
        }
    }
    if !any_link {
        lines.push("  (aucune liaison entity pour l'instant)".to_string());
    }

    lines.join("\n")
}

fn registry_json_snapshot(data_dir: &Path) -> String {
    let Ok(reg) = registry::load(data_dir) else {
        return r#"{ "ecosysteme": "Mon écosystème", "slogan": "", "entities": [] }"#
            .to_string();
    };
    serde_json::to_string_pretty(&reg).unwrap_or_else(|_| "{}".to_string())
}

fn schema_extraction_privacy_section() -> &'static str {
    r#"## Formulaire métier — extraction de schéma (pas de PII)

L'image est un **modèle vierge** (facture, fiche, bon…) : tu extrais uniquement la **structure** (sections + noms de champs).
- Les libellés « Nom », « Adresse », « Téléphone », « VIN »… du vendeur/acheteur/véhicule sont des **noms d'attributs** techniques (`nom_vendeur`, `adresse_acheteur`, `vin`…), **pas** des personnes ou données réelles.
- **Ne recopie aucune valeur** saisie ou pré-remplie dans le document.
- **N'inclus jamais** de métadonnées Safety / PII / Privacy / User Safety dans ta réponse.
- Sortie attendue : intro courte (2-3 phrases) + **un seul** bloc ```json` avec le registry Blin."#
}

const RETRY_SCHEMA_ADDENDUM: &str = r#"

---
RAPPEL : produis le registry JSON Blin complet. Aucune ligne Safety/PII/Privacy. Schéma de champs uniquement depuis le modèle de formulaire vierge."#;

fn response_contains_registry_json(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("```json") && (lower.contains("\"entities\"") || lower.contains("\"ecosysteme\""))
}

fn is_safety_refusal(text: &str) -> bool {
    if response_contains_registry_json(text) {
        return false;
    }
    let lower = text.to_lowercase();
    lower.contains("response safety:")
        || lower.contains("safety categories:")
        || (lower.contains("user safety:") && lower.contains("unsafe"))
        || (lower.contains("pii") && lower.contains("privacy"))
}

fn clean_vision_response(text: &str) -> String {
    if let Some(idx) = text.find("```json") {
        if idx > 0 {
            let head = &text[..idx];
            if head.to_lowercase().contains("safety") {
                return format!(
                    "Voici le registre d'entités extrait du formulaire :\n\n{}",
                    text[idx..].trim()
                );
            }
        }
    }
    text.trim().to_string()
}

fn vision_safety_error(text: &str, provider_label: &str) -> String {
    let snippet: String = text.lines().take(4).collect::<Vec<_>>().join(" · ");
    format!(
        "Analyse bloquée par le filtre de sécurité du modèle ({provider_label}). \
         Les libellés du formulaire (Nom, Adresse, Téléphone…) sont parfois interprétés à tort comme des données personnelles.\n\n\
         **À faire :**\n\
         1. **Paramètres → Assistant → Analyse d'image** — essayez **Gemini** (`gemini-2.0-flash`) \
            ou le modèle `qwen/qwen2.5-vl-72b-instruct:free`\n\
         2. Sur [openrouter.ai/settings/privacy](https://openrouter.ai/settings/privacy), vérifiez qu'aucun guardrail PII ne bloque vos requêtes\n\
         3. Réessayez avec la même image\n\n\
         Détail : {snippet}"
    )
}

fn attribute_immutability_section() -> &'static str {
    r#"## Règles sur les attributs existants (OBLIGATOIRE)

- **INTERDIT** sur toute entité déjà présente dans le registre :
  - supprimer un attribut existant ;
  - renommer un attribut existant (`nom`) ;
  - changer le `type` d'un attribut existant ;
  - modifier `ref`, `relation_multiple`, `required`, `label` ou toute propriété d'un attribut existant.
- **AUTORISÉ** :
  - ajouter de **nouveaux** attributs à la fin de `attributs[]` sur une entité existante (y compris entité fille / catalogue) ;
  - créer une **nouvelle** entité complète si absente du registre.
- Les attributs existants doivent être recopiés **à l'identique** dans le JSON final."#
}

fn full_registry_output_section(snapshot: &str) -> String {
    format!(
        r#"## JSON de sortie COMPLET (OBLIGATOIRE)

Tu DOIS renvoyer le **registre intégral** fusionné, pas un extrait :
- `ecosysteme`, `slogan` (conserver les valeurs actuelles sauf demande explicite)
- `entities[]` avec **TOUTES** les entités existantes **plus** les nouvelles ou complétées
- **Jamais** un JSON partiel (ex. uniquement `facture`) — l'utilisateur remplace tout le registre d'un coup

Registre actuel à fusionner sans perte (base de départ) :
```json
{snapshot}
```"#
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisionAttributeHint {
    pub nom: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisionEntityOptions {
    #[serde(default)]
    pub requires_signature: bool,
    #[serde(default)]
    pub ai_suggestions: bool,
    #[serde(default)]
    pub signatory_role_ids: Vec<String>,
    #[serde(default)]
    pub attribute_hints: Vec<VisionAttributeHint>,
}

fn entity_options_section(options: &VisionEntityOptions) -> String {
    let sig = if options.requires_signature {
        "true"
    } else {
        "false"
    };
    let sugg = if options.ai_suggestions {
        "true"
    } else {
        "false"
    };
    let roles = if options.signatory_role_ids.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            options
                .signatory_role_ids
                .iter()
                .map(|id| format!("\"{id}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        r#"## Options utilisateur — entité principale créée/étendue depuis l'image

Applique **exactement** ces flags sur l'entité document/mère identifiée (ex. `facture`, `commande`) :
- `requires_signature`: {sig}
- `ai_suggestions`: {sugg}
- `signatory_role_ids`: {roles}

Si `requires_signature` est false, laisse `signatory_role_ids` absent ou `[]`.

{attribute_hints_section}"#,
        attribute_hints_section = attribute_hints_prompt_section(options),
    )
}

fn attribute_hints_prompt_section(options: &VisionEntityOptions) -> String {
    if options.attribute_hints.is_empty() {
        return "## Colonnes / obligation\n\nAucune indication utilisateur — déduis les attributs depuis l'image ; marque `required: true` si le champ est visuellement obligatoire (*, « obligatoire », case sans « optionnel »).".to_string();
    }
    let mut lines = vec![
        "## Colonnes / obligation (hints utilisateur — OBLIGATOIRE)".to_string(),
        String::new(),
        "Applique `required` **uniquement** sur les **nouveaux** attributs créés ou ajoutés (ne modifie jamais `required` d'un attribut existant dans le registre) :".to_string(),
    ];
    for h in &options.attribute_hints {
        let nom = h.nom.trim();
        if nom.is_empty() {
            continue;
        }
        let req = if h.required { "true" } else { "false" };
        lines.push(format!("- `{nom}` → `\"required\": {req}`"));
    }
    lines.push(String::new());
    lines.push(
        "Pour les autres attributs nouveaux non listés ci-dessus, déduis `required` depuis l'image.".to_string(),
    );
    lines.join("\n")
}

fn child_entity_strategy_section() -> &'static str {
    r#"## Stratégie entités filles (OBLIGATOIRE — favoriser au maximum)

Blin modélise les documents métier ainsi :
1. **Entité catalogue / fille** (ex. `client`, `article`, `vehicule`) : fiche réutilisable avec ses propres attributs.
2. **Entité document / mère** (ex. `facture`, `commande`, `demande_achat`) : contient des **liaisons `type: "entity"`** vers les catalogues existants, **sans recopier** tous les champs de la fille sur le parent.

### Règles de priorité
- **Réutilise** les entités listées dans le registre ci-dessus si l'image correspond (client, article, véhicule…).
- **Bloc répété / tableau de lignes** dans l'image (articles, lignes facture, véhicules listés) → attribut `type: "entity"`, `ref: "<entité catalogue>"`, **`relation_multiple: true`**.
- **Une seule fiche liée** (un client, un vendeur) → `type: "entity"`, `ref: "..."`, `relation_multiple: false`.
- **Ne duplique pas** sur le parent les champs déjà présents sur l'entité fille (ex. ne mets pas `marque`, `modele` sur `facture` si `article` existe déjà — mets `articles` → ref `article`).
- Crée une **nouvelle entité fille catalogue** uniquement si l'image montre un concept absent du registre (ex. nouvelle notion « vendeur » sans entité équivalente).
- Pour une **nouvelle entité mère** (ex. `facture`) : minimum d'attributs propres (date, n°, statut, montant total…) + liaisons vers filles existantes.

### Exemple facture véhicule (image type formulaire)
- Entité mère `facture` : date_vente, prix_vente, mode_paiement…
- `client` → ref entité `client` existante (1-1)
- `vehicule` ou `articles` → ref entité `article` (ou `vehicule`) existante (1-1 ou liste selon l'image)
- **Ne pas** recréer `client`/`article` en entiers si déjà dans le registre — seulement **ajouter** des attributs manquants sur ces entités filles (sans toucher aux attributs existants).

### Impact stock (optionnel)
Si une ligne embarquée représente une quantité à décrémenter : sur l'attribut entity multiple, tu peux proposer `relation_impact_source`, `relation_impact_target` (champ numérique de la fille, ex. `qte_initial`), `relation_impact_action: "decrement"`."#
}

fn system_prompt(mode: VisionMode, data_dir: &Path, options: Option<&VisionEntityOptions>) -> String {
    let registry_ctx = registry_context(data_dir);
    let child_strategy = child_entity_strategy_section();
    let immutability = attribute_immutability_section();
    let privacy_schema = schema_extraction_privacy_section();
    let snapshot = registry_json_snapshot(data_dir);
    let full_output = full_registry_output_section(&snapshot);
    let entity_opts = options
        .map(entity_options_section)
        .unwrap_or_default();
    match mode {
        VisionMode::Registry => format!(
            r#"Tu es un architecte de données pour **Blin** (application desktop Tauri + SQLite).
Analyse l'image fournie (formulaire, tableau, fiche papier, capture d'écran métier) et produis un **JSON de registre d'entités** compatible Blin.

{registry_ctx}

{child_strategy}

{immutability}

{privacy_schema}

{full_output}

{entity_opts}

## Règles strictes Blin
- Réponds UNIQUEMENT en français dans l'introduction (2-3 phrases max), puis un bloc ```json unique.
- **Ne pas** inclure de commentaire dans le JSON.
- Clés autorisées par entité : nom, label, description, ai_suggestions, requires_signature, signatory_role_ids, is_session, attributs.
- Attribut : nom, type, label, required, ref (si type entity), relation_multiple, relation_impact_* (optionnel), matricule_ref (optionnel).
- Types : string, number, integer, float, boolean, date, datetime, time, email, photo, enum[a,b,c], entity, stock, compteur, matricule.
- **Ne pas** déclarer id/uuid dans les attributs (ID système auto).
- **Ne pas** déclarer l'entité stock (injectée auto si attribut stock).
- Propose des noms techniques en snake_case sans accents (ex. facture, demande_achat).
- Fusionne avec le registre existant : **toutes** les entités présentes dans la base ci-dessus doivent réapparaître dans ta réponse, avec les ajouts/modifications autorisés uniquement.

## Interdit
- Aucune instruction pour enregistrer automatiquement dans l'app.
- L'utilisateur collera le JSON lui-même dans Paramètres → Entités → Vue JSON.
- JSON partiel (une seule entité) — **interdit**.

Format attendu après ton intro :
```json
{{ "ecosysteme": "...", "slogan": "...", "entities": [ ... ] }}
```
"#,
        ),
        VisionMode::Print => format!(
            r#"Tu es un expert mise en page pour **Blin** (modèles d'impression HTML/CSS).
Reproduis l'apparence de l'image (fiche, facture, liste, certificat…) en HTML + CSS pour impression A4.

{registry_ctx}

{child_strategy}

{privacy_schema}

## Règles impression
- Introduction courte en français (2-3 phrases), puis un seul bloc ```json.
- Placeholders Blin : {{{{entite.attribut}}}} — utilise les **noms d'entités et attributs du registre ci-dessus**.
- Pour les lignes répétées (tableau), prévois un placeholder de bloc ou une boucle logique décrite en commentaire HTML si besoin.
- `screen_key` = nom de l'entité mère la plus probable (ex. facture, commande).
- HTML sémantique, classes .fiche, .fiche-grid, .fiche-field, .fiche-label, .fiche-value si adapté.
- CSS autonome (polices system-ui), fond blanc, prêt pour @media print.
- **Ne pas** enregistrer — l'utilisateur collera dans Paramètres → Création de modèles d'impression.

Format JSON strict :
```json
{{
  "name": "Nom du modèle",
  "description": "…",
  "screen_key": "nom_entite ou null",
  "html_content": "<div class=\"fiche\">…</div>",
  "css_content": "…"
}}
```
"#,
        ),
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Option<Vec<OpenAiChoice>>,
    error: Option<OpenAiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorBody {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessageOut>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessageOut {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

fn call_openrouter(
    api_key: &str,
    model: &str,
    system: &str,
    user_text: &str,
    data_url: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            {
                "role": "user",
                "content": [
                    { "type": "text", "text": user_text },
                    { "type": "image_url", "image_url": { "url": data_url } }
                ]
            }
        ],
        "temperature": 0.15,
        "max_tokens": 16384
    });
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("HTTP-Referer", "https://blin.local")
        .header("X-Title", "Blin")
        .json(&body)
        .send()
        .map_err(|e| format!("Appel OpenRouter impossible : {e}"))?;
    let status = resp.status();
    let raw = resp.text().map_err(|e| e.to_string())?;
    let parsed: OpenAiChatResponse =
        serde_json::from_str(&raw).map_err(|e| format!("Réponse OpenRouter illisible : {e}"))?;
    if let Some(err) = parsed.error {
        return Err(err
            .message
            .unwrap_or_else(|| format!("Erreur OpenRouter ({status})")));
    }
    let text = parsed
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err(format!(
            "OpenRouter n'a renvoyé aucun contenu ({status}). Réessayez ou changez de modèle dans Paramètres."
        ));
    }
    Ok(text.trim().to_string())
}

fn call_gemini(
    api_key: &str,
    model: &str,
    system: &str,
    user_text: &str,
    mime: &str,
    b64: &str,
) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    );
    let body = serde_json::json!({
        "systemInstruction": {
            "parts": [{ "text": system }]
        },
        "contents": [{
            "role": "user",
            "parts": [
                { "text": user_text },
                { "inline_data": { "mime_type": mime, "data": b64 } }
            ]
        }],
        "safetySettings": [
            { "category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE" },
            { "category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE" },
            { "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE" },
            { "category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE" }
        ],
        "generationConfig": {
            "temperature": 0.15,
            "maxOutputTokens": 16384
        }
    });
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("Appel Gemini impossible : {e}"))?;
    let status = resp.status();
    let raw = resp.text().map_err(|e| e.to_string())?;
    let parsed: GeminiResponse =
        serde_json::from_str(&raw).map_err(|e| format!("Réponse Gemini illisible : {e}"))?;
    if let Some(err) = parsed.error {
        return Err(err.message.unwrap_or_else(|| format!("Erreur Gemini ({status})")));
    }
    let text = parsed
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().find_map(|part| part.text))
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err("Gemini n'a renvoyé aucun contenu.".into());
    }
    Ok(text.trim().to_string())
}

pub fn analyze_image(
    data_dir: &Path,
    user_message: &str,
    image_base64: &str,
    entity_options: Option<VisionEntityOptions>,
) -> Result<String, String> {
    let cfg = load_config(data_dir);
    let provider = VisionProvider::parse(&cfg.provider);
    let api_key = resolved_api_key(&cfg);
    if api_key.is_empty() {
        return Err(
            "Analyse d'image non configurée. Paramètres → Assistant → Analyse d'image : \
             choisissez OpenRouter (gratuit) ou Gemini, puis enregistrez une clé API."
                .into(),
        );
    }
    let model = resolved_model(&cfg, provider);
    let (mime, data_url) = decode_image(image_base64)?;
    use base64::Engine;
    let b64_raw = data_url
        .split_once(",")
        .map(|(_, d)| d.to_string())
        .unwrap_or_else(|| {
            base64::engine::general_purpose::STANDARD.encode(data_url.as_bytes())
        });

    let mode = detect_mode(user_message);
    let instruction = user_message.trim();
    let user_text = if instruction.is_empty() {
        match mode {
            VisionMode::Print => {
                "Reproduis cette image comme modèle d'impression HTML/CSS pour Blin.".to_string()
            }
            VisionMode::Registry => {
                "Extrais les entités et attributs métier visibles dans cette image pour un registry.json Blin.".to_string()
            }
        }
    } else {
        instruction.to_string()
    };
    let system = system_prompt(mode, data_dir, entity_options.as_ref());

    let provider_label = provider.label();
    let call_once = |user: &str| -> Result<String, String> {
        match provider {
            VisionProvider::OpenRouter => {
                call_openrouter(&api_key, &model, &system, user, &data_url)
            }
            VisionProvider::Gemini => {
                call_gemini(&api_key, &model, &system, user, &mime, &b64_raw)
            }
        }
    };

    let mut text = call_once(&user_text)?;
    if is_safety_refusal(&text) {
        let retry_user = format!("{user_text}{RETRY_SCHEMA_ADDENDUM}");
        text = call_once(&retry_user)?;
    }
    let text = clean_vision_response(&text);

    if is_safety_refusal(&text) {
        return Err(vision_safety_error(&text, provider_label));
    }
    if mode == VisionMode::Registry && !response_contains_registry_json(&text) {
        return Err(format!(
            "Le modèle n'a pas renvoyé de JSON de registre valide ({provider_label}). \
             Réessayez ou changez de modèle dans Paramètres → Assistant → Analyse d'image.\n\n\
             Réponse reçue : {}",
            text.chars().take(400).collect::<String>()
        ));
    }

    let footer = match mode {
        VisionMode::Registry => "\n\n---\n📋 **Collez le JSON** dans **Paramètres → Entités → Vue JSON**, vérifiez, puis enregistrez. Blin ne modifie pas le registre automatiquement.",
        VisionMode::Print => "\n\n---\n🖨️ **Collez le JSON** (ou html/css séparément) dans **Paramètres → Création de modèles d'impression**, puis enregistrez manuellement.",
    };
    Ok(format!("{text}{footer}"))
}

pub fn apply_config_patch(cfg: &mut VisionConfig, provider: Option<&str>, api_key: Option<&str>, model: Option<&str>) {
    if let Some(p) = provider {
        let t = p.trim();
        if !t.is_empty() {
            cfg.provider = t.to_string();
        }
    }
    if let Some(k) = api_key {
        let t = k.trim();
        if !t.is_empty() {
            cfg.api_key = t.to_string();
            cfg.gemini_api_key.clear();
        }
    }
    if let Some(m) = model {
        let t = m.trim();
        if !t.is_empty() {
            cfg.model = t.to_string();
        }
    }
}
