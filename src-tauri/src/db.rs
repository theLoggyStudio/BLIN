use bcrypt::{hash, verify, DEFAULT_COST};

/// Compte administrateur créé automatiquement si aucun utilisateur n'existe.
pub const DEFAULT_ADMIN_EMAIL: &str = "admin@blin.local";
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin1234";
use chrono::{Duration, Utc};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

pub const PAIRING_TTL_MINUTES: i64 = 15;

/// Fichier SQLite principal — application Blin.
pub const DB_FILENAME: &str = "blin-gestion.sqlite";

const LEGACY_DB_FILENAMES: &[&str] = &[
    "loggmagic-gestion.sqlite",
    "loggmagic.db",
    "loggimmo.db",
];

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Erreur base de données : {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Message(String),
}

pub struct Database {
    pub(crate) conn: Connection,
    pub data_dir: PathBuf,
}

impl Database {
    pub fn open(app_data_dir: PathBuf) -> Result<Self, DbError> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| DbError::Message(e.to_string()))?;
        std::fs::create_dir_all(app_data_dir.join("photos"))
            .map_err(|e| DbError::Message(e.to_string()))?;
        let db_path = app_data_dir.join(DB_FILENAME);
        Self::migrate_legacy_db_files(&app_data_dir, &db_path)?;
        let conn = Connection::open(&db_path)?;
        let db = Self {
            conn,
            data_dir: app_data_dir,
        };
        db.migrate()?;
        db.migrate_v2()?;
        db.migrate_v3()?;
        db.migrate_v4()?;
        db.migrate_v5()?;
        db.migrate_v6()?;
        db.migrate_v7()?;
        db.migrate_v8()?;
        db.migrate_v9()?;
        db.migrate_v10()?;
        db.migrate_v11()?;
        db.migrate_v12()?;
        db.migrate_v13()?;
        db.migrate_v14()?;
        db.migrate_v15()?;
        db.migrate_v16()?;
        db.migrate_v17()?;
        db.migrate_v18()?;
        db.migrate_v19()?;
        db.migrate_v20()?;
        db.seed()?;
        db.ensure_admin_account()?;
        db.ensure_demo_bureau()?;
        Ok(db)
    }

    pub fn photos_dir(&self) -> PathBuf {
        self.data_dir.join("photos")
    }

    fn migrate_legacy_db_files(app_data_dir: &Path, target: &Path) -> Result<(), DbError> {
        if target.exists() {
            return Ok(());
        }
        for legacy_name in LEGACY_DB_FILENAMES {
            let legacy = app_data_dir.join(legacy_name);
            if !legacy.exists() {
                continue;
            }
            Self::rename_sqlite_bundle(&legacy, target)?;
            return Ok(());
        }
        Ok(())
    }

    fn rename_sqlite_bundle(from: &Path, to: &Path) -> Result<(), DbError> {
        std::fs::rename(from, to).map_err(|e| {
            DbError::Message(format!(
                "Migration base {} → {} : {e}",
                from.display(),
                to.display()
            ))
        })?;
        if let Some(from_str) = from.to_str() {
            let to_str = to.to_string_lossy();
            for suffix in ["-wal", "-shm"] {
                let side_from = PathBuf::from(format!("{from_str}{suffix}"));
                if side_from.exists() {
                    let side_to = PathBuf::from(format!("{to_str}{suffix}"));
                    let _ = std::fs::rename(&side_from, &side_to);
                }
            }
        }
        Ok(())
    }

    fn migrate(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS roles (
                id TEXT PRIMARY KEY NOT NULL,
                nom TEXT NOT NULL UNIQUE
            );
            CREATE TABLE IF NOT EXISTS role_privileges (
                role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
                privilege TEXT NOT NULL,
                PRIMARY KEY (role_id, privilege)
            );
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                nom TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role_id TEXT NOT NULL REFERENCES roles(id),
                actif INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS biens (
                id TEXT PRIMARY KEY NOT NULL,
                reference TEXT NOT NULL UNIQUE,
                adresse TEXT NOT NULL,
                type_bien TEXT NOT NULL,
                statut TEXT NOT NULL DEFAULT 'disponible',
                surface_m2 REAL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS hangars (
                id TEXT PRIMARY KEY NOT NULL,
                reference TEXT NOT NULL UNIQUE,
                zone TEXT NOT NULL,
                capacite_m3 REAL NOT NULL,
                statut TEXT NOT NULL DEFAULT 'libre',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS contrats (
                id TEXT PRIMARY KEY NOT NULL,
                reference TEXT NOT NULL UNIQUE,
                bien_id TEXT REFERENCES biens(id),
                locataire TEXT NOT NULL,
                loyer_mensuel REAL NOT NULL,
                date_debut TEXT NOT NULL,
                date_fin TEXT,
                statut TEXT NOT NULL DEFAULT 'brouillon',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pairing_tokens (
                token TEXT PRIMARY KEY NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL DEFAULT '',
                used INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;
        Ok(())
    }

    fn migrate_v2(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS finances (
                id TEXT PRIMARY KEY NOT NULL,
                reference TEXT NOT NULL UNIQUE,
                libelle TEXT NOT NULL,
                montant REAL NOT NULL,
                type_operation TEXT NOT NULL DEFAULT 'loyer',
                statut TEXT NOT NULL DEFAULT 'en_attente',
                date_echeance TEXT NOT NULL,
                bien_reference TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS hangar_photos (
                id TEXT PRIMARY KEY NOT NULL,
                hangar_id TEXT NOT NULL REFERENCES hangars(id) ON DELETE CASCADE,
                filename TEXT NOT NULL,
                uploaded_at TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'desktop'
            );
            "#,
        )?;
        let _ = self.conn.execute(
            "ALTER TABLE pairing_tokens ADD COLUMN expires_at TEXT NOT NULL DEFAULT ''",
            [],
        );
        Ok(())
    }

    fn seed(&self) -> Result<(), DbError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM roles", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let roles: [(&str, &str, &[&str]); 3] = [
            ("role-admin", "Administrateur", &["*"]),
            (
                "role-directeur",
                "Directeur",
                &[
                    "directeur:confirmer",
                    "ai:utiliser",
                    "parametres:assistant",
                    "parametres:entites",
                    "parametres:entites:creer",
                    "parametres:roles",
                    "parametres:utilisateurs",
                ],
            ),
            ("role-agent", "Agent", &["ai:utiliser"]),
        ];

        for (id, nom, privileges) in roles {
            self.conn
                .execute("INSERT INTO roles (id, nom) VALUES (?1, ?2)", params![id, nom])?;
            for privilege in privileges {
                self.conn.execute(
                    "INSERT INTO role_privileges (role_id, privilege) VALUES (?1, ?2)",
                    params![id, privilege],
                )?;
            }
        }

        let admin_id = Uuid::new_v4().to_string();
        let password_hash = hash(DEFAULT_ADMIN_PASSWORD, DEFAULT_COST)
            .map_err(|e| DbError::Message(format!("Hash mot de passe : {e}")))?;

        self.conn.execute(
            "INSERT INTO users (id, nom, email, password_hash, role_id, must_change_password)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params![
                admin_id,
                "Administrateur",
                DEFAULT_ADMIN_EMAIL,
                password_hash,
                "role-admin"
            ],
        )?;

        Ok(())
    }

    /// Garantit un compte administrateur si la base n'en a aucun (ou plus d'admin actif).
    pub fn ensure_admin_account(&self) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO roles (id, nom) VALUES ('role-admin', 'Administrateur')",
            [],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES ('role-admin', '*')",
            [],
        )?;

        let active_users: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM users WHERE actif = 1", [], |r| r.get(0))?;

        if active_users > 0 {
            return Ok(());
        }

        let admin_id = Uuid::new_v4().to_string();
        let password_hash = hash(DEFAULT_ADMIN_PASSWORD, DEFAULT_COST)
            .map_err(|e| DbError::Message(format!("Hash mot de passe : {e}")))?;
        self.conn.execute(
            "INSERT INTO users (id, nom, email, password_hash, role_id, must_change_password)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params![
                admin_id,
                "Administrateur",
                DEFAULT_ADMIN_EMAIL,
                password_hash,
                "role-admin"
            ],
        )?;
        Ok(())
    }

    /// Compte démo « Bureau » (rôle Directeur) — connexion auto par défaut côté desktop.
    fn ensure_demo_bureau(&self) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO roles (id, nom) VALUES ('role-directeur', 'Directeur')",
            [],
        )?;

        let bureau_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE email = 'bureau@blin.local' AND actif = 1",
            [],
            |r| r.get(0),
        )?;
        if bureau_count > 0 {
            return Ok(());
        }

        let bureau_id = Uuid::new_v4().to_string();
        let password_hash = hash("Admin123!", DEFAULT_COST)
            .map_err(|e| DbError::Message(format!("Hash mot de passe : {e}")))?;
        self.conn.execute(
            "INSERT INTO users (id, nom, email, password_hash, role_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                bureau_id,
                "Bureau",
                "bureau@blin.local",
                password_hash,
                "role-directeur"
            ],
        )?;
        Ok(())
    }

    pub fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(String, String, String, String, Vec<String>, bool), DbError> {
        let (id, nom, password_hash, role_id, role_nom, must_change): (
            String,
            String,
            String,
            String,
            String,
            i64,
        ) = self
            .conn
            .query_row(
                "SELECT u.id, u.nom, u.password_hash, u.role_id, r.nom, u.must_change_password
                 FROM users u JOIN roles r ON u.role_id = r.id
                 WHERE u.email = ?1 AND u.actif = 1",
                params![email],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    DbError::Message("Identifiants invalides".to_string())
                }
                other => DbError::from(other),
            })?;

        if !verify(password, &password_hash)
            .map_err(|e| DbError::Message(format!("Vérification mot de passe : {e}")))?
        {
            return Err(DbError::Message("Identifiants invalides".to_string()));
        }

        let privileges = self.get_role_privileges(&role_id)?;
        Ok((
            id,
            nom,
            role_nom,
            role_id,
            privileges,
            must_change != 0,
        ))
    }

    pub fn user_must_change_password(&self, user_id: &str) -> Result<bool, DbError> {
        let flag: i64 = self.conn.query_row(
            "SELECT must_change_password FROM users WHERE id = ?1 AND actif = 1",
            params![user_id],
            |row| row.get(0),
        )?;
        Ok(flag != 0)
    }

    pub fn change_password(
        &self,
        user_id: &str,
        new_password: &str,
        allow_default: bool,
    ) -> Result<(), DbError> {
        if new_password.len() < 6 {
            return Err(DbError::Message(
                "Le mot de passe doit contenir au moins 6 caractères.".to_string(),
            ));
        }
        if !allow_default && new_password == DEFAULT_ADMIN_PASSWORD {
            return Err(DbError::Message(
                "Choisissez un mot de passe différent du mot de passe d'usine.".to_string(),
            ));
        }
        let password_hash = hash(new_password, DEFAULT_COST)
            .map_err(|e| DbError::Message(format!("Hash mot de passe : {e}")))?;
        let updated = self.conn.execute(
            "UPDATE users SET password_hash = ?1, must_change_password = 0 WHERE id = ?2 AND actif = 1",
            params![password_hash, user_id],
        )?;
        if updated == 0 {
            return Err(DbError::Message("Utilisateur introuvable.".to_string()));
        }
        Ok(())
    }

    pub fn get_role_privileges(&self, role_id: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT privilege FROM role_privileges WHERE role_id = ?1")?;
        let rows = stmt
            .query_map(params![role_id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(rows
            .into_iter()
            .filter(|p| !crate::privileges::is_legacy_immo_privilege(p))
            .collect())
    }

    pub fn create_pairing_token(&self) -> Result<String, DbError> {
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires = (now + Duration::minutes(PAIRING_TTL_MINUTES)).to_rfc3339();
        self.conn.execute(
            "INSERT INTO pairing_tokens (token, created_at, expires_at) VALUES (?1, ?2, ?3)",
            params![token, now.to_rfc3339(), expires],
        )?;
        Ok(token)
    }

    pub fn validate_pairing_token(&self, token: &str) -> Result<bool, DbError> {
        let now = Utc::now().to_rfc3339();
        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pairing_tokens WHERE token = ?1 AND used = 0 AND expires_at > ?2",
            params![token, now],
            |r| r.get(0),
        )?;
        Ok(exists > 0)
    }

    pub fn mark_pairing_used(&self, token: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE pairing_tokens SET used = 1 WHERE token = ?1",
            params![token],
        )?;
        Ok(())
    }

    // --- Biens ---
    const BIEN_SELECT: &str = "SELECT id, reference, adresse, type_bien, statut, surface_m2,
        prix_defaut, nb_etages, nb_chambres, nb_pieces, nomenclature_json, domaine, devise, zone, capacite_m3 FROM biens";

    fn map_bien_row(row: &rusqlite::Row<'_>) -> Result<BienRow, rusqlite::Error> {
        let nom_json: String = row.get(10)?;
        Ok(BienRow {
            id: row.get(0)?,
            reference: row.get(1)?,
            adresse: row.get(2)?,
            type_bien: row.get(3)?,
            statut: row.get(4)?,
            surface_m2: row.get(5)?,
            prix_defaut: row.get(6)?,
            nb_etages: row.get(7)?,
            nb_chambres: row.get(8)?,
            nb_pieces: row.get(9)?,
            nomenclature: crate::bien_detail::parse_nomenclature_json(&nom_json),
            domaine: row.get(11)?,
            devise: row.get(12)?,
            zone: row.get(13)?,
            capacite_m3: row.get(14)?,
        })
    }

    fn default_statut_for_type(type_bien: &str) -> &'static str {
        if type_bien == "hangar" {
            "libre"
        } else {
            "disponible"
        }
    }

    pub fn list_biens(&self) -> Result<Vec<BienRow>, DbError> {
        let sql = format!("{} ORDER BY reference", Self::BIEN_SELECT);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], Self::map_bien_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_bien(&self, input: BienInput<'_>) -> Result<BienRow, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let statut = input
            .statut
            .unwrap_or_else(|| Self::default_statut_for_type(input.type_bien));
        let (nb_etages, nb_chambres, nb_pieces, nomenclature) =
            crate::bien_detail::normalize_bien_counts(
                input.type_bien,
                input.nb_etages,
                input.nb_chambres,
                input.nb_pieces,
                input.nomenclature,
            );
        let nom_json = crate::bien_detail::serialize_nomenclature(&nomenclature);
        let domaine = crate::monnaie_domaine::normalize_domaine(input.domaine, input.type_bien);
        let devise = crate::monnaie_domaine::normalize_devise(input.devise);
        self.conn.execute(
            "INSERT INTO biens (id, reference, adresse, type_bien, statut, surface_m2, prix_defaut,
             nb_etages, nb_chambres, nb_pieces, nomenclature_json, domaine, devise, zone, capacite_m3, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                id,
                input.reference,
                input.adresse,
                input.type_bien,
                statut,
                input.surface_m2,
                input.prix_defaut,
                nb_etages,
                nb_chambres,
                nb_pieces,
                nom_json,
                domaine,
                devise,
                input.zone,
                input.capacite_m3,
                now
            ],
        )?;
        self.get_bien(&id)
    }

    pub fn update_bien(&self, id: &str, input: BienInput<'_>) -> Result<BienRow, DbError> {
        let statut = input
            .statut
            .unwrap_or_else(|| Self::default_statut_for_type(input.type_bien));
        let (nb_etages, nb_chambres, nb_pieces, nomenclature) =
            crate::bien_detail::normalize_bien_counts(
                input.type_bien,
                input.nb_etages,
                input.nb_chambres,
                input.nb_pieces,
                input.nomenclature,
            );
        let nom_json = crate::bien_detail::serialize_nomenclature(&nomenclature);
        let domaine = crate::monnaie_domaine::normalize_domaine(input.domaine, input.type_bien);
        let devise = crate::monnaie_domaine::normalize_devise(input.devise);
        self.conn.execute(
            "UPDATE biens SET reference=?1, adresse=?2, type_bien=?3, statut=?4, surface_m2=?5,
             prix_defaut=?6, nb_etages=?7, nb_chambres=?8, nb_pieces=?9, nomenclature_json=?10,
             domaine=?11, devise=?12, zone=?13, capacite_m3=?14 WHERE id=?15",
            params![
                input.reference,
                input.adresse,
                input.type_bien,
                statut,
                input.surface_m2,
                input.prix_defaut,
                nb_etages,
                nb_chambres,
                nb_pieces,
                nom_json,
                domaine,
                devise,
                input.zone,
                input.capacite_m3,
                id
            ],
        )?;
        self.get_bien(id)
    }

    pub fn delete_bien(&self, id: &str) -> Result<(), DbError> {
        let _ = self.delete_documents_for_entity("bien", id);
        self.conn
            .execute("DELETE FROM biens WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_bien(&self, id: &str) -> Result<BienRow, DbError> {
        let sql = format!("{} WHERE id = ?1", Self::BIEN_SELECT);
        self.conn
            .query_row(&sql, params![id], Self::map_bien_row)
            .map_err(DbError::from)
    }

    pub fn get_bien_by_reference(&self, reference: &str) -> Result<BienRow, DbError> {
        let sql = format!("{} WHERE reference = ?1", Self::BIEN_SELECT);
        self.conn
            .query_row(&sql, params![reference], Self::map_bien_row)
            .map_err(DbError::from)
    }

    pub fn get_last_bien(&self) -> Result<BienRow, DbError> {
        let sql = format!(
            "{} ORDER BY datetime(created_at) DESC LIMIT 1",
            Self::BIEN_SELECT
        );
        self.conn
            .query_row(&sql, [], Self::map_bien_row)
            .map_err(DbError::from)
    }

    pub fn search_biens(&self, query: &str, limit: usize) -> Result<Vec<BienRow>, DbError> {
        let pattern = format!("%{}%", query.trim());
        let sql = format!(
            "{} WHERE reference LIKE ?1 OR adresse LIKE ?1 OR type_bien LIKE ?1
             ORDER BY datetime(created_at) DESC LIMIT ?2",
            Self::BIEN_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![pattern, limit.max(1).min(30) as i64], Self::map_bien_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // --- Hangars (type de bien « hangar ») ---
    fn map_hangar_row(row: &rusqlite::Row<'_>) -> Result<HangarRow, rusqlite::Error> {
        Ok(HangarRow {
            id: row.get(0)?,
            reference: row.get(1)?,
            zone: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            capacite_m3: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            statut: row.get(4)?,
        })
    }

    pub fn list_hangars(&self) -> Result<Vec<HangarRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, reference, zone, capacite_m3, statut FROM biens
             WHERE type_bien = 'hangar' ORDER BY zone, reference",
        )?;
        let rows = stmt
            .query_map([], Self::map_hangar_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_hangar(
        &self,
        reference: &str,
        zone: &str,
        capacite_m3: f64,
    ) -> Result<HangarRow, DbError> {
        let adresse = format!("Zone {zone} — hangar logistique");
        let input = BienInput {
            reference,
            adresse: &adresse,
            type_bien: "hangar",
            statut: Some("libre"),
            surface_m2: None,
            prix_defaut: None,
            nb_etages: None,
            nb_chambres: None,
            nb_pieces: None,
            nomenclature: &[],
            domaine: "commercial",
            devise: "EUR",
            zone: Some(zone),
            capacite_m3: Some(capacite_m3),
        };
        let row = self.create_bien(input)?;
        Ok(HangarRow {
            id: row.id,
            reference: row.reference,
            zone: row.zone.unwrap_or_else(|| zone.to_string()),
            capacite_m3: row.capacite_m3.unwrap_or(capacite_m3),
            statut: row.statut,
        })
    }

    pub fn update_hangar(
        &self,
        id: &str,
        reference: &str,
        zone: &str,
        capacite_m3: f64,
        statut: &str,
    ) -> Result<HangarRow, DbError> {
        let existing = self.get_bien(id)?;
        let adresse = if existing.adresse.trim().is_empty() {
            format!("Zone {zone} — hangar logistique")
        } else {
            existing.adresse
        };
        let input = BienInput {
            reference,
            adresse: &adresse,
            type_bien: "hangar",
            statut: Some(statut),
            surface_m2: None,
            prix_defaut: existing.prix_defaut,
            nb_etages: None,
            nb_chambres: None,
            nb_pieces: None,
            nomenclature: &[],
            domaine: &existing.domaine,
            devise: &existing.devise,
            zone: Some(zone),
            capacite_m3: Some(capacite_m3),
        };
        let row = self.update_bien(id, input)?;
        Ok(HangarRow {
            id: row.id,
            reference: row.reference,
            zone: row.zone.unwrap_or_else(|| zone.to_string()),
            capacite_m3: row.capacite_m3.unwrap_or(capacite_m3),
            statut: row.statut,
        })
    }

    pub fn hangar_exists(&self, id: &str) -> Result<bool, DbError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM biens WHERE id = ?1 AND type_bien = 'hangar'",
            params![id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    pub fn add_hangar_photo(
        &self,
        hangar_id: &str,
        filename: &str,
        source: &str,
    ) -> Result<HangarPhotoRow, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO hangar_photos (id, hangar_id, filename, uploaded_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, hangar_id, filename, now, source],
        )?;
        Ok(HangarPhotoRow {
            id,
            hangar_id: hangar_id.to_string(),
            filename: filename.to_string(),
            uploaded_at: now,
            source: source.to_string(),
        })
    }

    pub fn list_hangar_photos(&self, hangar_id: &str) -> Result<Vec<HangarPhotoRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, hangar_id, filename, uploaded_at, source FROM hangar_photos
             WHERE hangar_id = ?1 ORDER BY uploaded_at DESC",
        )?;
        let rows = stmt
            .query_map(params![hangar_id], |row| {
                Ok(HangarPhotoRow {
                    id: row.get(0)?,
                    hangar_id: row.get(1)?,
                    filename: row.get(2)?,
                    uploaded_at: row.get(3)?,
                    source: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn photo_absolute_path(&self, hangar_id: &str, filename: &str) -> PathBuf {
        self.photos_dir().join(hangar_id).join(filename)
    }

    // --- Contrats ---
    const CONTRAT_SELECT: &str = "SELECT c.id, c.reference, c.locataire, c.locataire_email, c.locataire_telephone,
        c.loyer_mensuel, c.date_debut, c.date_fin, c.statut, b.reference AS bien_reference, c.bien_id,
        b.type_bien AS bien_type, c.logement_cle, c.devise, b.domaine AS bien_domaine
        FROM contrats c LEFT JOIN biens b ON c.bien_id = b.id";

    fn map_contrat_row(row: &rusqlite::Row<'_>) -> Result<ContratRow, rusqlite::Error> {
        Ok(ContratRow {
            id: row.get(0)?,
            reference: row.get(1)?,
            locataire: row.get(2)?,
            locataire_email: row.get(3)?,
            locataire_telephone: row.get(4)?,
            loyer_mensuel: row.get(5)?,
            date_debut: row.get(6)?,
            date_fin: row.get(7)?,
            statut: row.get(8)?,
            bien_reference: row.get(9)?,
            bien_id: row.get(10)?,
            bien_type: row.get(11)?,
            logement_cle: row.get(12)?,
            devise: row.get(13)?,
            bien_domaine: row.get(14)?,
        })
    }

    fn resolve_contrat_devise(&self, bien_id: Option<&str>, devise_in: &str) -> Result<String, DbError> {
        let trimmed = devise_in.trim();
        if !trimmed.is_empty() {
            return Ok(crate::monnaie_domaine::normalize_devise(trimmed));
        }
        if let Some(bid) = bien_id.filter(|s| !s.is_empty()) {
            let bien = self.get_bien(bid)?;
            return Ok(crate::monnaie_domaine::normalize_devise(&bien.devise));
        }
        Ok(crate::monnaie_domaine::normalize_devise("EUR"))
    }

    pub(crate) fn validate_contrat_logement(
        &self,
        bien_id: Option<&str>,
        logement_cle: Option<&str>,
    ) -> Result<(), DbError> {
        let Some(bid) = bien_id.filter(|s| !s.is_empty()) else {
            return Ok(());
        };
        let bien = self.get_bien(bid)?;
        if bien.type_bien != "immeuble" {
            return Ok(());
        }
        let cle = logement_cle
            .filter(|s| !s.is_empty())
            .ok_or_else(|| DbError::Message("Chambre / logement requis pour un immeuble".to_string()))?;
        if !crate::bien_detail::logement_cle_exists_in_bien(
            &bien.reference,
            &bien.nomenclature,
            cle,
        ) {
            return Err(DbError::Message(
                "Clé de logement invalide pour la nomenclature de cet immeuble".to_string(),
            ));
        }
        Ok(())
    }

    pub fn list_contrats(&self) -> Result<Vec<ContratRow>, DbError> {
        let sql = format!("{} ORDER BY c.date_debut DESC", Self::CONTRAT_SELECT);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], Self::map_contrat_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_contrat(&self, input: ContratInput<'_>) -> Result<ContratRow, DbError> {
        self.validate_contrat_logement(input.bien_id, input.logement_cle)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let logement = input
            .logement_cle
            .filter(|s| !s.is_empty());
        let devise = self.resolve_contrat_devise(input.bien_id, input.devise)?;
        self.conn.execute(
            "INSERT INTO contrats (id, reference, bien_id, locataire, locataire_email, locataire_telephone,
             loyer_mensuel, date_debut, date_fin, statut, logement_cle, devise, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'brouillon', ?10, ?11, ?12)",
            params![
                id,
                input.reference,
                input.bien_id,
                input.locataire,
                input.locataire_email,
                input.locataire_telephone,
                input.loyer_mensuel,
                input.date_debut,
                input.date_fin,
                logement,
                devise,
                now
            ],
        )?;
        self.get_contrat(&id)
    }

    pub fn sign_contrat(&self, id: &str) -> Result<ContratRow, DbError> {
        self.conn.execute(
            "UPDATE contrats SET statut = 'actif' WHERE id = ?1",
            params![id],
        )?;
        self.get_contrat(id)
    }

    pub fn get_contrat(&self, id: &str) -> Result<ContratRow, DbError> {
        let sql = format!("{} WHERE c.id = ?1", Self::CONTRAT_SELECT);
        self.conn
            .query_row(&sql, params![id], Self::map_contrat_row)
            .map_err(DbError::from)
    }

    pub fn get_contrat_by_reference(&self, reference: &str) -> Result<ContratRow, DbError> {
        let sql = format!("{} WHERE c.reference = ?1", Self::CONTRAT_SELECT);
        self.conn
            .query_row(&sql, params![reference], Self::map_contrat_row)
            .map_err(DbError::from)
    }

    pub fn list_contrats_for_bien_id(&self, bien_id: &str) -> Result<Vec<ContratRow>, DbError> {
        let sql = format!(
            "{} WHERE c.bien_id = ?1 ORDER BY c.date_debut DESC",
            Self::CONTRAT_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![bien_id], Self::map_contrat_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_finance_by_reference(&self, reference: &str) -> Result<FinanceRow, DbError> {
        self.conn
            .query_row(
                &format!(
                    "{} WHERE f.reference = ?1",
                    Self::FINANCE_SELECT
                ),
                params![reference],
                Self::map_finance_row,
            )
            .map_err(DbError::from)
    }

    pub fn update_contrat(&self, id: &str, input: ContratInput<'_>) -> Result<ContratRow, DbError> {
        self.validate_contrat_logement(input.bien_id, input.logement_cle)?;
        let devise = self.resolve_contrat_devise(input.bien_id, input.devise)?;
        let logement = input.logement_cle.filter(|s| !s.is_empty());
        self.conn.execute(
            "UPDATE contrats SET reference=?1, bien_id=?2, locataire=?3, locataire_email=?4,
             locataire_telephone=?5, loyer_mensuel=?6, date_debut=?7, date_fin=?8, logement_cle=?9, devise=?10
             WHERE id=?11",
            params![
                input.reference,
                input.bien_id,
                input.locataire,
                input.locataire_email,
                input.locataire_telephone,
                input.loyer_mensuel,
                input.date_debut,
                input.date_fin,
                logement,
                devise,
                id
            ],
        )?;
        self.get_contrat(id)
    }

    pub fn delete_contrat(&self, id: &str) -> Result<(), DbError> {
        self.conn
            .execute("UPDATE finances SET contrat_id = NULL WHERE contrat_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM contrats WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn create_finance(&self, input: FinanceInput<'_>) -> Result<FinanceRow, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let devise = crate::monnaie_domaine::normalize_devise(input.devise);
        self.conn.execute(
            "INSERT INTO finances (id, reference, libelle, montant, type_operation, statut, date_echeance,
             bien_reference, devise, created_at, contrat_id, periode_annee, periode_mois)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                input.reference,
                input.libelle,
                input.montant,
                input.type_operation,
                input.statut,
                input.date_echeance,
                input.bien_reference,
                devise,
                now,
                input.contrat_id,
                input.periode_annee,
                input.periode_mois,
            ],
        )?;
        self.get_finance(&id)
    }

    pub fn update_finance(&self, id: &str, input: FinanceInput<'_>) -> Result<FinanceRow, DbError> {
        let devise = crate::monnaie_domaine::normalize_devise(input.devise);
        self.conn.execute(
            "UPDATE finances SET reference=?1, libelle=?2, montant=?3, type_operation=?4, statut=?5,
             date_echeance=?6, bien_reference=?7, devise=?8, contrat_id=?9, periode_annee=?10, periode_mois=?11
             WHERE id=?12",
            params![
                input.reference,
                input.libelle,
                input.montant,
                input.type_operation,
                input.statut,
                input.date_echeance,
                input.bien_reference,
                devise,
                input.contrat_id,
                input.periode_annee,
                input.periode_mois,
                id
            ],
        )?;
        self.get_finance(id)
    }

    pub fn delete_finance(&self, id: &str) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM finances WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Utilisateurs ---
    pub fn list_users(&self) -> Result<Vec<UserRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT u.id, u.nom, u.email, r.nom AS role_nom, r.id AS role_id, u.actif
             FROM users u JOIN roles r ON u.role_id = r.id ORDER BY u.nom",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UserRow {
                    id: row.get(0)?,
                    nom: row.get(1)?,
                    email: row.get(2)?,
                    role: row.get(3)?,
                    role_id: row.get(4)?,
                    actif: row.get::<_, i64>(5)? == 1,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_roles(&self) -> Result<Vec<RoleRow>, DbError> {
        let mut stmt = self.conn.prepare("SELECT id, nom FROM roles ORDER BY nom")?;
        let rows = stmt
            .query_map([], |row| Ok(RoleRow { id: row.get(0)?, nom: row.get(1)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_role(&self, nom: &str) -> Result<RoleRow, DbError> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err(DbError::Message("Nom du rôle requis.".to_string()));
        }
        let id = role_id_from_nom(nom);
        self.conn
            .execute("INSERT INTO roles (id, nom) VALUES (?1, ?2)", params![id, nom])
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    DbError::Message("Ce nom de rôle existe déjà.".to_string())
                } else {
                    DbError::Sqlite(e)
                }
            })?;
        Ok(RoleRow { id, nom: nom.to_string() })
    }

    pub fn update_role(&self, id: &str, nom: &str) -> Result<RoleRow, DbError> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err(DbError::Message("Nom du rôle requis.".to_string()));
        }
        let n = self
            .conn
            .execute("UPDATE roles SET nom = ?1 WHERE id = ?2", params![nom, id])
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    DbError::Message("Ce nom de rôle existe déjà.".to_string())
                } else {
                    DbError::Sqlite(e)
                }
            })?;
        if n == 0 {
            return Err(DbError::Message("Rôle introuvable.".to_string()));
        }
        Ok(RoleRow {
            id: id.to_string(),
            nom: nom.to_string(),
        })
    }

    pub fn delete_role(&self, id: &str) -> Result<(), DbError> {
        if is_protected_role(id) {
            return Err(DbError::Message(
                "Ce rôle système ne peut pas être supprimé.".to_string(),
            ));
        }
        let users: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE role_id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        if users > 0 {
            return Err(DbError::Message(format!(
                "Impossible de supprimer : {users} utilisateur(s) utilisent ce rôle."
            )));
        }
        let n = self
            .conn
            .execute("DELETE FROM roles WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(DbError::Message("Rôle introuvable.".to_string()));
        }
        Ok(())
    }

    pub fn count_users_for_role(&self, role_id: &str) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role_id = ?1",
                params![role_id],
                |r| r.get(0),
            )
            .map_err(DbError::from)
    }

    pub fn list_all_privileges(&self) -> Result<Vec<String>, DbError> {
        use std::collections::HashSet;
        let mut set: HashSet<String> = crate::privileges::system_privileges()
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT privilege FROM role_privileges ORDER BY privilege")?;
        let rows = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        for p in rows {
            if !crate::privileges::is_legacy_immo_privilege(&p) {
                set.insert(p);
            }
        }
        let mut out: Vec<String> = set.into_iter().collect();
        out.sort();
        Ok(out)
    }

    /// Rôles disposant d'au moins un privilège sur l'entité (`{entité}:*` ou `{entité}:…`).
    pub fn list_role_names_with_entity_access(&self, entity_key: &str) -> Result<Vec<String>, DbError> {
        let roles = self.list_roles_with_privileges()?;
        let mut names: Vec<String> = roles
            .into_iter()
            .filter(|r| crate::privileges::has_any_entity_privilege(&r.privileges, entity_key))
            .map(|r| r.nom)
            .collect();
        names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        names.dedup();
        Ok(names)
    }

    pub fn list_roles_with_privileges(&self) -> Result<Vec<RoleWithPrivilegesRow>, DbError> {
        let roles = self.list_roles()?;
        let mut out = Vec::with_capacity(roles.len());
        for role in roles {
            let privileges = self.get_role_privileges(&role.id)?;
            out.push(RoleWithPrivilegesRow {
                id: role.id,
                nom: role.nom,
                privileges,
            });
        }
        Ok(out)
    }

    pub fn update_role_privileges(
        &self,
        role_id: &str,
        privileges: &[String],
    ) -> Result<(), DbError> {
        let catalog: std::collections::HashSet<String> =
            self.list_all_privileges()?.into_iter().collect();
        for privilege in privileges {
            if crate::privileges::is_legacy_immo_privilege(privilege) {
                return Err(DbError::Message(format!(
                    "Le privilège « {privilege} » n'est plus utilisé (module immobilier retiré)."
                )));
            }
            if !catalog.contains(privilege) {
                return Err(DbError::Message(format!(
                    "Le privilège « {privilege} » n'existe pas. Les privilèges sont créés uniquement par les triggers (entités, sync DDA)."
                )));
            }
        }
        self.conn.execute(
            "DELETE FROM role_privileges WHERE role_id = ?1",
            params![role_id],
        )?;
        for privilege in privileges {
            self.conn.execute(
                "INSERT INTO role_privileges (role_id, privilege) VALUES (?1, ?2)",
                params![role_id, privilege],
            )?;
        }
        Ok(())
    }

    pub fn create_user(
        &self,
        nom: &str,
        email: &str,
        password: &str,
        role_id: &str,
    ) -> Result<UserRow, DbError> {
        let id = Uuid::new_v4().to_string();
        let password_hash = hash(password, DEFAULT_COST)
            .map_err(|e| DbError::Message(format!("Hash : {e}")))?;
        self.conn.execute(
            "INSERT INTO users (id, nom, email, password_hash, role_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, nom, email, password_hash, role_id],
        )?;
        self.get_user(&id)
    }

    pub fn update_user(
        &self,
        id: &str,
        nom: &str,
        email: &str,
        role_id: &str,
        actif: bool,
    ) -> Result<UserRow, DbError> {
        self.conn.execute(
            "UPDATE users SET nom=?1, email=?2, role_id=?3, actif=?4 WHERE id=?5",
            params![nom, email, role_id, actif as i64, id],
        )?;
        self.get_user(id)
    }

    pub fn reset_user_password_force_change(
        &self,
        user_id: &str,
        new_password: &str,
    ) -> Result<UserRow, DbError> {
        let password_hash = hash(new_password, DEFAULT_COST)
            .map_err(|e| DbError::Message(format!("Hash : {e}")))?;
        let affected = self.conn.execute(
            "UPDATE users
             SET password_hash = ?1, must_change_password = 1
             WHERE id = ?2 AND actif = 1",
            params![password_hash, user_id],
        )?;
        if affected == 0 {
            return Err(DbError::Message("Utilisateur introuvable ou inactif.".into()));
        }
        self.get_user(user_id)
    }

    pub fn get_user(&self, id: &str) -> Result<UserRow, DbError> {
        self.conn.query_row(
            "SELECT u.id, u.nom, u.email, r.nom, r.id, u.actif FROM users u JOIN roles r ON u.role_id = r.id WHERE u.id = ?1",
            params![id],
            |row| {
                Ok(UserRow {
                    id: row.get(0)?,
                    nom: row.get(1)?,
                    email: row.get(2)?,
                    role: row.get(3)?,
                    role_id: row.get(4)?,
                    actif: row.get::<_, i64>(5)? == 1,
                })
            },
        )
        .map_err(DbError::from)
    }

    // --- Finances ---
    const FINANCE_SELECT: &str = "SELECT f.id, f.reference, f.libelle, f.montant, f.type_operation, f.statut,
             f.date_echeance, f.bien_reference, f.devise, b.domaine AS bien_domaine,
             f.contrat_id, c.reference AS contrat_reference, c.locataire,
             f.periode_annee, f.periode_mois, f.date_paiement
             FROM finances f
             LEFT JOIN biens b ON b.reference = f.bien_reference
             LEFT JOIN contrats c ON c.id = f.contrat_id";

    fn map_finance_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FinanceRow> {
        Ok(FinanceRow {
            id: row.get(0)?,
            reference: row.get(1)?,
            libelle: row.get(2)?,
            montant: row.get(3)?,
            type_operation: row.get(4)?,
            statut: row.get(5)?,
            date_echeance: row.get(6)?,
            bien_reference: row.get(7)?,
            devise: row.get(8)?,
            bien_domaine: row.get(9)?,
            contrat_id: row.get(10)?,
            contrat_reference: row.get(11)?,
            locataire: row.get(12)?,
            periode_annee: row.get(13)?,
            periode_mois: row.get(14)?,
            date_paiement: row.get(15)?,
        })
    }

    pub fn list_finances(&self) -> Result<Vec<FinanceRow>, DbError> {
        let sql = format!("{} ORDER BY f.date_echeance DESC, f.reference", Self::FINANCE_SELECT);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], Self::map_finance_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn validate_finance(&self, id: &str) -> Result<FinanceRow, DbError> {
        let paid_on = Utc::now().format("%Y-%m-%d").to_string();
        self.conn.execute(
            "UPDATE finances SET statut = 'valide', date_paiement = ?2 WHERE id = ?1",
            params![id, paid_on],
        )?;
        self.get_finance(id)
    }

    pub fn get_finance(&self, id: &str) -> Result<FinanceRow, DbError> {
        let sql = format!("{} WHERE f.id = ?1", Self::FINANCE_SELECT);
        self.conn
            .query_row(&sql, params![id], Self::map_finance_row)
            .map_err(DbError::from)
    }

    fn parse_ymd_date(s: &str) -> Option<chrono::NaiveDate> {
        let trimmed = s.trim();
        if trimmed.len() >= 10 {
            chrono::NaiveDate::parse_from_str(&trimmed[..10], "%Y-%m-%d").ok()
        } else {
            None
        }
    }

    fn last_day_of_month(year: i32, month: u32) -> Option<chrono::NaiveDate> {
        if !(1..=12).contains(&month) {
            return None;
        }
        let next = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)?
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)?
        };
        next.pred_opt()
    }

    fn contrat_covers_period(contrat: &ContratRow, annee: i32, mois: u32) -> bool {
        let Some(period_start) = chrono::NaiveDate::from_ymd_opt(annee, mois, 1) else {
            return false;
        };
        let Some(period_end) = Self::last_day_of_month(annee, mois) else {
            return false;
        };
        let Some(debut) = Self::parse_ymd_date(&contrat.date_debut) else {
            return false;
        };
        if debut > period_end {
            return false;
        }
        if let Some(ref fin) = contrat.date_fin {
            if let Some(fin_date) = Self::parse_ymd_date(fin) {
                if fin_date < period_start {
                    return false;
                }
            }
        }
        true
    }

    fn mois_libelle_fr(mois: u32) -> &'static str {
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

    /// Génère les écritures de loyer pour chaque contrat actif couvrant la période (idempotent).
    pub fn generate_monthly_finances(
        &self,
        annee: i32,
        mois: u32,
    ) -> Result<GenerateMonthlyFinancesResult, DbError> {
        if !(1..=12).contains(&mois) {
            return Err(DbError::Message("Mois invalide (1-12)".into()));
        }
        let contrats = self.list_contrats()?;
        let mut created = 0u32;
        let mut skipped = 0u32;
        let mut contrats_eligibles = 0u32;

        for c in contrats {
            if c.statut != "actif" {
                continue;
            }
            if !Self::contrat_covers_period(&c, annee, mois) {
                continue;
            }
            contrats_eligibles += 1;

            let exists: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM finances WHERE contrat_id = ?1 AND periode_annee = ?2 AND periode_mois = ?3",
                params![c.id, annee, mois],
                |r| r.get(0),
            )?;
            if exists > 0 {
                skipped += 1;
                continue;
            }

            let reference = format!("LOY-{}-{:04}-{:02}", c.reference, annee, mois);
            let ref_exists: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM finances WHERE reference = ?1",
                params![reference],
                |r| r.get(0),
            )?;
            if ref_exists > 0 {
                skipped += 1;
                continue;
            }

            let libelle = format!(
                "Loyer {} — {} {}",
                c.locataire,
                Self::mois_libelle_fr(mois),
                annee
            );
            let date_echeance = format!("{:04}-{:02}-01", annee, mois);
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            self.conn.execute(
                "INSERT INTO finances (id, reference, libelle, montant, type_operation, statut, date_echeance,
                 bien_reference, devise, created_at, contrat_id, periode_annee, periode_mois)
                 VALUES (?1, ?2, ?3, ?4, 'loyer', 'en_attente', ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    id,
                    reference,
                    libelle,
                    c.loyer_mensuel,
                    date_echeance,
                    c.bien_reference,
                    c.devise,
                    now,
                    c.id,
                    annee,
                    mois,
                ],
            )?;
            created += 1;
        }

        Ok(GenerateMonthlyFinancesResult {
            created,
            skipped,
            contrats_eligibles,
            annee,
            mois,
        })
    }

    pub fn dashboard_stats(&self) -> Result<DashboardStats, DbError> {
        Ok(DashboardStats {
            biens_total: self.count_query("SELECT COUNT(*) FROM biens")?,
            biens_disponibles: self.count_query("SELECT COUNT(*) FROM biens WHERE statut = 'disponible'")?,
            hangars_libres: self.count_query(
                "SELECT COUNT(*) FROM biens WHERE type_bien = 'hangar' AND statut = 'libre'",
            )?,
            contrats_actifs: self.count_query("SELECT COUNT(*) FROM contrats WHERE statut = 'actif'")?,
            finances_en_attente: self
                .count_query("SELECT COUNT(*) FROM finances WHERE statut = 'en_attente'")?,
        })
    }

    fn count_query(&self, sql: &str) -> Result<i64, DbError> {
        self.conn.query_row(sql, [], |r| r.get(0)).map_err(DbError::from)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BienRow {
    pub id: String,
    pub reference: String,
    pub adresse: String,
    pub type_bien: String,
    pub statut: String,
    pub surface_m2: Option<f64>,
    pub prix_defaut: Option<f64>,
    pub nb_etages: Option<i32>,
    pub nb_chambres: Option<i32>,
    pub nb_pieces: Option<i32>,
    pub nomenclature: Vec<crate::bien_detail::BienEtageNom>,
    pub domaine: String,
    pub devise: String,
    pub zone: Option<String>,
    pub capacite_m3: Option<f64>,
}

pub struct BienInput<'a> {
    pub reference: &'a str,
    pub adresse: &'a str,
    pub type_bien: &'a str,
    pub statut: Option<&'a str>,
    pub surface_m2: Option<f64>,
    pub prix_defaut: Option<f64>,
    pub nb_etages: Option<i32>,
    pub nb_chambres: Option<i32>,
    pub nb_pieces: Option<i32>,
    pub nomenclature: &'a [crate::bien_detail::BienEtageNom],
    pub domaine: &'a str,
    pub devise: &'a str,
    pub zone: Option<&'a str>,
    pub capacite_m3: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct HangarRow {
    pub id: String,
    pub reference: String,
    pub zone: String,
    pub capacite_m3: f64,
    pub statut: String,
}

#[derive(Debug, serde::Serialize)]
pub struct HangarPhotoRow {
    pub id: String,
    pub hangar_id: String,
    pub filename: String,
    pub uploaded_at: String,
    pub source: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ContratRow {
    pub id: String,
    pub reference: String,
    pub locataire: String,
    pub locataire_email: String,
    pub locataire_telephone: String,
    pub loyer_mensuel: f64,
    pub date_debut: String,
    pub date_fin: Option<String>,
    pub statut: String,
    pub bien_reference: Option<String>,
    pub bien_id: Option<String>,
    pub bien_type: Option<String>,
    pub logement_cle: Option<String>,
    pub devise: String,
    pub bien_domaine: Option<String>,
}

pub struct ContratInput<'a> {
    pub reference: &'a str,
    pub bien_id: Option<&'a str>,
    pub locataire: &'a str,
    pub locataire_email: &'a str,
    pub locataire_telephone: &'a str,
    pub loyer_mensuel: f64,
    pub date_debut: &'a str,
    pub date_fin: Option<&'a str>,
    pub logement_cle: Option<&'a str>,
    pub devise: &'a str,
}

#[derive(Debug, serde::Serialize)]
pub struct UserRow {
    pub id: String,
    pub nom: String,
    pub email: String,
    pub role: String,
    pub role_id: String,
    pub actif: bool,
}

fn role_id_from_nom(nom: &str) -> String {
    let slug: String = nom
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                '_'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        format!("role-{}", uuid::Uuid::new_v4())
    } else {
        format!("role-{slug}")
    }
}

