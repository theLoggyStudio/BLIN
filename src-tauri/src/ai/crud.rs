//! Exécution CRUD Blin pour Loggy — fusion des champs (patch) par entité.

use serde_json::Value;

use crate::bien_detail::BienEtageNom;
use crate::db::{BienInput, BienRow, ContratInput, ContratRow, Database, FinanceInput, HangarRow};

pub const WRITE_TOOLS: &[&str] = &[
    "create_bien",
    "update_bien",
    "delete_bien",
    "delete_all_biens",
    "create_hangar",
    "update_hangar",
    "delete_hangar",
    "create_contrat",
    "update_contrat",
    "delete_contrat",
    "sign_contrat",
    "create_finance",
    "update_finance",
    "delete_finance",
    "validate_finance",
];

pub fn is_crud_write_tool(name: &str) -> bool {
    WRITE_TOOLS.contains(&name)
}

pub fn param_str(params: &Value, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.trim().to_string())
            } else if v.is_number() {
                Some(v.to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

pub fn param_f64(params: &Value, key: &str) -> Option<f64> {
    params.get(key).and_then(|v| v.as_f64())
}

pub fn param_i32(params: &Value, key: &str) -> Option<i32> {
    params.get(key).and_then(|v| v.as_i64()).map(|n| n as i32)
}

pub fn param_u32(params: &Value, key: &str) -> Option<u32> {
    params.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

fn parse_nomenclature(params: &Value) -> Vec<BienEtageNom> {
    params
        .get("nomenclature")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

pub fn resolve_bien_id(db: &Database, params: &Value) -> Result<String, String> {
    if let Some(id) = param_str(params, "id") {
        return Ok(id);
    }
    let reference = param_str(params, "reference")
        .ok_or("Paramètre reference ou id requis pour le bien")?;
    db.get_bien_by_reference(&reference)
        .map(|b| b.id)
        .map_err(|e| e.to_string())
}

pub fn resolve_hangar_id(db: &Database, params: &Value) -> Result<String, String> {
    if let Some(id) = param_str(params, "id") {
        let type_bien: String = db
            .conn
            .query_row(
                "SELECT type_bien FROM biens WHERE id = ?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if type_bien != "hangar" {
            return Err("L'id fourni n'est pas un hangar.".into());
        }
        return Ok(id);
    }
    let reference = param_str(params, "reference")
        .ok_or("Paramètre reference ou id requis pour le hangar")?;
    let bien = db
        .get_bien_by_reference(&reference)
        .map_err(|e| e.to_string())?;
    if bien.type_bien != "hangar" {
        return Err(format!("La référence « {reference} » n'est pas un hangar."));
    }
    Ok(bien.id)
}

pub fn resolve_contrat_id(db: &Database, params: &Value) -> Result<String, String> {
    if let Some(id) = param_str(params, "id") {
        return Ok(id);
    }
    let reference = param_str(params, "reference")
        .ok_or("Paramètre reference ou id requis pour le contrat")?;
    db.get_contrat_by_reference(&reference)
        .map(|c| c.id)
        .map_err(|e| e.to_string())
}

pub fn resolve_finance_id(db: &Database, params: &Value) -> Result<String, String> {
    if let Some(id) = param_str(params, "id") {
        return Ok(id);
    }
    let reference = param_str(params, "reference")
        .ok_or("Paramètre reference ou id requis pour la finance")?;
    db.get_finance_by_reference(&reference)
        .map(|f| f.id)
        .map_err(|e| e.to_string())
}

pub fn resolve_bien_id_optional(db: &Database, params: &Value) -> Result<Option<String>, String> {
    if let Some(id) = param_str(params, "bien_id") {
        return Ok(Some(id));
    }
    if let Some(br) = param_str(params, "bien_reference") {
        let bien = db.get_bien_by_reference(&br).map_err(|e| e.to_string())?;
        return Ok(Some(bien.id));
    }
    Ok(None)
}

struct BienPatch {
    reference: String,
    adresse: String,
    type_bien: String,
    statut: String,
    surface_m2: Option<f64>,
    prix_defaut: Option<f64>,
    nb_etages: Option<i32>,
    nb_chambres: Option<i32>,
    nb_pieces: Option<i32>,
    nomenclature: Vec<BienEtageNom>,
    domaine: String,
    devise: String,
    zone: Option<String>,
    capacite_m3: Option<f64>,
}

fn merge_bien_patch(existing: &BienRow, params: &Value) -> BienPatch {
    BienPatch {
        reference: param_str(params, "reference").unwrap_or_else(|| existing.reference.clone()),
        adresse: param_str(params, "adresse").unwrap_or_else(|| existing.adresse.clone()),
        type_bien: param_str(params, "type_bien").unwrap_or_else(|| existing.type_bien.clone()),
        statut: param_str(params, "statut").unwrap_or_else(|| existing.statut.clone()),
        surface_m2: param_f64(params, "surface_m2").or(existing.surface_m2),
        prix_defaut: param_f64(params, "prix_defaut").or(existing.prix_defaut),
        nb_etages: param_i32(params, "nb_etages").or(existing.nb_etages),
        nb_chambres: param_i32(params, "nb_chambres").or(existing.nb_chambres),
        nb_pieces: param_i32(params, "nb_pieces").or(existing.nb_pieces),
        nomenclature: if params.get("nomenclature").is_some() {
            parse_nomenclature(params)
        } else {
            existing.nomenclature.clone()
        },
        domaine: param_str(params, "domaine").unwrap_or_else(|| existing.domaine.clone()),
        devise: param_str(params, "devise").unwrap_or_else(|| existing.devise.clone()),
        zone: param_str(params, "zone").or(existing.zone.clone()),
        capacite_m3: param_f64(params, "capacite_m3").or(existing.capacite_m3),
    }
}

fn bien_patch_to_input<'a>(p: &'a BienPatch) -> BienInput<'a> {
    BienInput {
        reference: &p.reference,
        adresse: &p.adresse,
        type_bien: &p.type_bien,
        statut: Some(&p.statut),
        surface_m2: p.surface_m2,
        prix_defaut: p.prix_defaut,
        nb_etages: p.nb_etages,
        nb_chambres: p.nb_chambres,
        nb_pieces: p.nb_pieces,
        nomenclature: &p.nomenclature,
        domaine: &p.domaine,
        devise: &p.devise,
        zone: p.zone.as_deref(),
        capacite_m3: p.capacite_m3,
    }
}

struct ContratPatch {
    reference: String,
    bien_id: Option<String>,
    locataire: String,
    locataire_email: String,
    locataire_telephone: String,
    loyer_mensuel: f64,
    date_debut: String,
    date_fin: Option<String>,
    logement_cle: Option<String>,
    devise: String,
}

fn merge_contrat_patch(existing: &ContratRow, params: &Value) -> ContratPatch {
    ContratPatch {
        reference: param_str(params, "reference").unwrap_or_else(|| existing.reference.clone()),
        bien_id: existing.bien_id.clone(),
        locataire: param_str(params, "locataire").unwrap_or_else(|| existing.locataire.clone()),
        locataire_email: param_str(params, "locataire_email")
            .unwrap_or_else(|| existing.locataire_email.clone()),
        locataire_telephone: param_str(params, "locataire_telephone")
            .unwrap_or_else(|| existing.locataire_telephone.clone()),
        loyer_mensuel: param_f64(params, "loyer_mensuel").unwrap_or(existing.loyer_mensuel),
        date_debut: param_str(params, "date_debut").unwrap_or_else(|| existing.date_debut.clone()),
        date_fin: param_str(params, "date_fin").or(existing.date_fin.clone()),
        logement_cle: param_str(params, "logement_cle").or(existing.logement_cle.clone()),
        devise: param_str(params, "devise").unwrap_or_else(|| existing.devise.clone()),
    }
}

fn contrat_patch_to_input<'a>(p: &'a ContratPatch) -> ContratInput<'a> {
    ContratInput {
        reference: &p.reference,
        bien_id: p.bien_id.as_deref(),
        locataire: &p.locataire,
        locataire_email: &p.locataire_email,
        locataire_telephone: &p.locataire_telephone,
        loyer_mensuel: p.loyer_mensuel,
        date_debut: &p.date_debut,
        date_fin: p.date_fin.as_deref(),
        logement_cle: p.logement_cle.as_deref(),
        devise: &p.devise,
    }
}

