use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::Utc;
use rusqlite::params;
use std::path::PathBuf;
use uuid::Uuid;

use crate::csv_util::push_row;
use crate::db::{Database, DbError};
use serde::Serialize;

pub const MAX_DOCUMENT_BYTES: usize = 12 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvTable {
    Biens,
    Contrats,
    Hangars,
    Finances,
}

impl CsvTable {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "biens" => Ok(Self::Biens),
            "contrats" => Ok(Self::Contrats),
            "hangars" => Ok(Self::Hangars),
            "finances" => Ok(Self::Finances),
            _ => Err(format!("Table CSV inconnue : {s}")),
        }
    }

    pub fn import_privilege(&self) -> &'static str {
        match self {
            Self::Biens => "biens:importer",
            Self::Contrats => "contrats:importer",
            Self::Hangars => "hangars:importer",
            Self::Finances => "finances:importer",
        }
    }

    pub fn export_privilege(&self) -> &'static str {
        match self {
            Self::Biens => "biens:exporter",
            Self::Contrats => "contrats:exporter",
            Self::Hangars => "hangars:exporter",
            Self::Finances => "finances:exporter",
        }
    }

    pub fn file_name(&self) -> &'static str {
        match self {
            Self::Biens => "blin_biens.csv",
            Self::Contrats => "blin_contrats.csv",
            Self::Hangars => "blin_hangars.csv",
            Self::Finances => "blin_finances.csv",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CsvImportResponse {
    pub success: bool,
    pub inserted: u32,
    pub updated: u32,
    pub error_count: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DocumentExportPayload {
    pub base64: String,
    pub mime: String,
    pub file_name: String,
    pub kind: String,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DocumentRow {
    pub id: String,
    pub original_name: String,
    pub kind: String,
    pub bytes: i64,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub caption: String,
    pub uploaded_at: String,
    pub source: String,
}

impl Database {
    pub fn migrate_v3(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY NOT NULL,
                original_name TEXT NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('png','jpeg','pdf')),
                bytes INTEGER NOT NULL,
                entity_type TEXT NOT NULL DEFAULT 'general',
                entity_id TEXT,
                caption TEXT NOT NULL DEFAULT '',
                uploaded_at TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'desktop'
            );
            "#,
        )?;
        std::fs::create_dir_all(self.documents_store_dir())
            .map_err(|e| DbError::Message(e.to_string()))?;
        self.ensure_io_privileges()?;
        Ok(())
    }

    fn ensure_io_privileges(&self) -> Result<(), DbError> {
        let grants: [(&str, &[&str]); 4] = [
            (
                "role-admin",
                &["*"],
            ),
            (
                "role-agent",
                &[
                    "documents:voir",
                    "documents:importer",
                    "documents:exporter",
                ],
            ),
            ("role-tech", &["documents:voir", "documents:importer", "documents:exporter"]),
            ("role-compta", &["documents:voir", "documents:exporter"]),
        ];
        for (role_id, privileges) in grants {
            for privilege in privileges {
                if *privilege == "*" {
                    continue;
                }
                let _ = self.conn.execute(
                    "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES (?1, ?2)",
                    params![role_id, privilege],
                );
            }
        }
        Ok(())
    }

    pub fn documents_store_dir(&self) -> PathBuf {
        self.data_dir.join("documents_store")
    }

    pub fn document_absolute_path(&self, id: &str, kind: &str) -> PathBuf {
        let ext = match kind {
            "png" => "png",
            "jpeg" => "jpg",
            "pdf" => "pdf",
            _ => "bin",
        };
        self.documents_store_dir().join(format!("{id}.{ext}"))
    }

    pub fn sniff_document_kind(data: &[u8]) -> Option<&'static str> {
        if data.len() >= 4 && data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
            return Some("png");
        }
        if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return Some("jpeg");
        }
        if data.len() >= 4 && data[0] == 0x25 && data[1] == 0x50 && data[2] == 0x44 && data[3] == 0x46 {
            return Some("pdf");
        }
        None
    }

    pub fn export_csv_table(&self, table: CsvTable) -> Result<String, DbError> {
        let mut w = String::from("\u{feff}");
        match table {
            CsvTable::Biens => {
                push_row(
                    &mut w,
                    &[
                        "reference".into(),
                        "adresse".into(),
                        "type_bien".into(),
                        "statut".into(),
                        "surface_m2".into(),
                        "prix_defaut".into(),
                        "nb_etages".into(),
                        "nb_chambres".into(),
                        "nb_pieces".into(),
                        "nomenclature_json".into(),
                        "domaine".into(),
                        "devise".into(),
                        "zone".into(),
                        "capacite_m3".into(),
                    ],
                );
                let biens = self.list_biens()?;
                for b in biens {
                    let nom_json = crate::bien_detail::serialize_nomenclature(&b.nomenclature);
                    push_row(
                        &mut w,
                        &[
                            b.reference,
                            b.adresse,
                            b.type_bien,
                            b.statut,
                            b.surface_m2.map(|s| s.to_string()).unwrap_or_default(),
                            b.prix_defaut.map(|p| p.to_string()).unwrap_or_default(),
                            b.nb_etages.map(|n| n.to_string()).unwrap_or_default(),
                            b.nb_chambres.map(|n| n.to_string()).unwrap_or_default(),
                            b.nb_pieces.map(|n| n.to_string()).unwrap_or_default(),
                            nom_json,
                            b.domaine,
                            b.devise,
                            b.zone.unwrap_or_default(),
                            b.capacite_m3.map(|c| c.to_string()).unwrap_or_default(),
                        ],
                    );
                }
            }
            CsvTable::Hangars => {
                push_row(
                    &mut w,
                    &[
                        "reference".into(),
                        "zone".into(),
                        "capacite_m3".into(),
                        "statut".into(),
                    ],
                );
                for h in self.list_hangars()? {
                    push_row(
                        &mut w,
                        &[
                            h.reference,
                            h.zone,
                            h.capacite_m3.to_string(),
                            h.statut,
                        ],
                    );
                }
            }
            CsvTable::Contrats => {
                push_row(
                    &mut w,
                    &[
                        "reference".into(),
                        "bien_reference".into(),
                        "locataire".into(),
                        "locataire_email".into(),
                        "locataire_telephone".into(),
                        "logement_cle".into(),
                        "loyer_mensuel".into(),
                        "date_debut".into(),
                        "date_fin".into(),
                        "statut".into(),
                        "devise".into(),
                    ],
                );
                for c in self.list_contrats()? {
                    push_row(
                        &mut w,
                        &[
                            c.reference,
                            c.bien_reference.unwrap_or_default(),
                            c.locataire,
                            c.locataire_email,
                            c.locataire_telephone,
                            c.logement_cle.unwrap_or_default(),
                            c.loyer_mensuel.to_string(),
                            c.date_debut,
                            c.date_fin.unwrap_or_default(),
                            c.statut,
                            c.devise,
                        ],
                    );
                }
            }
            CsvTable::Finances => {
                push_row(
                    &mut w,
                    &[
                        "reference".into(),
                        "libelle".into(),
                        "montant".into(),
                        "type_operation".into(),
                        "statut".into(),
                        "date_echeance".into(),
                        "bien_reference".into(),
                        "devise".into(),
                    ],
                );
                for f in self.list_finances()? {
                    push_row(
                        &mut w,
                        &[
                            f.reference,
                            f.libelle,
                            f.montant.to_string(),
                            f.type_operation,
                            f.statut,
                            f.date_echeance,
                            f.bien_reference.unwrap_or_default(),
                            f.devise,
                        ],
                    );
                }
            }
        }
        Ok(w)
    }

    pub fn import_csv_table(
        &self,
        table: CsvTable,
        csv_text: &str,
    ) -> Result<CsvImportResponse, String> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_text.as_bytes());

        let headers = reader
            .headers()
            .map_err(|e| e.to_string())?
            .iter()
            .map(|h| h.trim().to_lowercase())
            .collect::<Vec<_>>();

        let mut inserted = 0u32;
        let mut updated = 0u32;
        let mut errors: Vec<String> = Vec::new();

        for (line_no, result) in reader.records().enumerate() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    errors.push(format!("Ligne {} : {e}", line_no + 2));
                    continue;
                }
            };

            let get = |name: &str| -> String {
                headers
                    .iter()
                    .position(|h| h == name)
                    .and_then(|i| record.get(i))
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };

            let res = match table {
                CsvTable::Biens => {
                    let reference = get("reference");
                    if reference.is_empty() {
                        Err("reference manquante".to_string())
                    } else {
                        let exists: i64 = self
                            .conn
                            .query_row(
                                "SELECT COUNT(*) FROM biens WHERE reference = ?1",
                                params![reference],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        let surface: Option<f64> = get("surface_m2").parse().ok();
                        let prix: Option<f64> = get("prix_defaut").parse().ok();
                        let nb_etages: Option<i32> = get("nb_etages").parse().ok();
                        let nb_chambres: Option<i32> = get("nb_chambres").parse().ok();
                        let nb_pieces: Option<i32> = get("nb_pieces").parse().ok();
                        let nom_raw = get("nomenclature_json");
                        let nomenclature = crate::bien_detail::parse_nomenclature_json(&nom_raw);
                        let type_bien = get("type_bien");
                        let zone_val = if get("zone").is_empty() {
                            None
                        } else {
                            Some(get("zone"))
                        };
                        let capacite_val: Option<f64> = get("capacite_m3").parse().ok();
                        let statut_val = if get("statut").is_empty() {
                            if type_bien == "hangar" {
                                "libre".to_string()
                            } else {
                                "disponible".to_string()
                            }
                        } else {
                            get("statut")
                        };
                        let statut = statut_val.as_str();
                        let domaine_raw = if get("domaine").is_empty() {
                            crate::monnaie_domaine::default_domaine_for_type(&type_bien).to_string()
                        } else {
                            get("domaine")
                        };
                        let devise_raw = if get("devise").is_empty() {
                            "EUR".to_string()
                        } else {
                            get("devise")
                        };
                        let input = crate::db::BienInput {
                            reference: &reference,
                            adresse: &get("adresse"),
                            type_bien: &type_bien,
                            statut: Some(statut),
                            surface_m2: surface,
                            prix_defaut: prix,
                            nb_etages,
                            nb_chambres,
                            nb_pieces,
                            nomenclature: &nomenclature,
                            domaine: domaine_raw.as_str(),
                            devise: devise_raw.as_str(),
                            zone: zone_val.as_deref(),
                            capacite_m3: capacite_val,
                        };
                        if exists > 0 {
                            let id: String = self
                                .conn
                                .query_row(
                                    "SELECT id FROM biens WHERE reference = ?1",
                                    params![reference],
                                    |r| r.get(0),
                                )
                                .map_err(|e| e.to_string())?;
                            self.update_bien(&id, input).map_err(|e| e.to_string())?;
                            Ok(false)
                        } else {
                            self.create_bien(input).map_err(|e| e.to_string())?;
                            Ok(true)
                        }
                    }
                }
                CsvTable::Hangars => {
                    let reference = get("reference");
                    if reference.is_empty() {
                        Err("reference manquante".to_string())
                    } else {
                        let exists: i64 = self
                            .conn
                            .query_row(
                                "SELECT COUNT(*) FROM biens WHERE reference = ?1 AND type_bien = 'hangar'",
                                params![reference],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        let capacite: f64 = get("capacite_m3").parse().unwrap_or(0.0);
                        let statut = if get("statut").is_empty() {
                            "libre".to_string()
                        } else {
                            get("statut")
                        };
                        if exists > 0 {
                            let id: String = self.conn.query_row(
                                "SELECT id FROM biens WHERE reference = ?1 AND type_bien = 'hangar'",
                                params![reference],
                                |r| r.get(0),
                            ).map_err(|e| e.to_string())?;
                            self.update_hangar(
                                &id,
                                &reference,
                                &get("zone"),
                                capacite,
                                &statut,
                            )
                            .map_err(|e| e.to_string())?;
                            Ok(false)
                        } else {
                            self.create_hangar(&reference, &get("zone"), capacite)
                                .map_err(|e| e.to_string())?;
                            Ok(true)
                        }
                    }
                }
                CsvTable::Contrats => {
                    let reference = get("reference");
                    if reference.is_empty() {
                        Err("reference manquante".to_string())
                    } else {
                        let exists: i64 = self
                            .conn
                            .query_row(
                                "SELECT COUNT(*) FROM contrats WHERE reference = ?1",
                                params![reference],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        let bien_id = if get("bien_reference").is_empty() {
                            None
                        } else {
                            self.conn
                                .query_row(
                                    "SELECT id FROM biens WHERE reference = ?1 LIMIT 1",
                                    params![get("bien_reference")],
                                    |r| r.get::<_, String>(0),
                                )
                                .ok()
                        };
                        let loyer: f64 = get("loyer_mensuel").parse().unwrap_or(0.0);
                        let date_fin = if get("date_fin").is_empty() {
                            None
                        } else {
                            Some(get("date_fin"))
                        };
                        let logement_owned = if get("logement_cle").is_empty() {
                            None
                        } else {
                            Some(get("logement_cle"))
                        };
                        let logement = logement_owned.as_deref();
                        let email_owned = if get("locataire_email").is_empty() {
                            "contact@locataire.local".to_string()
                        } else {
                            get("locataire_email")
                        };
                        let tel_owned = if get("locataire_telephone").is_empty() {
                            "0000000000".to_string()
                        } else {
                            get("locataire_telephone")
                        };
                        let devise_owned = get("devise");
                        let input = crate::db::ContratInput {
                            reference: &reference,
                            bien_id: bien_id.as_deref(),
                            locataire: &get("locataire"),
                            locataire_email: email_owned.as_str(),
                            locataire_telephone: tel_owned.as_str(),
                            loyer_mensuel: loyer,
                            date_debut: &get("date_debut"),
                            date_fin: date_fin.as_deref(),
                            logement_cle: logement,
                            devise: devise_owned.as_str(),
                        };
                        if exists > 0 {
                            self.validate_contrat_logement(bien_id.as_deref(), logement)
                                .map_err(|e| e.to_string())?;
                            self.conn.execute(
                                "UPDATE contrats SET bien_id=?1, locataire=?2, locataire_email=?3,
                                 locataire_telephone=?4, logement_cle=?5, loyer_mensuel=?6, date_debut=?7,
                                 date_fin=?8, statut=?9, devise=?10 WHERE reference=?11",
                                params![
                                    bien_id,
                                    get("locataire"),
                                    get("locataire_email"),
                                    get("locataire_telephone"),
                                    logement_owned,
                                    loyer,
                                    get("date_debut"),
                                    date_fin,
                                    get("statut"),
                                    crate::monnaie_domaine::normalize_devise(&devise_owned),
                                    reference
                                ],
                            )
                            .map_err(|e| e.to_string())?;
                            Ok(false)
                        } else {
                            self.create_contrat(input).map_err(|e| e.to_string())?;
                            Ok(true)
                        }
                    }
                }
                CsvTable::Finances => {
                    let reference = get("reference");
                    if reference.is_empty() {
                        Err("reference manquante".to_string())
                    } else {
                        let exists: i64 = self
                            .conn
                            .query_row(
                                "SELECT COUNT(*) FROM finances WHERE reference = ?1",
                                params![reference],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        let montant: f64 = get("montant").parse().unwrap_or(0.0);
                        let bien_ref = if get("bien_reference").is_empty() {
                            None
                        } else {
                            Some(get("bien_reference"))
                        };
                        let devise_raw = get("devise");
                        let devise = if devise_raw.is_empty() {
                            if let Some(ref br) = bien_ref {
                                self.conn
                                    .query_row(
                                        "SELECT devise FROM biens WHERE reference = ?1",
                                        params![br],
                                        |r| r.get::<_, String>(0),
                                    )
                                    .unwrap_or_else(|_| "EUR".to_string())
                            } else {
                                "EUR".to_string()
                            }
                        } else {
                            devise_raw
                        };
                        let devise = crate::monnaie_domaine::normalize_devise(&devise);
                        if exists > 0 {
                            self.conn.execute(
                                "UPDATE finances SET libelle=?1, montant=?2, type_operation=?3, statut=?4, date_echeance=?5, bien_reference=?6, devise=?7 WHERE reference=?8",
                                params![
                                    get("libelle"),
                                    montant,
                                    get("type_operation"),
                                    get("statut"),
                                    get("date_echeance"),
                                    bien_ref,
                                    devise,
                                    reference
                                ],
                            )
                            .map_err(|e| e.to_string())?;
                            Ok(false)
                        } else {
                            let id = Uuid::new_v4().to_string();
                            let now = Utc::now().to_rfc3339();
                            self.conn.execute(
                                "INSERT INTO finances (id, reference, libelle, montant, type_operation, statut, date_echeance, bien_reference, devise, created_at)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                                params![
                                    id,
                                    reference,
                                    get("libelle"),
                                    montant,
                                    get("type_operation"),
                                    get("statut"),
                                    get("date_echeance"),
                                    bien_ref,
                                    devise,
                                    now
                                ],
                            )
                            .map_err(|e| e.to_string())?;
                            Ok(true)
                        }
                    }
                }
            };

            match res {
                Ok(true) => inserted += 1,
                Ok(false) => updated += 1,
                Err(msg) => {
                    if errors.len() < 25 {
                        errors.push(format!("Ligne {} : {msg}", line_no + 2));
                    }
                }
            }
        }

        let error_count = errors.len() as u32;
        Ok(CsvImportResponse {
            success: error_count == 0,
            inserted,
            updated,
            error_count,
            errors,
        })
    }

    pub fn list_documents(
        &self,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
    ) -> Result<Vec<DocumentRow>, DbError> {
        let mut sql = String::from(
            "SELECT id, original_name, kind, bytes, entity_type, entity_id, caption, uploaded_at, source
             FROM documents WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(et) = entity_type {
            sql.push_str(" AND entity_type = ?");
            params_vec.push(Box::new(et.to_string()));
        }
        if let Some(eid) = entity_id {
            sql.push_str(" AND entity_id = ?");
            params_vec.push(Box::new(eid.to_string()));
        }
        sql.push_str(" ORDER BY uploaded_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(DocumentRow {
                    id: row.get(0)?,
                    original_name: row.get(1)?,
                    kind: row.get(2)?,
                    bytes: row.get(3)?,
                    entity_type: row.get(4)?,
                    entity_id: row.get(5)?,
                    caption: row.get(6)?,
                    uploaded_at: row.get(7)?,
                    source: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn import_document(
        &self,
        original_name: &str,
        raw: &[u8],
        entity_type: &str,
        entity_id: Option<&str>,
        caption: &str,
        source: &str,
    ) -> Result<DocumentRow, String> {
        if raw.len() > MAX_DOCUMENT_BYTES {
            return Err(format!(
                "Fichier trop volumineux (max {} Mo)",
                MAX_DOCUMENT_BYTES / 1024 / 1024
            ));
        }
        let kind = Self::sniff_document_kind(raw)
            .ok_or("Format non reconnu (PNG, JPEG ou PDF requis)")?
            .to_string();

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let path = self.document_absolute_path(&id, &kind);
        std::fs::write(&path, raw).map_err(|e| e.to_string())?;

        self.conn
            .execute(
                "INSERT INTO documents (id, original_name, kind, bytes, entity_type, entity_id, caption, uploaded_at, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    original_name,
                    kind,
                    raw.len() as i64,
                    entity_type,
                    entity_id,
                    caption,
                    now,
                    source
                ],
            )
            .map_err(|e| e.to_string())?;

        Ok(DocumentRow {
            id,
            original_name: original_name.to_string(),
            kind,
            bytes: raw.len() as i64,
            entity_type: entity_type.to_string(),
            entity_id: entity_id.map(|s| s.to_string()),
            caption: caption.to_string(),
            uploaded_at: now,
            source: source.to_string(),
        })
    }

    pub fn export_document(
        &self,
        id: &str,
    ) -> Result<DocumentExportPayload, String> {
        let (original_name, kind): (String, String) = self
            .conn
            .query_row(
                "SELECT original_name, kind FROM documents WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| "Document introuvable".to_string())?;

        let path = self.document_absolute_path(id, &kind);
        let raw = std::fs::read(&path).map_err(|e| e.to_string())?;
        let mime = match kind.as_str() {
            "png" => "image/png",
            "jpeg" => "image/jpeg",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        };
        let file_name = if original_name.to_lowercase().ends_with(&format!(".{}", kind)) {
            original_name
        } else if kind == "jpeg" {
            format!("{original_name}.jpg")
        } else {
            format!("{original_name}.{kind}")
        };

        Ok(DocumentExportPayload {
            base64: B64.encode(&raw),
            mime: mime.to_string(),
            file_name,
            kind,
        })
    }

    pub fn delete_documents_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<(), DbError> {
        let docs = self.list_documents(Some(entity_type), Some(entity_id))?;
        for d in docs {
            let _ = self.delete_document(&d.id);
        }
        Ok(())
    }

    pub fn delete_document(&self, id: &str) -> Result<(), String> {
        let kind: String = self
            .conn
            .query_row(
                "SELECT kind FROM documents WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|_| "Document introuvable".to_string())?;
        let path = self.document_absolute_path(id, &kind);
        let _ = std::fs::remove_file(&path);
        self.conn
            .execute("DELETE FROM documents WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn migrate_v7(&self) -> Result<(), DbError> {
        let alters = [
            "ALTER TABLE biens ADD COLUMN domaine TEXT NOT NULL DEFAULT 'habitation'",
            "ALTER TABLE biens ADD COLUMN devise TEXT NOT NULL DEFAULT 'EUR'",
            "ALTER TABLE contrats ADD COLUMN devise TEXT NOT NULL DEFAULT 'EUR'",
            "ALTER TABLE finances ADD COLUMN devise TEXT NOT NULL DEFAULT 'EUR'",
        ];
        for sql in alters {
            let _ = self.conn.execute(sql, []);
        }
        let _ = self.conn.execute(
            "UPDATE biens SET domaine = 'commercial' WHERE type_bien = 'local_commercial' AND domaine = 'habitation'",
            [],
        );
        let _ = self.conn.execute(
            "UPDATE biens SET domaine = 'foncier' WHERE type_bien = 'terrain' AND domaine = 'habitation'",
            [],
        );
        let _ = self.conn.execute(
            "UPDATE contrats SET devise = (SELECT devise FROM biens WHERE biens.id = contrats.bien_id)
             WHERE bien_id IS NOT NULL AND bien_id != ''",
            [],
        );
        let _ = self.conn.execute(
            "UPDATE finances SET devise = COALESCE(
                (SELECT devise FROM biens WHERE biens.reference = finances.bien_reference), 'EUR'
             )",
            [],
        );
        Ok(())
    }

    pub fn migrate_v8(&self) -> Result<(), DbError> {
        let _ = self.conn.execute("ALTER TABLE biens ADD COLUMN zone TEXT", []);
        let _ = self.conn.execute("ALTER TABLE biens ADD COLUMN capacite_m3 REAL", []);
        let _ = self.conn.execute(
            "INSERT INTO biens (id, reference, adresse, type_bien, statut, capacite_m3, zone, domaine, devise,
             nomenclature_json, created_at)
             SELECT h.id, h.reference, 'Zone ' || h.zone || ' — hangar', 'hangar', h.statut, h.capacite_m3, h.zone,
             'commercial', 'EUR', '[]', h.created_at
             FROM hangars h
             WHERE NOT EXISTS (SELECT 1 FROM biens b WHERE b.id = h.id OR b.reference = h.reference)",
            [],
        );
        let _ = self.conn.execute(
            "UPDATE documents SET entity_type = 'bien' WHERE entity_type = 'hangar'",
            [],
        );
        Ok(())
    }

    pub fn migrate_v11(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS ai_experience (
                id TEXT PRIMARY KEY NOT NULL,
                message_norm TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                params_json TEXT NOT NULL DEFAULT '{}',
                summary TEXT NOT NULL,
                outcome TEXT NOT NULL DEFAULT 'success',
                use_count INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                last_used_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ai_experience_norm ON ai_experience(message_norm);
            CREATE INDEX IF NOT EXISTS idx_ai_experience_tool ON ai_experience(tool_name);
            "#,
        )?;
        Ok(())
    }

    pub fn migrate_v10(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS ai_runtime_profile (
                id TEXT PRIMARY KEY NOT NULL,
                backend TEXT NOT NULL,
                gpu_layers INTEGER NOT NULL,
                ctx_size INTEGER NOT NULL,
                threads INTEGER NOT NULL,
                load_ms INTEGER NOT NULL,
                profiled_at TEXT NOT NULL,
                bin_fingerprint TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn migrate_v9(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS ai_chunks (
                id TEXT PRIMARY KEY NOT NULL,
                source TEXT NOT NULL,
                content TEXT NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS ai_fts USING fts5(
                chunk_id UNINDEXED,
                source UNINDEXED,
                content
            );
            CREATE TABLE IF NOT EXISTS ai_conversations (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ai_messages (
                id TEXT PRIMARY KEY NOT NULL,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ai_pending_actions (
                id TEXT PRIMARY KEY NOT NULL,
                conversation_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                params_json TEXT NOT NULL,
                privilege TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )?;
        for role_id in ["role-admin", "role-agent", "role-tech", "role-compta"] {
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES (?1, 'ai:utiliser')",
                params![role_id],
            );
        }
        Ok(())
    }

    pub fn migrate_v6(&self) -> Result<(), DbError> {
        let alters = [
            "ALTER TABLE contrats ADD COLUMN locataire_email TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE contrats ADD COLUMN locataire_telephone TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE contrats ADD COLUMN logement_cle TEXT",
        ];
        for sql in alters {
            let _ = self.conn.execute(sql, []);
        }
        let _ = self.conn.execute(
            "UPDATE contrats SET locataire_email = 'martin.dupont@email.fr',
             locataire_telephone = '06 12 34 56 78' WHERE reference = 'CTR-2024-001' AND locataire_email = ''",
            [],
        );
        Ok(())
    }

    pub fn migrate_v5(&self) -> Result<(), DbError> {
        let alters = [
            "ALTER TABLE biens ADD COLUMN prix_defaut REAL",
            "ALTER TABLE biens ADD COLUMN nb_etages INTEGER",
            "ALTER TABLE biens ADD COLUMN nb_chambres INTEGER",
            "ALTER TABLE biens ADD COLUMN nb_pieces INTEGER",
            "ALTER TABLE biens ADD COLUMN nomenclature_json TEXT NOT NULL DEFAULT '[]'",
        ];
        for sql in alters {
            let _ = self.conn.execute(sql, []);
        }
        self.patch_demo_biens_details()?;
        Ok(())
    }

    fn patch_demo_biens_details(&self) -> Result<(), DbError> {
        let demo_immeuble_nom = r#"[{"numero":0,"libelle":"RDC","chambres":[{"code":"CH-01","nom":"Studio RDC","pieces":[{"code":"P-01","nom":"Séjour","type_piece":"séjour"},{"code":"P-02","nom":"Cuisine","type_piece":"cuisine"}]}],"pieces_communes":[{"code":"PC-01","nom":"Hall","type_piece":"commun"}]},{"numero":1,"libelle":"1er étage","chambres":[{"code":"CH-02","nom":"T2 A","pieces":[{"code":"P-03","nom":"Salon","type_piece":"séjour"},{"code":"P-04","nom":"Chambre","type_piece":"chambre"}]},{"code":"CH-03","nom":"T2 B","pieces":[{"code":"P-05","nom":"Salon","type_piece":"séjour"}]}],"pieces_communes":[]}]"#;
        let patches: [(&str, Option<f64>, Option<i32>, Option<i32>, Option<i32>, &str); 4] = [
            ("BIEN-001", Some(850.0), None, Some(2), Some(4), "[]"),
            ("BIEN-002", Some(2500.0), Some(1), None, Some(3), "[]"),
            ("BIEN-003", Some(1200.0), Some(2), Some(4), Some(6), "[]"),
            ("BIEN-004", Some(45000.0), Some(2), Some(3), Some(7), demo_immeuble_nom),
        ];
        for (reference, prix, etages, chambres, pieces, nom) in patches {
            let _ = self.conn.execute(
                "UPDATE biens SET prix_defaut = ?1, nb_etages = ?2, nb_chambres = ?3, nb_pieces = ?4,
                 nomenclature_json = ?5 WHERE reference = ?6",
                params![prix, etages, chambres, pieces, nom, reference],
            );
        }
        Ok(())
    }

    pub fn migrate_v4(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS document_print_models (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL DEFAULT '',
                html_content TEXT NOT NULL DEFAULT '',
                css_content TEXT NOT NULL DEFAULT '',
                screen_key TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;
        self.ensure_print_model_privileges()?;
        self.ensure_all_screen_print_models()?;
        Ok(())
    }

    pub fn migrate_v12(&self) -> Result<(), DbError> {
        self.ensure_all_screen_print_models()?;
        Ok(())
    }

    pub fn migrate_v13(&self) -> Result<(), DbError> {
        let _ = self.conn.execute(
            "ALTER TABLE finances ADD COLUMN contrat_id TEXT REFERENCES contrats(id)",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE finances ADD COLUMN periode_annee INTEGER",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE finances ADD COLUMN periode_mois INTEGER",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE finances ADD COLUMN date_paiement TEXT",
            [],
        );
        self.conn.execute_batch(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_finances_contrat_periode
                ON finances(contrat_id, periode_annee, periode_mois)
                WHERE contrat_id IS NOT NULL
                  AND periode_annee IS NOT NULL
                  AND periode_mois IS NOT NULL;
            "#,
        )?;
        Ok(())
    }

    pub fn migrate_v14(&self) -> Result<(), DbError> {
        let _ = self.conn.execute(
            "ALTER TABLE ai_pending_actions ADD COLUMN confirm_privilege TEXT",
            [],
        );
        self.ensure_directeur_role()?;
        Ok(())
    }

    /// Renommage LoggImmo → LoggMagic : e-mail admin historique.
    pub fn migrate_v15(&self) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE users SET email = 'admin@loggmagic.local'
             WHERE email = 'admin@loggimmo.local'
             AND NOT EXISTS (SELECT 1 FROM users WHERE email = 'admin@loggmagic.local')",
            [],
        )?;
        Ok(())
    }

    /// Renommage LoggMagic → Blin : e-mails locaux des comptes seed.
    pub fn migrate_v18(&self) -> Result<(), DbError> {
        for (from_suffix, to_email) in [
            ("admin@loggmagic.local", "admin@blin.local"),
            ("bureau@loggmagic.local", "bureau@blin.local"),
        ] {
            self.conn.execute(
                &format!(
                    "UPDATE users SET email = ?1
                     WHERE email = ?2
                     AND NOT EXISTS (SELECT 1 FROM users WHERE email = ?1)"
                ),
                params![to_email, from_suffix],
            )?;
        }
        Ok(())
    }

    /// Changement de mot de passe obligatoire à la première connexion.
    pub fn migrate_v16(&self) -> Result<(), DbError> {
        let _ = self.conn.execute(
            "ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0",
            [],
        );
        self.flag_default_admin_passwords()?;
        Ok(())
    }

    /// Retire les privilèges du module immobilier (biens, finances, hangars, contrats).
    pub fn migrate_v17(&self) -> Result<(), DbError> {
        self.purge_legacy_immo_privileges()?;
        Ok(())
    }

    fn purge_legacy_immo_privileges(&self) -> Result<(), DbError> {
        for pattern in ["biens:%", "finances:%", "hangars:%", "contrats:%"] {
            self.conn.execute(
                "DELETE FROM role_privileges WHERE privilege LIKE ?1",
                params![pattern],
            )?;
        }
        Ok(())
    }

    /// Marque les comptes encore sur mot de passe d'usine (admin1234 ou ancien Admin123!).
    fn flag_default_admin_passwords(&self) -> Result<(), DbError> {
        use bcrypt::verify;

        let candidates = [
            crate::db::DEFAULT_ADMIN_PASSWORD,
            "Admin123!",
        ];
        let mut stmt = self.conn.prepare(
            "SELECT id, password_hash FROM users WHERE email = ?1 AND actif = 1",
        )?;
        let rows = stmt.query_map(params![crate::db::DEFAULT_ADMIN_EMAIL], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, password_hash) = row?;
            let still_default = candidates
                .iter()
                .any(|plain| verify(plain, &password_hash).unwrap_or(false));
            if still_default {
                self.conn.execute(
                    "UPDATE users SET must_change_password = 1 WHERE id = ?1",
                    params![id],
                )?;
            }
        }
        Ok(())
    }

    /// Rôle Directeur + privilège de confirmation des actions sensibles Loggy.
    fn ensure_directeur_role(&self) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO roles (id, nom) VALUES ('role-directeur', 'Directeur')",
            [],
        )?;
        let privileges = [
            "directeur:confirmer",
            "documents:voir",
            "documents:exporter",
            "ai:utiliser",
            "users:voir",
            "users:modifier",
        ];
        for privilege in privileges {
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES ('role-directeur', ?1)",
                params![privilege],
            );
        }
        Ok(())
    }

    fn ensure_print_model_privileges(&self) -> Result<(), DbError> {
        let grants: [(&str, &[&str]); 5] = [
            ("role-admin", &["documents:modeles_voir", "documents:modeles_gerer"]),
            (
                "role-directeur",
                &["documents:modeles_voir", "documents:modeles_gerer"],
            ),
            ("role-agent", &["documents:modeles_voir"]),
            ("role-tech", &["documents:modeles_voir"]),
            ("role-compta", &["documents:modeles_voir"]),
        ];
        for (role_id, privileges) in grants {
            for privilege in privileges {
                let _ = self.conn.execute(
                    "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES (?1, ?2)",
                    params![role_id, privilege],
                );
            }
        }
        Ok(())
    }

    /// Insère un modèle par écran (`screen_key`) s'il n'existe pas encore.
    pub fn ensure_all_screen_print_models(&self) -> Result<(), DbError> {
        use crate::print_seed::{ALL_SCREEN_MODELS, LIST_PRINT_CSS, PRINT_CSS, PRINT_HTML};
        let (app_name, slogan) = crate::entity::branding::load_branding(&self.data_dir);
        let html_content = PRINT_HTML
            .replace("LoggMagic", &app_name)
            .replace("Blin", &app_name)
            .replace("Gestion immobilière", &slogan);
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        for seed in ALL_SCREEN_MODELS {
            let model_name = seed
                .name
                .replace("LoggMagic", &app_name)
                .replace("Blin", &app_name);
            let html = if seed.screen_key == "stock" {
                crate::print_template::build_stock_list_print_html()
            } else {
                html_content.clone()
            };
            let _ = self.ensure_list_print_model_for_screen(
                seed.screen_key,
                &model_name,
                seed.description,
                &html,
                LIST_PRINT_CSS,
            )?;
        }
        Ok(())
    }

    /// Nom unique pour `document_print_models.name` (contrainte globale, pas par écran).
    pub fn disambiguate_print_model_name(
        &self,
        base: &str,
        screen_key: &str,
    ) -> Result<String, DbError> {
        let taken: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM document_print_models WHERE name = ?1",
            params![base],
            |r| r.get(0),
        )?;
        if taken == 0 {
            return Ok(base.to_string());
        }
        let same_screen: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM document_print_models WHERE name = ?1 AND screen_key = ?2",
            params![base, screen_key],
            |r| r.get(0),
        )?;
        if same_screen > 0 {
            return Ok(base.to_string());
        }
        let alt = format!("{base} ({screen_key})");
        let alt_taken: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM document_print_models WHERE name = ?1",
            params![alt],
            |r| r.get(0),
        )?;
        if alt_taken == 0 {
            return Ok(alt);
        }
        Ok(alt)
    }

    /// Renomme les modèles dont le nom global est en doublon (contrainte UNIQUE sur `name`).
    pub fn dedupe_print_model_names(&self) -> Result<u32, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM document_print_models GROUP BY name HAVING COUNT(*) > 1",
        )?;
        let dup_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .flatten()
            .collect();

        let mut renamed = 0u32;
        for name in dup_names {
            let mut rows_stmt = self.conn.prepare(
                "SELECT id, screen_key FROM document_print_models WHERE name = ?1 ORDER BY created_at ASC",
            )?;
            let rows: Vec<(String, String)> = rows_stmt
                .query_map(params![name], |row| Ok((row.get(0)?, row.get(1)?)))?
                .flatten()
                .collect();
            for (id, screen_key) in rows.into_iter().skip(1) {
                let unique = self.disambiguate_print_model_name(&name, &screen_key)?;
                if unique == name {
                    continue;
                }
                self.conn.execute(
                    "UPDATE document_print_models SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
                    params![unique, id],
                )?;
                renamed += 1;
            }
        }
        Ok(renamed)
    }

    /// Met à jour ou insère un modèle auto DDA (HTML/CSS) — rafraîchit l'apparence à chaque sync.
    pub fn sync_auto_print_model(
        &self,
        screen_key: &str,
        name: &str,
        description: &str,
        html_content: &str,
        css_content: &str,
        kind_hint: &str,
    ) -> Result<(), DbError> {
        use crate::print_seed::AUTO_PRINT_DESCRIPTION_PREFIX;
        use rusqlite::OptionalExtension;

        let like_desc = format!("{AUTO_PRINT_DESCRIPTION_PREFIX}%");
        let like_name = format!("%{kind_hint}%");
        let existing_id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM document_print_models
                 WHERE screen_key = ?1
                   AND (description LIKE ?2 OR name LIKE ?3 OR description LIKE '%généré DDA%')
                 ORDER BY created_at ASC LIMIT 1",
                params![screen_key, like_desc, like_name],
                |r| r.get(0),
            )
            .optional()?;

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if let Some(id) = existing_id {
            self.conn.execute(
                "UPDATE document_print_models
                 SET html_content = ?1, css_content = ?2, description = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![html_content, css_content, description, &now, id],
            )?;
            return Ok(());
        }

        let unique_name = self.disambiguate_print_model_name(name, screen_key)?;
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO document_print_models (id, name, description, html_content, css_content, screen_key, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                unique_name,
                description,
                html_content,
                css_content,
                screen_key,
                &now,
                &now,
            ],
        )?;
        Ok(())
    }

    /// Insère un modèle « Liste » pour l'écran s'il n'existe pas encore (ne remplace pas les modèles utilisateur).
    pub fn ensure_list_print_model_for_screen(
        &self,
        screen_key: &str,
        name: &str,
        description: &str,
        html_content: &str,
        css_content: &str,
    ) -> Result<(), DbError> {
        self.sync_auto_print_model(screen_key, name, description, html_content, css_content, "Liste")
    }

    /// Modèle liste tabulaire pour un écran (priorité au nom « Liste »).
    pub fn get_list_print_model_for_screen(
        &self,
        screen_key: &str,
    ) -> Result<Option<PrintModelDetail>, DbError> {
        use rusqlite::OptionalExtension;
        let id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM document_print_models WHERE screen_key = ?1
                 ORDER BY CASE WHEN name LIKE '%Liste%' THEN 0 WHEN name LIKE '%liste%' THEN 0 ELSE 1 END, updated_at DESC
                 LIMIT 1",
                params![screen_key],
                |r| r.get(0),
            )
            .optional()?;
        match id {
            Some(i) => Ok(Some(self.get_print_model(&i)?)),
            None => Ok(None),
        }
    }

    pub fn list_print_models(&self) -> Result<Vec<PrintModelRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, screen_key, created_at, updated_at
             FROM document_print_models ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(PrintModelRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    screen_key: r.get(3)?,
                    created_at: r.get(4)?,
                    updated_at: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_print_model(&self, id: &str) -> Result<PrintModelDetail, DbError> {
        self.conn
            .query_row(
                "SELECT id, name, description, html_content, css_content, screen_key, created_at, updated_at
                 FROM document_print_models WHERE id = ?1",
                params![id],
                |r| {
                    Ok(PrintModelDetail {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        description: r.get(2)?,
                        html_content: r.get(3)?,
                        css_content: r.get(4)?,
                        screen_key: r.get(5)?,
                        created_at: r.get(6)?,
                        updated_at: r.get(7)?,
                    })
                },
            )
            .map_err(|_| DbError::Message("Modèle introuvable".to_string()))
    }

    pub fn upsert_print_model(
        &self,
        id_in: Option<&str>,
        name: &str,
        description: &str,
        html_content: &str,
        css_content: &str,
        screen_key: Option<&str>,
    ) -> Result<PrintModelDetail, DbError> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let sk: Option<&str> = screen_key.filter(|s| !s.is_empty());

        if let Some(id) = id_in.filter(|s| !s.is_empty()) {
            self.conn
                .execute(
                    "UPDATE document_print_models SET name = ?1, description = ?2, html_content = ?3,
                     css_content = ?4, screen_key = ?5, updated_at = ?6 WHERE id = ?7",
                    params![name, description, html_content, css_content, sk, now, id],
                )
                .map_err(|e| {
                    if e.to_string().contains("UNIQUE") {
                        DbError::Message("Ce nom de modèle existe déjà".to_string())
                    } else {
                        DbError::Sqlite(e)
                    }
                })?;
            return self.get_print_model(id);
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        self.conn
            .execute(
                "INSERT INTO document_print_models (id, name, description, html_content, css_content, screen_key, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![new_id, name, description, html_content, css_content, sk, now, now],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    DbError::Message("Ce nom de modèle existe déjà".to_string())
                } else {
                    DbError::Sqlite(e)
                }
            })?;
        self.get_print_model(&new_id)
    }

    /// Modèle fiche objet unique pour un écran / entité (priorité au nom « Fiche »).
    pub fn get_fiche_print_model_for_screen(
        &self,
        screen_key: &str,
    ) -> Result<Option<PrintModelDetail>, DbError> {
        use rusqlite::OptionalExtension;
        let id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM document_print_models WHERE screen_key = ?1
                 ORDER BY CASE WHEN name LIKE '%Fiche%' THEN 0 ELSE 1 END, updated_at DESC
                 LIMIT 1",
                params![screen_key],
                |r| r.get(0),
            )
            .optional()?;
        match id {
            Some(i) => Ok(Some(self.get_print_model(&i)?)),
            None => Ok(None),
        }
    }

    pub fn delete_print_model(&self, id: &str) -> Result<bool, DbError> {
        let n = self
            .conn
            .execute("DELETE FROM document_print_models WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PrintModelRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub screen_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PrintModelDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub html_content: String,
    pub css_content: String,
    pub screen_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
