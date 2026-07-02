use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
pub struct SessionUser {
    pub id: String,
    pub nom: String,
    pub email: String,
    pub role: String,
    pub privileges: Vec<String>,
    #[serde(default)]
    pub must_change_password: bool,
}

#[derive(Clone, Debug)]
pub struct ActiveSession {
    pub token: String,
    pub user: SessionUser,
}

/// Session unique de l'application desktop (Tauri invoke).
pub struct SessionManager {
    inner: RwLock<Option<ActiveSession>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    pub fn set(&self, session: ActiveSession) {
        *self.inner.write() = Some(session);
    }

    pub fn clear(&self) {
        *self.inner.write() = None;
    }

    pub fn get(&self) -> Option<ActiveSession> {
        self.inner.read().clone()
    }

    pub fn require_session(&self) -> Result<ActiveSession, String> {
        self.get()
            .ok_or_else(|| "Session expirée — reconnectez-vous".to_string())
    }

    pub fn require_privilege(&self, privilege: &str) -> Result<ActiveSession, String> {
        let session = self.require_session()?;
        crate::privileges::require_privilege(&session.user.privileges, privilege)?;
        Ok(session)
    }

    /// Recharge les privilèges du rôle depuis SQLite (après création d'entité / trigger).
    pub fn sync_privileges(&self, db: &crate::db::Database) -> Result<Option<SessionUser>, String> {
        let Some(mut session) = self.get() else {
            return Ok(None);
        };
        let role_id: String = db
            .conn
            .query_row(
                "SELECT role_id FROM users WHERE id = ?1 AND actif = 1",
                rusqlite::params![session.user.id],
                |row| row.get(0),
            )
            .map_err(|_| "Utilisateur introuvable pour la session.".to_string())?;
        let privileges = db
            .get_role_privileges(&role_id)
            .map_err(|e| e.to_string())?;
        let must_change = db
            .user_must_change_password(&session.user.id)
            .map_err(|e| e.to_string())?;
        session.user.privileges = privileges;
        session.user.must_change_password = must_change;
        let user = session.user.clone();
        self.set(session);
        Ok(Some(user))
    }
}

/// Sessions mobiles indépendantes (ne remplacent pas la session desktop).
pub struct RemoteSessionStore {
    inner: RwLock<HashMap<String, ActiveSession>>,
}

impl RemoteSessionStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, session: ActiveSession) {
        self.inner
            .write()
            .insert(session.token.clone(), session);
    }

    pub fn get(&self, token: &str) -> Option<ActiveSession> {
        self.inner.read().get(token).cloned()
    }

    pub fn require_session(&self, token: &str) -> Result<ActiveSession, String> {
        self.get(token)
            .ok_or_else(|| "Session mobile expirée — reconnectez-vous".to_string())
    }

    pub fn require_privilege(&self, token: &str, privilege: &str) -> Result<ActiveSession, String> {
        let session = self.require_session(token)?;
        crate::privileges::require_privilege(&session.user.privileges, privilege)?;
        Ok(session)
    }
}

pub type SharedSession = Arc<SessionManager>;
pub type SharedRemoteSessions = Arc<RemoteSessionStore>;