struct FinancePatch {
    reference: String,
    libelle: String,
    montant: f64,
    type_operation: String,
    statut: String,
    date_echeance: String,
    bien_reference: Option<String>,
    devise: String,
    contrat_id: Option<String>,
    periode_annee: Option<i32>,
    periode_mois: Option<i32>,
}

fn merge_finance_patch(
    existing: &crate::db::FinanceRow,
    params: &Value,
) -> FinancePatch {
    FinancePatch {
        reference: param_str(params, "reference").unwrap_or_else(|| existing.reference.clone()),
        libelle: param_str(params, "libelle").unwrap_or_else(|| existing.libelle.clone()),
        montant: param_f64(params, "montant").unwrap_or(existing.montant),
        type_operation: param_str(params, "type_operation")
            .unwrap_or_else(|| existing.type_operation.clone()),
        statut: param_str(params, "statut").unwrap_or_else(|| existing.statut.clone()),
        date_echeance: param_str(params, "date_echeance")
            .unwrap_or_else(|| existing.date_echeance.clone()),
        bien_reference: param_str(params, "bien_reference").or(existing.bien_reference.clone()),
        devise: param_str(params, "devise").unwrap_or_else(|| existing.devise.clone()),
        contrat_id: param_str(params, "contrat_id").or(existing.contrat_id.clone()),
        periode_annee: param_i32(params, "periode_annee").or(existing.periode_annee),
        periode_mois: param_i32(params, "periode_mois").or(existing.periode_mois),
    }
}

