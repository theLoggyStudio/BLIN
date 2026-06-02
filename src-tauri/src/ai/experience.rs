use chrono::Utc;
use rusqlite::params;
use serde_json::Value;
use uuid::Uuid;

use crate::ai::intent_filters::normalize_message;
use crate::ai::tools::{
    execute_read_tool, format_tool_reply, is_write_tool, queue_write_action, ToolCall, ToolResult,
};
use crate::db::{Database, DbError};

const MIN_REUSE_SCORE: f32 = 0.38;
const MAX_CANDIDATES: usize = 80;

#[derive(Debug, Clone)]
pub struct ExperienceMatch {
    pub tool_name: String,
    pub params: Value,
    pub summary: String,
    pub score: f32,
    pub use_count: i64,
}

#[derive(Debug, Clone)]
struct ExperienceRow {
    message_norm: String,
    tool_name: String,
    params_json: String,
    summary: String,
    use_count: i64,
}

impl Database {
    pub fn ai_upsert_experience(
        &self,
        message_norm: &str,
        tool_name: &str,
        params_json: &str,
        summary: &str,
        outcome: &str,
    ) -> Result<(), DbError> {
        if message_norm.len() < 4 {
            return Ok(());
        }
        let now = Utc::now().to_rfc3339();
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM ai_experience WHERE message_norm = ?1 AND tool_name = ?2",
                params![message_norm, tool_name],
                |row| row.get(0),
            )
            .ok();
        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE ai_experience SET params_json = ?1, summary = ?2, outcome = ?3,
                 use_count = use_count + 1, last_used_at = ?4 WHERE id = ?5",
                params![params_json, summary, outcome, now, id],
            )?;
        } else {
            let id = Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO ai_experience (id, message_norm, tool_name, params_json, summary, outcome, use_count, created_at, last_used_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
                params![id, message_norm, tool_name, params_json, summary, outcome, now],
            )?;
        }
        Ok(())
    }

    pub fn ai_fetch_experience_candidates(&self, limit: usize) -> Result<Vec<ExperienceRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT message_norm, tool_name, params_json, summary, use_count
             FROM ai_experience WHERE outcome IN ('success', 'confirmed')
             ORDER BY use_count DESC, last_used_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit.max(1).min(MAX_CANDIDATES) as i64], |row| {
                Ok(ExperienceRow {
                    message_norm: row.get(0)?,
                    tool_name: row.get(1)?,
                    params_json: row.get(2)?,
                    summary: row.get(3)?,
                    use_count: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn ai_experience_count(&self) -> Result<i64, DbError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM ai_experience", [], |r| r.get::<_, i64>(0))?)
    }

    pub fn ai_last_user_message(&self, conversation_id: &str) -> Result<Option<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT content FROM ai_messages
             WHERE conversation_id = ?1 AND role = 'user'
             ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![conversation_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row.get(0)?));
        }
        Ok(None)
    }
}

fn token_set(text: &str) -> Vec<String> {
    normalize_message(text)
        .split_whitespace()
        .filter(|w| w.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

fn similarity(query: &str, stored_norm: &str) -> f32 {
    let q = token_set(query);
    let s = token_set(stored_norm);
    if q.is_empty() || s.is_empty() {
        return 0.0;
    }
    let mut inter = 0usize;
    for t in &q {
        if s.iter().any(|x| x == t || x.contains(t.as_str()) || t.contains(x.as_str())) {
            inter += 1;
        }
    }
    inter as f32 / q.len().max(s.len()) as f32
}

pub fn find_best_experience(db: &Database, user_message: &str) -> Option<ExperienceMatch> {
    let candidates = db.ai_fetch_experience_candidates(MAX_CANDIDATES).ok()?;
    let mut best: Option<(ExperienceRow, f32)> = None;
    for row in candidates {
        let score = similarity(user_message, &row.message_norm);
        if score < MIN_REUSE_SCORE {
            continue;
        }
        let boosted = score + (row.use_count.min(10) as f32 * 0.02);
        if best.as_ref().map(|(_, s)| boosted > *s).unwrap_or(true) {
            best = Some((row, boosted));
        }
    }
    let (row, score) = best?;
    let params: Value = serde_json::from_str(&row.params_json).unwrap_or(Value::Object(Default::default()));
    Some(ExperienceMatch {
        tool_name: row.tool_name,
        params,
        summary: row.summary,
        score,
        use_count: row.use_count,
    })
}

pub fn format_experience_hints(db: &Database, user_message: &str, limit: usize) -> String {
    let Ok(candidates) = db.ai_fetch_experience_candidates(40) else {
        return String::new();
    };
    let mut scored: Vec<(String, f32)> = candidates
        .into_iter()
        .map(|r| {
            let score = similarity(user_message, &r.message_norm);
            (format!("- « {} » → {} : {}", r.message_norm, r.tool_name, r.summary), score)
        })
        .filter(|(_, s)| *s >= 0.25)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    if scored.is_empty() {
        return String::new();
    }
    let app = crate::entity::branding::ecosystem_name(&db.data_dir);
    format!(
        "Expérience {app} (demandes similaires déjà réussies sur ce PC):\n{}",
        scored
            .into_iter()
            .map(|(line, _)| line)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

pub fn record_success(
    db: &Database,
    user_message: &str,
    tool_name: &str,
    params: &Value,
    outcome: &str,
) {
    let norm = normalize_message(user_message);
    let params_json = serde_json::to_string(params).unwrap_or_else(|_| "{}".into());
    let summary = if let Some(s) = params.get("statut").and_then(|v| v.as_str()) {
        format!("statut={s}")
    } else {
        "sans filtre".into()
    };
    let _ = db.ai_upsert_experience(
        &norm,
        tool_name,
        &params_json,
        &summary,
        outcome,
    );
}

pub fn try_experience_intent(
    db: &Database,
    conversation_id: &str,
    user_message: &str,
    privileges: &[String],
) -> Option<(String, ToolResult, ExperienceMatch)> {
    let exp = find_best_experience(db, user_message)?;
    let call = ToolCall {
        tool: exp.tool_name.clone(),
        params: exp.params.clone(),
        explain: Some(format!(
            "Réutilisation expérience (score {:.0}%, {} fois)",
            exp.score * 100.0,
            exp.use_count
        )),
    };
    let tr = if is_write_tool(&call.tool) {
        queue_write_action(db, conversation_id, &call, privileges).ok()?
    } else {
        execute_read_tool(db, privileges, &call).ok()?
    };
    if !tr.success && !tr.requires_confirmation {
        return None;
    }
    record_success(db, user_message, &call.tool, &call.params, "success");
    let msg = if tr.requires_confirmation {
        format!(
            "{}\n\n(J’ai reconnu une demande similaire à une action passée — confirmez pour continuer.)",
            tr.message
        )
    } else if tr.data.is_some() {
        format_tool_reply(&call.tool, &tr)
    } else {
        tr.message.clone()
    };
    Some((msg, tr, exp))
}
