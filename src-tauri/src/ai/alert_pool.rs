//! Pool de réponses Loggy pré-générées (base SQLite séparée `blin-alert-pool.sqlite`).
//!
//! Objectif : les alertes (`alert.tsx`) ne doivent plus attendre le LLM.
//! Pour chaque « situation » (gabarit de message), on garde en réserve jusqu'à
//! [`POOL_TARGET`] reformulations. À l'affichage on en sort une au hasard,
//! on la supprime de la table, puis Loggy en régénère une en arrière-plan
//! (sans bloquer le reste de l'application : aucun verrou sur la base métier).

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;

use parking_lot::Mutex;
use rand::seq::SliceRandom;
use regex::Regex;
use rusqlite::{params, Connection};

use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::db::Database;

/// Nombre de réponses gardées en réserve par situation + variante.
pub const POOL_TARGET: i64 = 10;

const DB_FILENAME: &str = "blin-alert-pool.sqlite";
const MAX_SLOTS: usize = 20;

/// Marqueurs réinjectables (lettres pour ne jamais entrer en collision avec les nombres).
fn sentinel(idx: usize) -> String {
    format!("⟦{}⟧", (b'A' + idx as u8) as char)
}

/// Regex capturant les parties variables : « … » entre guillemets, ou groupes de chiffres.
fn slot_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"«[^»]*»|\d+(?:[ \u{00a0}.,]\d+)*").expect("regex slot valide")
    })
}

/// Découpe un message en gabarit (avec marqueurs) + valeurs capturées.
struct Templated {
    /// Clé de regroupement = message avec marqueurs (stable pour une même situation).
    key: String,
    /// Message à fournir au LLM (identique à `key`).
    sentinel_msg: String,
    /// Valeurs variables, dans l'ordre des marqueurs A, B, C…
    slots: Vec<String>,
}

fn templatize(raw: &str) -> Templated {
    let re = slot_regex();
    let mut slots: Vec<String> = Vec::new();
    let mut out = String::with_capacity(raw.len());
    let mut last = 0;
    for m in re.find_iter(raw) {
        if slots.len() >= MAX_SLOTS {
            break;
        }
        out.push_str(&raw[last..m.start()]);
        out.push_str(&sentinel(slots.len()));
        slots.push(m.as_str().to_string());
        last = m.end();
    }
    out.push_str(&raw[last..]);
    Templated {
        key: out.clone(),
        sentinel_msg: out,
        slots,
    }
}

/// Remplace les marqueurs par les valeurs réelles capturées.
fn reinject(body: &str, slots: &[String]) -> String {
    let mut out = body.to_string();
    for (i, value) in slots.iter().enumerate() {
        out = out.replace(&sentinel(i), value);
    }
    out
}

/// Vérifie que tous les marqueurs attendus sont présents (sinon réinjection impossible).
fn has_all_sentinels(body: &str, slot_count: usize) -> bool {
    (0..slot_count).all(|i| body.contains(&sentinel(i)))
}

pub struct AlertPool {
    conn: Mutex<Connection>,
    /// Situations en cours de régénération (évite les générations concurrentes en double).
    inflight: Mutex<HashSet<String>>,
}

impl AlertPool {
    fn init(conn: Connection) -> Result<Self, String> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS alert_pool (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 template_key TEXT NOT NULL,
                 variant TEXT NOT NULL,
                 body TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE INDEX IF NOT EXISTS idx_alert_pool_key
                 ON alert_pool(template_key, variant);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            conn: Mutex::new(conn),
            inflight: Mutex::new(HashSet::new()),
        })
    }

    pub fn open(data_dir: &Path) -> Result<Self, String> {
        let path = data_dir.join(DB_FILENAME);
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        Self::init(conn)
    }

    /// Repli si le fichier ne peut pas être ouvert (pool volatile, non bloquant).
    pub fn open_memory() -> Self {
        let conn = Connection::open_in_memory().expect("connexion sqlite mémoire");
        Self::init(conn).expect("init pool mémoire")
    }

    fn flight_key(key: &str, variant: &str) -> String {
        format!("{variant}\u{1}{key}")
    }

    /// Sort une réponse prête (et la supprime) pour la situation correspondante.
    /// Renvoie `None` si la réserve est vide (l'appelant utilisera un repli local instantané).
    fn pop_body(&self, key: &str, variant: &str) -> Option<String> {
        let conn = self.conn.lock();
        let mut ids: Vec<i64> = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT id FROM alert_pool WHERE template_key = ?1 AND variant = ?2")
                .ok()?;
            let rows = stmt
                .query_map(params![key, variant], |r| r.get::<_, i64>(0))
                .ok()?;
            for r in rows.flatten() {
                ids.push(r);
            }
        }
        let chosen = *ids.choose(&mut rand::thread_rng())?;
        let body: Option<String> = conn
            .query_row(
                "SELECT body FROM alert_pool WHERE id = ?1",
                params![chosen],
                |r| r.get(0),
            )
            .ok();
        let _ = conn.execute("DELETE FROM alert_pool WHERE id = ?1", params![chosen]);
        body
    }

    fn count(&self, key: &str, variant: &str) -> i64 {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT COUNT(*) FROM alert_pool WHERE template_key = ?1 AND variant = ?2",
            params![key, variant],
            |r| r.get(0),
        )
        .unwrap_or(0)
    }

    fn insert(&self, key: &str, variant: &str, body: &str) {
        let conn = self.conn.lock();
        let _ = conn.execute(
            "INSERT INTO alert_pool (template_key, variant, body) VALUES (?1, ?2, ?3)",
            params![key, variant, body],
        );
    }

    /// Tente de sortir une réponse personnifiée instantanée pour ce message.
    pub fn take_personified(&self, raw: &str, variant: &str) -> Option<String> {
        let tpl = templatize(raw);
        let body = self.pop_body(&tpl.key, variant)?;
        Some(reinject(&body, &tpl.slots))
    }
}