fn is_protected_role(id: &str) -> bool {
    matches!(
        id,
        "role-admin"
            | "role-agent"
            | "role-directeur"
            | "role-tech"
            | "role-compta"
    )
}

#[derive(Debug, serde::Serialize)]
pub struct RoleRow {
    pub id: String,
    pub nom: String,
}

#[derive(Debug, serde::Serialize)]
pub struct RoleWithPrivilegesRow {
    pub id: String,
    pub nom: String,
    pub privileges: Vec<String>,
}

pub struct FinanceInput<'a> {
    pub reference: &'a str,
    pub libelle: &'a str,
    pub montant: f64,
    pub type_operation: &'a str,
    pub statut: &'a str,
    pub date_echeance: &'a str,
    pub bien_reference: Option<&'a str>,
    pub devise: &'a str,
    pub contrat_id: Option<&'a str>,
    pub periode_annee: Option<i32>,
    pub periode_mois: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FinanceRow {
    pub id: String,
    pub reference: String,
    pub libelle: String,
    pub montant: f64,
    pub type_operation: String,
    pub statut: String,
    pub date_echeance: String,
    pub bien_reference: Option<String>,
    pub devise: String,
    pub bien_domaine: Option<String>,
    pub contrat_id: Option<String>,
    pub contrat_reference: Option<String>,
    pub locataire: Option<String>,
    pub periode_annee: Option<i32>,
    pub periode_mois: Option<i32>,
    pub date_paiement: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct GenerateMonthlyFinancesResult {
    pub created: u32,
    pub skipped: u32,
    pub contrats_eligibles: u32,
    pub annee: i32,
    pub mois: u32,
}

#[derive(Debug, serde::Serialize)]
pub struct DashboardStats {
    pub biens_total: i64,
    pub biens_disponibles: i64,
    pub hangars_libres: i64,
    pub contrats_actifs: i64,
    pub finances_en_attente: i64,
}
