use rusqlite::params;
use uuid::Uuid;

use crate::db::{Database, DbError};

const CHUNK_SIZE: usize = 700;

pub struct RagStore<'a> {
    db: &'a Database,
}

impl<'a> RagStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn clear(&self) -> Result<(), DbError> {
        self.db.conn.execute("DELETE FROM ai_chunks", [])?;
        self.db.conn.execute("DELETE FROM ai_fts", [])?;
        Ok(())
    }

    pub fn insert_chunk(&self, source: &str, content: &str) -> Result<(), DbError> {
        let id = Uuid::new_v4().to_string();
        self.db.conn.execute(
            "INSERT INTO ai_chunks (id, source, content) VALUES (?1, ?2, ?3)",
            params![id, source, content],
        )?;
        self.db.conn.execute(
            "INSERT INTO ai_fts (chunk_id, source, content) VALUES (?1, ?2, ?3)",
            params![id, source, content],
        )?;
        Ok(())
    }

    pub fn index_text(&self, source: &str, text: &str) -> Result<usize, DbError> {
        let mut count = 0usize;
        let normalized = text.replace('\r', "");
        if normalized.trim().is_empty() {
            return Ok(0);
        }
        if normalized.len() <= CHUNK_SIZE {
            self.insert_chunk(source, &normalized)?;
            return Ok(1);
        }
        let mut start = 0usize;
        let chars: Vec<char> = normalized.chars().collect();
        while start < chars.len() {
            let end = (start + CHUNK_SIZE).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            self.insert_chunk(source, &chunk)?;
            count += 1;
            start = end;
        }
        Ok(count)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<String>, DbError> {
        let Some(fts_q) = build_fts_match_query(query) else {
            return Ok(Vec::new());
        };
        let sql = format!(
            "SELECT content FROM ai_fts WHERE ai_fts MATCH ?1 LIMIT {}",
            limit.min(8)
        );
        let mut stmt = self.db.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![fts_q], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

/// Nettoie un token FTS5 (underscores conservés pour MASTER_entities_*).
fn sanitize_fts_token(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn fts_quote_token(token: &str) -> String {
    format!("\"{}\"", token.replace('"', "\"\""))
}

fn collect_fts_tokens(user_query: &str) -> Vec<String> {
    let line = user_query.lines().next().unwrap_or(user_query).trim();
    if line.is_empty() {
        return Vec::new();
    }
    let with_or = line.replace(" or ", " OR ").replace(" Or ", " OR ");
    let parts: Vec<String> = if with_or.contains(" OR ") {
        with_or
            .split(" OR ")
            .map(|p| sanitize_fts_token(p.trim()))
            .filter(|t| t.len() >= 2)
            .collect()
    } else {
        line.split_whitespace()
            .map(sanitize_fts_token)
            .filter(|t| t.len() >= 2)
            .collect()
    };
    parts
}

fn build_fts_match_query(user_query: &str) -> Option<String> {
    let tokens = collect_fts_tokens(user_query);
    if tokens.is_empty() {
        return None;
    }
    Some(
        tokens
            .into_iter()
            .map(|t| fts_quote_token(&t))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

pub fn build_project_knowledge(root: &std::path::Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let json_knowledge = root.join("src").join("constante").join("json");
    if json_knowledge.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&json_knowledge) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(s) = std::fs::read_to_string(&path) {
                        let name = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("screen")
                            .to_string();
                        out.push((format!("json_screen_{name}"), s));
                    }
                }
            }
        }
    }
    let files = [
        ("README.md", root.join("README.md")),
        (".cursorrules", root.join(".cursorrules")),
    ];
    for (label, path) in files {
        if let Ok(s) = std::fs::read_to_string(&path) {
            out.push((label.to_string(), s));
        }
    }
    out.push((
        "outils_ia".into(),
        include_str!("knowledge_tools.txt").to_string(),
    ));
    out.push((
        "schema_metier".into(),
        include_str!("knowledge_schema.txt").to_string(),
    ));
    out.push((
        "layout_liste_vs_fiche".into(),
        include_str!("knowledge_layout.txt").to_string(),
    ));
    out
}
