use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::db::{Database, DbError};

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiMessageRow {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiConversationSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
}

#[derive(Debug, Clone)]
pub struct AiPendingRow {
    pub id: String,
    pub conversation_id: String,
    pub tool_name: String,
    pub params_json: String,
    pub privilege: String,
    pub confirm_privilege: Option<String>,
}

impl Database {
    pub fn ai_create_conversation(
        &self,
        id: &str,
        user_id: &str,
        title: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO ai_conversations (id, user_id, title, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, user_id, title, now],
        )?;
        Ok(())
    }

    pub fn ai_add_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO ai_messages (id, conversation_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, conversation_id, role, content, now],
        )?;
        Ok(())
    }

    /// Dernier message assistant de la conversation (hors message utilisateur en cours).
    pub fn ai_last_assistant_message(
        &self,
        conversation_id: &str,
    ) -> Result<Option<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT content FROM ai_messages
             WHERE conversation_id = ?1 AND role = 'assistant'
             ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![conversation_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row.get(0)?));
        }
        Ok(None)
    }

    pub fn ai_list_messages(
        &self,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<AiMessageRow>, DbError> {
        // Prend les N DERNIERS messages puis les remet en ordre chronologique.
        // Évite que le LLM reste bloqué sur le début de conversation.
        let sql = format!(
            "SELECT role, content FROM (
                SELECT id, role, content, created_at
                FROM ai_messages
                WHERE conversation_id = ?1
                ORDER BY created_at DESC, id DESC
                LIMIT {}
             ) latest
             ORDER BY created_at ASC, id ASC",
            limit.max(1).min(30)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![conversation_id], |row| {
                Ok(AiMessageRow {
                    role: row.get(0)?,
                    content: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn ai_insert_pending(
        &self,
        id: &str,
        conversation_id: &str,
        tool_name: &str,
        params_json: &str,
        privilege: &str,
        confirm_privilege: Option<&str>,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO ai_pending_actions (id, conversation_id, tool_name, params_json, privilege, confirm_privilege, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                conversation_id,
                tool_name,
                params_json,
                privilege,
                confirm_privilege,
                now
            ],
        )?;
        Ok(())
    }

    pub fn ai_get_pending(&self, id: &str) -> Result<AiPendingRow, DbError> {
        self.conn.query_row(
            "SELECT id, conversation_id, tool_name, params_json, privilege, confirm_privilege FROM ai_pending_actions WHERE id = ?1",
            params![id],
            |row| {
                Ok(AiPendingRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    tool_name: row.get(2)?,
                    params_json: row.get(3)?,
                    privilege: row.get(4)?,
                    confirm_privilege: row.get(5)?,
                })
            },
        )
        .map_err(DbError::from)
    }

    pub fn ai_delete_pending(&self, id: &str) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM ai_pending_actions WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Conversations de l'utilisateur, les plus récentes en premier.
    pub fn ai_list_conversations(&self, user_id: &str) -> Result<Vec<AiConversationSummary>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.title, c.created_at,
                    COALESCE(
                        (SELECT MAX(m.created_at) FROM ai_messages m WHERE m.conversation_id = c.id),
                        c.created_at
                    ) AS updated_at,
                    (SELECT COUNT(*) FROM ai_messages m WHERE m.conversation_id = c.id) AS message_count
             FROM ai_conversations c
             WHERE c.user_id = ?1
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                Ok(AiConversationSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    message_count: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn ai_conversation_owned_by(&self, conversation_id: &str, user_id: &str) -> Result<bool, DbError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM ai_conversations WHERE id = ?1 AND user_id = ?2",
            params![conversation_id, user_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    pub fn ai_rename_conversation(
        &self,
        user_id: &str,
        conversation_id: &str,
        title: &str,
    ) -> Result<bool, DbError> {
        if !self.ai_conversation_owned_by(conversation_id, user_id)? {
            return Ok(false);
        }
        let n = self.conn.execute(
            "UPDATE ai_conversations SET title = ?1 WHERE id = ?2 AND user_id = ?3",
            params![title, conversation_id, user_id],
        )?;
        Ok(n > 0)
    }

    pub fn ai_delete_conversation(&self, user_id: &str, conversation_id: &str) -> Result<bool, DbError> {
        if !self.ai_conversation_owned_by(conversation_id, user_id)? {
            return Ok(false);
        }
        self.conn.execute(
            "DELETE FROM ai_pending_actions WHERE conversation_id = ?1",
            params![conversation_id],
        )?;
        self.conn.execute(
            "DELETE FROM ai_messages WHERE conversation_id = ?1",
            params![conversation_id],
        )?;
        let n = self.conn.execute(
            "DELETE FROM ai_conversations WHERE id = ?1 AND user_id = ?2",
            params![conversation_id, user_id],
        )?;
        Ok(n > 0)
    }

    /// Historique complet d'une conversation (tableau de bord).
    pub fn ai_list_conversation_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AiMessageRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content FROM ai_messages
             WHERE conversation_id = ?1 AND role IN ('user', 'assistant')
             ORDER BY created_at ASC
             LIMIT 500",
        )?;
        let rows = stmt
            .query_map(params![conversation_id], |row| {
                Ok(AiMessageRow {
                    role: row.get(0)?,
                    content: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