/// Construit le couple (système, utilisateur) demandant une reformulation avec marqueurs préservés.
fn build_prompt(app_name: &str, variant: &str, sentinel_msg: &str, slot_count: usize) -> Vec<ChatMessage> {
    let tone = match variant {
        "success" => "enthousiaste et rassurant — une bonne nouvelle à partager",
        "danger" => "direct mais bienveillant — un problème à signaler clairement",
        "warning" => "prudent sans alarmisme — une mise en garde amicale",
        _ => "naturel et utile — une info à transmettre avec clarté",
    };
    let marker_rule = if slot_count > 0 {
        let markers: Vec<String> = (0..slot_count).map(sentinel).collect();
        format!(
            "- Le message contient des marqueurs {} : RECOPIE-LES EXACTEMENT tels quels, \
             au bon endroit dans ta phrase (ne les traduis pas, ne les supprime pas, n'en ajoute pas).\n",
            markers.join(", ")
        )
    } else {
        String::new()
    };
    let system = format!(
        "Tu es Loggy, l'assistant IA de {app_name}. Tu parles à ton collègue utilisateur.\n\
         Règles strictes :\n\
         - Français parlé, 2 ou 3 phrases COMPLÈTES (jamais une ligne télégraphique).\n\
         - Première personne (je, j'ai, je viens de…), tutoiement naturel.\n\
         - Expressif et vivant : une vraie voix, pas une notification système.\n\
         {marker_rule}\
         - Ton {tone}.\n\
         - Interdit : JSON, puces, guillemets englobants, te nommer à la 3e personne, plus de 3 phrases."
    );
    let variation = chrono::Utc::now().timestamp_subsec_nanos();
    let user = format!(
        "Notification à reformuler ({variant}) — variation {variation} :\n{sentinel_msg}\n\
         Transforme-la en paroles naturelles que tu dirais à ton collègue."
    );
    vec![
        ChatMessage {
            role: "system".into(),
            content: system,
        },
        ChatMessage {
            role: "user".into(),
            content: user,
        },
    ]
}

fn sanitize(raw: &str, max_len: usize) -> String {
    let mut text = raw.trim().trim_matches('"').trim().to_string();
    if text.starts_with('«') && text.ends_with('»') && text.chars().count() > 2 {
        text = text.trim_start_matches('«').trim_end_matches('»').trim().to_string();
    }
    text = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() > max_len {
        let mut out: String = text.chars().take(max_len).collect();
        out.push('…');
        out
    } else {
        text
    }
}

/// Génère UNE reformulation (avec marqueurs) ; ne tient aucun verrou de base pendant l'appel LLM.
fn generate_one(app_name: &str, variant: &str, sentinel_msg: &str, slot_count: usize) -> Option<String> {
    let messages = build_prompt(app_name, variant, sentinel_msg, slot_count);
    let raw = LlamaServer::chat_with_options(None, messages, 0.9, 220).ok()?;
    let body = sanitize(&raw, 480);
    if body.chars().count() < 16 {
        return None;
    }
    // La réinjection exige tous les marqueurs : sinon on jette cette génération.
    if !has_all_sentinels(&body, slot_count) {
        return None;
    }
    Some(body)
}

/// Recomplète en arrière-plan la réserve d'une situation, sans bloquer l'application.
/// `db_arc` n'est verrouillé qu'un court instant pour lire le nom de l'écosystème.
pub fn spawn_refill(
    pool: Arc<AlertPool>,
    db_arc: Arc<Mutex<Database>>,
    raw_message: String,
    variant: String,
) {
    if !LlamaServer::model_ready() {
        return;
    }
    let tpl = templatize(&raw_message);
    let flight = AlertPool::flight_key(&tpl.key, &variant);

    {
        let mut inflight = pool.inflight.lock();
        if inflight.contains(&flight) {
            return;
        }
        inflight.insert(flight.clone());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let app_name = {
            let db = db_arc.lock();
            crate::entity::branding::ecosystem_name(&db.data_dir)
        };

        // Génère jusqu'à atteindre la cible ; s'arrête au premier échec pour ne pas tourner en boucle.
        while pool.count(&tpl.key, &variant) < POOL_TARGET {
            match generate_one(&app_name, &variant, &tpl.sentinel_msg, tpl.slots.len()) {
                Some(body) => pool.insert(&tpl.key, &variant, &body),
                None => break,
            }
        }

        pool.inflight.lock().remove(&flight);
    });
}