/// Exécute une action d'écriture confirmée (tous écrans métier).
pub fn execute_crud_write(db: &Database, tool: &str, params: &Value) -> Result<String, String> {
    match tool {
        "create_bien" => {
            if crate::ai::dda_bridge::biens_uses_dda() {
                return crate::ai::dda_bridge::execute_bien_write_via_dda(db, tool, params);
            }
            let reference = param_str(params, "reference").ok_or("reference requis")?;
            let adresse = param_str(params, "adresse").ok_or("adresse requise")?;
            let type_bien = param_str(params, "type_bien").unwrap_or_else(|| "appartement".into());
            let statut = param_str(params, "statut");
            let zone = param_str(params, "zone");
            let nomenclature = parse_nomenclature(params);
            let domaine = param_str(params, "domaine").unwrap_or_else(|| "habitation".into());
            let devise = param_str(params, "devise").unwrap_or_else(|| "EUR".into());
            let input = BienInput {
                reference: &reference,
                adresse: &adresse,
                type_bien: &type_bien,
                statut: statut.as_deref(),
                surface_m2: param_f64(params, "surface_m2"),
                prix_defaut: param_f64(params, "prix_defaut"),
                nb_etages: param_i32(params, "nb_etages"),
                nb_chambres: param_i32(params, "nb_chambres"),
                nb_pieces: param_i32(params, "nb_pieces"),
                nomenclature: &nomenclature,
                domaine: &domaine,
                devise: &devise,
                zone: zone.as_deref(),
                capacite_m3: param_f64(params, "capacite_m3"),
            };
            let row = db.create_bien(input).map_err(|e| e.to_string())?;
            Ok(format!("Bien {} créé.", row.reference))
        }
        "update_bien" => {
            if crate::ai::dda_bridge::biens_uses_dda() {
                return crate::ai::dda_bridge::execute_bien_write_via_dda(db, tool, params);
            }
            let id = resolve_bien_id(db, params)?;
            let existing = db.get_bien(&id).map_err(|e| e.to_string())?;
            let patch = merge_bien_patch(&existing, params);
            let input = bien_patch_to_input(&patch);
            let row = db.update_bien(&id, input).map_err(|e| e.to_string())?;
            Ok(format!("Bien {} mis à jour.", row.reference))
        }
        "delete_bien" => {
            if crate::ai::dda_bridge::biens_uses_dda() {
                return crate::ai::dda_bridge::execute_bien_write_via_dda(db, tool, params);
            }
            let id = resolve_bien_id(db, params)?;
            let row = db.get_bien(&id).map_err(|e| e.to_string())?;
            if row.type_bien == "hangar" {
                return Err("C'est un hangar : utilisez delete_hangar.".into());
            }
            let reference = row.reference;
            db.delete_bien(&id).map_err(|e| e.to_string())?;
            Ok(format!("Bien {reference} supprimé."))
        }
        "delete_all_biens" => {
            let biens: Vec<_> = db
                .list_biens()
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|b| b.type_bien != "hangar")
                .collect();
            if biens.is_empty() {
                return Ok("Aucun bien à supprimer (hors hangars).".into());
            }
            let mut refs = Vec::with_capacity(biens.len());
            for row in biens {
                refs.push(row.reference.clone());
                db.delete_bien(&row.id).map_err(|e| e.to_string())?;
            }
            let n = refs.len();
            Ok(format!(
                "{n} bien(s) supprimé(s) : {}.",
                refs.join(", ")
            ))
        }
        "create_hangar" => {
            let reference = param_str(params, "reference").ok_or("reference requis")?;
            let zone = param_str(params, "zone").ok_or("zone requise")?;
            let capacite = param_f64(params, "capacite_m3").unwrap_or(100.0);
            let row = db
                .create_hangar(&reference, &zone, capacite)
                .map_err(|e| e.to_string())?;
            Ok(format!("Hangar {} créé (zone {}).", row.reference, row.zone))
        }
        "update_hangar" => {
            let id = resolve_hangar_id(db, params)?;
            let existing = db.get_bien(&id).map_err(|e| e.to_string())?;
            let h = HangarRow {
                id: existing.id.clone(),
                reference: existing.reference.clone(),
                zone: existing.zone.clone().unwrap_or_default(),
                capacite_m3: existing.capacite_m3.unwrap_or(0.0),
                statut: existing.statut.clone(),
            };
            let reference = param_str(params, "reference").unwrap_or(h.reference);
            let zone = param_str(params, "zone").unwrap_or(h.zone);
            let capacite = param_f64(params, "capacite_m3").unwrap_or(h.capacite_m3);
            let statut = param_str(params, "statut").unwrap_or(h.statut);
            let row = db
                .update_hangar(&id, &reference, &zone, capacite, &statut)
                .map_err(|e| e.to_string())?;
            Ok(format!("Hangar {} mis à jour.", row.reference))
        }
        "delete_hangar" => {
            let id = resolve_hangar_id(db, params)?;
            let reference = db.get_bien(&id).map_err(|e| e.to_string())?.reference;
            db.delete_bien(&id).map_err(|e| e.to_string())?;
            Ok(format!("Hangar {reference} supprimé."))
        }
        "create_contrat" => {
            let reference = param_str(params, "reference").ok_or("reference requis")?;
            let locataire = param_str(params, "locataire").ok_or("locataire requis")?;
            let email = param_str(params, "locataire_email").unwrap_or_else(|| "-".into());
            let tel = param_str(params, "locataire_telephone").unwrap_or_else(|| "0000000000".into());
            let loyer = param_f64(params, "loyer_mensuel").ok_or("loyer_mensuel requis")?;
            let date_debut = param_str(params, "date_debut").ok_or("date_debut requise")?;
            let bien_id = resolve_bien_id_optional(db, params)?;
            let date_fin = param_str(params, "date_fin");
            let logement_cle = param_str(params, "logement_cle");
            let devise = param_str(params, "devise").unwrap_or_else(|| "EUR".into());
            let input = ContratInput {
                reference: &reference,
                bien_id: bien_id.as_deref(),
                locataire: &locataire,
                locataire_email: &email,
                locataire_telephone: &tel,
                loyer_mensuel: loyer,
                date_debut: &date_debut,
                date_fin: date_fin.as_deref(),
                logement_cle: logement_cle.as_deref(),
                devise: &devise,
            };
            let row = db.create_contrat(input).map_err(|e| e.to_string())?;
            Ok(format!("Contrat {} créé (brouillon).", row.reference))
        }
        "update_contrat" => {
            let id = resolve_contrat_id(db, params)?;
            let existing = db.get_contrat(&id).map_err(|e| e.to_string())?;
            let mut patch = merge_contrat_patch(&existing, params);
            if let Ok(Some(bid)) = resolve_bien_id_optional(db, params) {
                patch.bien_id = Some(bid);
            }
            let input = contrat_patch_to_input(&patch);
            let row = db.update_contrat(&id, input).map_err(|e| e.to_string())?;
            Ok(format!("Contrat {} mis à jour.", row.reference))
        }
        "delete_contrat" => {
            let id = resolve_contrat_id(db, params)?;
            let reference = db.get_contrat(&id).map_err(|e| e.to_string())?.reference;
            db.delete_contrat(&id).map_err(|e| e.to_string())?;
            Ok(format!("Contrat {reference} supprimé."))
        }
        "sign_contrat" => {
            let id = resolve_contrat_id(db, params)?;
            let reference = db.get_contrat(&id).map_err(|e| e.to_string())?.reference;
            db.sign_contrat(&id).map_err(|e| e.to_string())?;
            Ok(format!("Contrat {reference} signé (actif)."))
        }
        "create_finance" => {
            let reference = param_str(params, "reference").ok_or("reference requis")?;
            let libelle = param_str(params, "libelle").ok_or("libelle requis")?;
            let montant = param_f64(params, "montant").ok_or("montant requis")?;
            let date_echeance = param_str(params, "date_echeance").ok_or("date_echeance requise")?;
            let statut = param_str(params, "statut").unwrap_or_else(|| "en_attente".into());
            let type_op = param_str(params, "type_operation").unwrap_or_else(|| "loyer".into());
            let devise = param_str(params, "devise").unwrap_or_else(|| "EUR".into());
            let contrat_id = param_str(params, "contrat_id").or_else(|| {
                param_str(params, "contrat_reference").and_then(|r| {
                    db.get_contrat_by_reference(&r).ok().map(|c| c.id)
                })
            });
            let bien_reference = param_str(params, "bien_reference");
            let input = FinanceInput {
                reference: &reference,
                libelle: &libelle,
                montant,
                type_operation: &type_op,
                statut: &statut,
                date_echeance: &date_echeance,
                bien_reference: bien_reference.as_deref(),
                devise: &devise,
                contrat_id: contrat_id.as_deref(),
                periode_annee: param_i32(params, "periode_annee"),
                periode_mois: param_i32(params, "periode_mois"),
            };
            let row = db.create_finance(input).map_err(|e| e.to_string())?;
            Ok(format!("Écriture {} créée.", row.reference))
        }
        "update_finance" => {
            let id = resolve_finance_id(db, params)?;
            let existing = db.get_finance(&id).map_err(|e| e.to_string())?;
            let patch = merge_finance_patch(&existing, params);
            let input = FinanceInput {
                reference: &patch.reference,
                libelle: &patch.libelle,
                montant: patch.montant,
                type_operation: &patch.type_operation,
                statut: &patch.statut,
                date_echeance: &patch.date_echeance,
                bien_reference: patch.bien_reference.as_deref(),
                devise: &patch.devise,
                contrat_id: patch.contrat_id.as_deref(),
                periode_annee: patch.periode_annee,
                periode_mois: patch.periode_mois,
            };
            let row = db.update_finance(&id, input).map_err(|e| e.to_string())?;
            Ok(format!("Écriture {} mise à jour.", row.reference))
        }
        "delete_finance" => {
            let id = resolve_finance_id(db, params)?;
            let reference = db.get_finance(&id).map_err(|e| e.to_string())?.reference;
            db.delete_finance(&id).map_err(|e| e.to_string())?;
            Ok(format!("Écriture {reference} supprimée."))
        }
        "validate_finance" => {
            let id = resolve_finance_id(db, params)?;
            let reference = db.get_finance(&id).map_err(|e| e.to_string())?.reference;
            db.validate_finance(&id).map_err(|e| e.to_string())?;
            Ok(format!("Écriture {reference} marquée comme payée."))
        }
        other => Err(format!("Action CRUD inconnue : {other}")),
    }
}
