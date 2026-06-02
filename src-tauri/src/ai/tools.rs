use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use chrono::Datelike;

use crate::ai::crud::{execute_crud_write, is_crud_write_tool};
use crate::ai::experience::record_success;
use crate::ai::intent_filters::{statut_label, wants_last_bien};
use crate::ai::reports::{build_bien_report_html, build_finances_month_html, write_export_file};
use crate::db::{BienRow, Database};
use crate::privileges::has_privilege;

fn row_matches_statut(row_statut: &str, filter: Option<&str>) -> bool {
    filter.map(|f| row_statut == f).unwrap_or(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub params: Value,
    pub explain: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
    pub requires_confirmation: bool,
    pub pending_id: Option<String>,
    /// Si présent, seul un utilisateur avec ce privilège peut confirmer (ex. Directeur).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_privilege: Option<String>,
}

pub const DIRECTEUR_CONFIRM_PRIVILEGE: &str = "directeur:confirmer";

const RETIRED_IMMOBILIER_MSG: &str =
    "Cette fonctionnalité immobilière (biens, finances) n'existe plus. Utilisez le tableau de bord et les entités déclarées dans Paramètres > Entités.";

const RETIRED_IMMOBILIER_TOOLS: &[&str] = &[
    "dashboard_stats",
    "list_biens",
    "get_bien",
    "search_biens",
    "export_bien_report",
    "list_hangars",
    "list_contrats",
    "get_contrat",
    "list_finances",
    "finances_generate_month",
    "export_finances_month",
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

pub fn is_retired_immobilier_tool(name: &str) -> bool {
    RETIRED_IMMOBILIER_TOOLS.contains(&name)
}

fn retired_tool_result(tool: &str) -> ToolResult {
    ToolResult {
        tool: tool.to_string(),
        success: false,
        message: RETIRED_IMMOBILIER_MSG.to_string(),
        data: None,
        requires_confirmation: false,
        pending_id: None,
        confirm_privilege: None,
    }
}

fn normalize_tool_name(tool: &str) -> String {
    match tool {
        "supprime_bien" | "supprimer_bien" => "delete_bien".into(),
        "supprime_biens" | "supprimer_biens" | "delete_all" => "delete_all_biens".into(),
        other => other.to_string(),
    }
}

pub fn tool_confirm_privilege(name: &str) -> Option<&'static str> {
    match name {
        "delete_all_biens" => Some(DIRECTEUR_CONFIRM_PRIVILEGE),
        _ => None,
    }
}

fn count_biens_for_mass_delete(db: &Database) -> Result<usize, String> {
    Ok(db
        .list_biens()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|b| b.type_bien != "hangar")
        .count())
}

pub fn format_tool_reply(tool: &str, tr: &ToolResult) -> String {
    if let Some(data) = &tr.data {
        if let Ok(pretty) = serde_json::to_string_pretty(data) {
            return format!("{}\n\n```json\n{pretty}\n```", tr.message);
        }
    }
    format!("{} — {}", tool, tr.message)
}

pub fn extract_json_value(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(&text[start..=end]).ok()
}

pub fn parse_tool_call(text: &str) -> Option<ToolCall> {
    let v = extract_json_value(text)?;
    if let Some(tool) = v.get("tool").and_then(|t| t.as_str()) {
        let mut params = v
            .get("params")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));
        let mut tool_name = normalize_tool_name(tool);
        if params.get("biens").and_then(|b| b.as_array()).is_some() {
            tool_name = "delete_all_biens".into();
            params = Value::Object(Default::default());
        }
        let explain = v
            .get("explain")
            .and_then(|e| e.as_str())
            .map(|s| s.to_string());
        return Some(ToolCall {
            tool: tool_name,
            params,
            explain,
        });
    }
    infer_crud_call_from_params(v, "")
}

/// JSON CRUD explicite ou métier sans clé `tool`.
pub fn parse_crud_from_user_message(message: &str) -> Option<ToolCall> {
    if let Some(v) = extract_json_value(message) {
        if let Some(tool) = v.get("tool").and_then(|t| t.as_str()) {
            if is_crud_write_tool(tool) {
                let params = v
                    .get("params")
                    .cloned()
                    .unwrap_or_else(|| {
                        let mut o = v.as_object().cloned().unwrap_or_default();
                        o.remove("tool");
                        o.remove("explain");
                        Value::Object(o)
                    });
                return Some(ToolCall {
                    tool: tool.to_string(),
                    params,
                    explain: v
                        .get("explain")
                        .and_then(|e| e.as_str())
                        .map(str::to_string),
                });
            }
        }
        if let Some(call) = infer_crud_call_from_params(v, message) {
            return Some(call);
        }
    }
    None
}

/// JSON utilisateur `{ "reference", "adresse", "type_bien", ... }` sans clé `tool`.
pub fn parse_create_from_user_message(message: &str) -> Option<ToolCall> {
    if !crate::ai::intent_filters::wants_create_intent(message) {
        return None;
    }
    let v = extract_json_value(message)?;
    infer_crud_call_from_params(v, message).filter(|c| c.tool.starts_with("create_"))
}

/// Mise à jour par JSON ou langage naturel + référence.
pub fn parse_update_from_user_message(message: &str) -> Option<ToolCall> {
    if !crate::ai::intent_filters::wants_update_intent(message) {
        return None;
    }
    if let Some(v) = extract_json_value(message) {
        if let Some(call) = infer_update_call_from_params(&v, message) {
            return Some(call);
        }
    }
    infer_update_call_from_text(message)
}

/// Suppression massive — confirmation réservée au Directeur.
pub fn parse_delete_all_from_user_message(message: &str) -> Option<ToolCall> {
    if !crate::ai::intent_filters::wants_delete_all_biens_intent(message) {
        return None;
    }
    Some(ToolCall {
        tool: "delete_all_biens".into(),
        params: Value::Object(Default::default()),
        explain: Some("Suppression de tous les biens (hors hangars)".into()),
    })
}

/// JSON `{ "reference": "BIEN-001" }` ou message « supprime le bien … ».
pub fn parse_delete_from_user_message(message: &str) -> Option<ToolCall> {
    if crate::ai::intent_filters::wants_delete_all_biens_intent(message) {
        return None;
    }
    if !crate::ai::intent_filters::wants_delete_intent(message) {
        return None;
    }
    if let Some(v) = extract_json_value(message) {
        if let Some(call) = infer_delete_call_from_params(&v, message) {
            return Some(call);
        }
    }
    infer_delete_call_from_text(message)
}

fn infer_delete_call_from_params(v: &Value, message: &str) -> Option<ToolCall> {
    let reference = v
        .get("reference")
        .and_then(|r| r.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let tool = if crate::ai::intent_filters::message_targets_finance(message) {
        "delete_finance"
    } else if crate::ai::intent_filters::message_targets_contrat(message) {
        "delete_contrat"
    } else if v.get("type_bien").and_then(|t| t.as_str()) == Some("hangar")
        || crate::ai::intent_filters::message_targets_hangar(message)
    {
        "delete_hangar"
    } else {
        "delete_bien"
    };
    Some(ToolCall {
        tool: tool.to_string(),
        params: serde_json::json!({ "reference": reference }),
        explain: Some("Suppression demandée par l'utilisateur".into()),
    })
}

fn infer_delete_call_from_text(message: &str) -> Option<ToolCall> {
    let n = crate::ai::intent_filters::normalize_message(message);
    let reference = extract_reference_token(&n)?;
    let tool = if crate::ai::intent_filters::message_targets_finance(message) {
        "delete_finance"
    } else if crate::ai::intent_filters::message_targets_contrat(message) {
        "delete_contrat"
    } else if crate::ai::intent_filters::message_targets_hangar(message) {
        "delete_hangar"
    } else {
        "delete_bien"
    };
    Some(ToolCall {
        tool: tool.to_string(),
        params: serde_json::json!({ "reference": reference }),
        explain: None,
    })
}

pub fn extract_reference_from_message(message: &str) -> Option<String> {
    let n = crate::ai::intent_filters::normalize_message(message);
    extract_reference_token(&n)
}

fn extract_reference_token(normalized: &str) -> Option<String> {
    for word in normalized.split_whitespace() {
        let w = word.trim_matches(|c: char| ",.;:!?()[]{}'\"".contains(c));
        if w.len() >= 4
            && (w.contains('-')
                || w.starts_with("fin")
                || w.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
            && (w.chars().any(|c| c.is_ascii_digit()) || w.starts_with("fin"))
        {
            let upper = w.to_uppercase();
            return Some(if upper.starts_with("FIN") && !upper.contains('-') {
                format!("FIN-{}", upper.trim_start_matches("FIN"))
            } else {
                upper
            });
        }
    }
    None
}

fn infer_crud_call_from_params(v: Value, message: &str) -> Option<ToolCall> {
    if let Some(call) = infer_update_call_from_params(&v, message) {
        return Some(call);
    }
    infer_create_call_from_params(v, message)
}

fn infer_create_call_from_params(v: Value, message: &str) -> Option<ToolCall> {
    let obj = v.as_object()?;
    let _reference = obj.get("reference").and_then(|r| r.as_str()).filter(|s| !s.is_empty())?;
    if obj.get("libelle").is_some() && obj.get("montant").is_some() {
        return Some(ToolCall {
            tool: "create_finance".into(),
            params: v,
            explain: Some("Création écriture financière".into()),
        });
    }
    if obj.get("locataire").is_some() && obj.get("loyer_mensuel").is_some() {
        return Some(ToolCall {
            tool: "create_contrat".into(),
            params: v,
            explain: Some("Création contrat".into()),
        });
    }
    if obj.get("zone").is_some()
        || obj.get("type_bien").and_then(|t| t.as_str()) == Some("hangar")
        || crate::ai::intent_filters::message_targets_hangar(message)
    {
        return Some(ToolCall {
            tool: "create_hangar".into(),
            params: v,
            explain: Some("Création hangar".into()),
        });
    }
    if obj.get("adresse").is_some() {
        let type_bien = obj
            .get("type_bien")
            .and_then(|t| t.as_str())
            .unwrap_or("appartement");
        let tool = if type_bien == "hangar" {
            "create_hangar"
        } else {
            "create_bien"
        };
        return Some(ToolCall {
            tool: tool.to_string(),
            params: v,
            explain: Some("Création bien".into()),
        });
    }
    None
}

fn infer_update_call_from_params(v: &Value, message: &str) -> Option<ToolCall> {
    let reference = v
        .get("reference")
        .and_then(|r| r.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let tool = if v.get("tool").and_then(|t| t.as_str()).is_some_and(|t| t.starts_with("update_")) {
        v.get("tool").and_then(|t| t.as_str())?
    } else if crate::ai::intent_filters::message_targets_finance(message) {
        "update_finance"
    } else if crate::ai::intent_filters::message_targets_contrat(message) {
        "update_contrat"
    } else if crate::ai::intent_filters::message_targets_hangar(message) {
        "update_hangar"
    } else {
        "update_bien"
    };
    if !is_crud_write_tool(tool) || !tool.starts_with("update_") {
        return None;
    }
    Some(ToolCall {
        tool: tool.to_string(),
        params: v.clone(),
        explain: Some("Mise à jour demandée".into()),
    })
}

fn infer_update_call_from_text(message: &str) -> Option<ToolCall> {
    let n = crate::ai::intent_filters::normalize_message(message);
    let reference = extract_reference_token(&n)?;
    let tool = if crate::ai::intent_filters::message_targets_finance(message) {
        "update_finance"
    } else if crate::ai::intent_filters::message_targets_contrat(message) {
        "update_contrat"
    } else if crate::ai::intent_filters::message_targets_hangar(message) {
        "update_hangar"
    } else {
        "update_bien"
    };
    Some(ToolCall {
        tool: tool.to_string(),
        params: serde_json::json!({ "reference": reference }),
        explain: None,
    })
}

fn param_str(params: &Value, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn param_f64(params: &Value, key: &str) -> Option<f64> {
    params.get(key).and_then(|v| v.as_f64())
}

fn list_message(entity: &str, count: usize, statut: Option<&str>) -> String {
    match statut {
        Some(s) => format!(
            "{} {}(s) {}.",
            count,
            entity,
            statut_label(s)
        ),
        None => format!("{} {}(s).", count, entity),
    }
}

pub fn tool_privilege(name: &str) -> &'static str {
    match name {
        "dashboard_stats" | "list_biens" | "get_bien" | "search_biens" | "export_bien_report" => {
            "biens:voir"
        }
        "list_hangars" => "hangars:voir",
        "list_contrats" | "get_contrat" => "contrats:voir",
        "list_finances" | "finances_generate_month" | "export_finances_month" => {
            "finances:valider"
        }
        "create_bien" => "biens:creer",
        "update_bien" | "delete_bien" | "delete_all_biens" => "biens:modifier",
        "create_hangar" | "update_hangar" | "delete_hangar" => "hangars:modifier",
        "create_contrat" | "update_contrat" | "delete_contrat" | "sign_contrat" => {
            "contrats:signer"
        }
        "create_finance"
        | "update_finance"
        | "delete_finance"
        | "validate_finance" => "finances:valider",
        "web_search" => "ai:utiliser",
        _ if name.starts_with("dda_") => "ai:utiliser",
        _ => "ai:utiliser",
    }
}

pub fn is_write_tool(name: &str) -> bool {
    is_crud_write_tool(name)
}

fn resolve_bien_for_params(
    db: &Database,
    params: &Value,
    message_hint: Option<&str>,
) -> Result<BienRow, String> {
    if let Some(reference) = param_str(params, "reference") {
        return db
            .get_bien_by_reference(&reference)
            .map_err(|e| e.to_string());
    }
    let use_last = params
        .get("use_last")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || message_hint.is_some_and(wants_last_bien);
    if use_last {
        return db.get_last_bien().map_err(|e| {
            if e.to_string().contains("QueryReturnedNoRows") {
                "Aucun bien en base.".into()
            } else {
                e.to_string()
            }
        });
    }
    db.get_last_bien().map_err(|e| e.to_string())
}

fn bien_to_json(b: &BienRow) -> Value {
    serde_json::json!({
        "reference": b.reference,
        "adresse": b.adresse,
        "type_bien": b.type_bien,
        "statut": b.statut,
        "domaine": b.domaine,
        "devise": b.devise,
        "surface_m2": b.surface_m2,
        "prix_defaut": b.prix_defaut,
        "nb_etages": b.nb_etages,
        "nb_chambres": b.nb_chambres,
        "nb_pieces": b.nb_pieces,
        "zone": b.zone,
        "capacite_m3": b.capacite_m3,
    })
}

fn resolve_bien_id_by_reference(db: &Database, reference: &str) -> Result<String, String> {
    db.conn
        .query_row(
            "SELECT id FROM biens WHERE reference = ?1",
            rusqlite::params![reference],
            |r| r.get(0),
        )
        .map_err(|_| format!("Référence « {reference} » introuvable."))
}

fn resolve_hangar_id_by_reference(db: &Database, reference: &str) -> Result<String, String> {
    let id = resolve_bien_id_by_reference(db, reference)?;
    let type_bien: String = db
        .conn
        .query_row(
            "SELECT type_bien FROM biens WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if type_bien != "hangar" {
        return Err(format!(
            "La référence « {reference} » n'est pas un hangar (type : {type_bien})."
        ));
    }
    Ok(id)
}

pub fn execute_read_tool(
    db: &Database,
    privileges: &[String],
    call: &ToolCall,
) -> Result<ToolResult, String> {
    if is_retired_immobilier_tool(&call.tool) {
        return Ok(retired_tool_result(&call.tool));
    }
    let priv_req = tool_privilege(&call.tool);
    if !has_privilege(privileges, priv_req) {
        return Ok(ToolResult {
            tool: call.tool.clone(),
            success: false,
            message: format!("Privilège requis : {priv_req}"),
            data: None,
            requires_confirmation: false,
            pending_id: None,
            confirm_privilege: None,
        });
    }

    let result = match call.tool.as_str() {
        "dashboard_stats" => {
            let s = db.dashboard_stats().map_err(|e| e.to_string())?;
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: format!(
                    "Biens: {} total, {} dispo | Hangars libres: {} | Contrats actifs: {} | Finances en attente: {}",
                    s.biens_total,
                    s.biens_disponibles,
                    s.hangars_libres,
                    s.contrats_actifs,
                    s.finances_en_attente
                ),
                data: Some(serde_json::to_value(&s).unwrap_or(Value::Null)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "list_biens" => {
            let statut_filter = param_str(&call.params, "statut");
            let rows = db.list_biens().map_err(|e| e.to_string())?;
            let filtered: Vec<_> = rows
                .iter()
                .filter(|b| row_matches_statut(&b.statut, statut_filter.as_deref()))
                .collect();
            let summary: Vec<_> = filtered
                .iter()
                .take(25)
                .map(|b| {
                    serde_json::json!({
                        "reference": b.reference,
                        "type": b.type_bien,
                        "statut": b.statut,
                        "adresse": b.adresse,
                    })
                })
                .collect();
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: list_message("bien", filtered.len(), statut_filter.as_deref()),
                data: Some(Value::Array(summary)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "list_hangars" => {
            let statut_filter = param_str(&call.params, "statut");
            let rows = db.list_hangars().map_err(|e| e.to_string())?;
            let filtered: Vec<_> = rows
                .iter()
                .filter(|h| row_matches_statut(&h.statut, statut_filter.as_deref()))
                .collect();
            let summary: Vec<_> = filtered
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "reference": h.reference,
                        "zone": h.zone,
                        "capacite_m3": h.capacite_m3,
                        "statut": h.statut,
                    })
                })
                .collect();
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: list_message("hangar", filtered.len(), statut_filter.as_deref()),
                data: Some(Value::Array(summary)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "list_contrats" => {
            let statut_filter = param_str(&call.params, "statut");
            let rows = db.list_contrats().map_err(|e| e.to_string())?;
            let filtered: Vec<_> = rows
                .iter()
                .filter(|c| row_matches_statut(&c.statut, statut_filter.as_deref()))
                .collect();
            let summary: Vec<_> = filtered
                .iter()
                .take(20)
                .map(|c| {
                    serde_json::json!({
                        "reference": c.reference,
                        "locataire": c.locataire,
                        "loyer": c.loyer_mensuel,
                        "devise": c.devise,
                        "statut": c.statut,
                    })
                })
                .collect();
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: list_message("contrat", filtered.len(), statut_filter.as_deref()),
                data: Some(Value::Array(summary)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "list_finances" => {
            let statut_filter = param_str(&call.params, "statut");
            let rows = db.list_finances().map_err(|e| e.to_string())?;
            let filtered: Vec<_> = rows
                .iter()
                .filter(|f| row_matches_statut(&f.statut, statut_filter.as_deref()))
                .collect();
            let summary: Vec<_> = filtered
                .iter()
                .take(25)
                .map(|f| {
                    serde_json::json!({
                        "reference": f.reference,
                        "libelle": f.libelle,
                        "montant": f.montant,
                        "devise": f.devise,
                        "statut": f.statut,
                        "date_echeance": f.date_echeance,
                        "locataire": f.locataire,
                        "contrat_reference": f.contrat_reference,
                    })
                })
                .collect();
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: list_message("écriture", filtered.len(), statut_filter.as_deref()),
                data: Some(Value::Array(summary)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "get_bien" => {
            let bien = resolve_bien_for_params(db, &call.params, None)?;
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: format!("Fiche du bien {}.", bien.reference),
                data: Some(bien_to_json(&bien)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "search_biens" => {
            let q = param_str(&call.params, "query")
                .or_else(|| param_str(&call.params, "q"))
                .ok_or("Paramètre query requis")?;
            let rows = db.search_biens(&q, 15).map_err(|e| e.to_string())?;
            let summary: Vec<_> = rows.iter().map(|b| bien_to_json(b)).collect();
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: format!("{} bien(s) correspondant à « {q} ».", rows.len()),
                data: Some(Value::Array(summary)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "web_search" => {
            let q = param_str(&call.params, "query")
                .or_else(|| param_str(&call.params, "q"))
                .ok_or("Paramètre query requis")?;
            let result = crate::ai::web_search::search(&db.data_dir, &q)?;
            let answer = crate::ai::web_search::synthesize_answer(db, &q, &result)?;
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: answer,
                data: Some(serde_json::to_value(&result).unwrap_or(Value::Null)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "get_contrat" => {
            let reference = param_str(&call.params, "reference")
                .ok_or("Paramètre reference requis")?;
            let c = db.get_contrat_by_reference(&reference).map_err(|e| e.to_string())?;
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: format!("Contrat {} — {}.", c.reference, c.locataire),
                data: Some(serde_json::json!({
                    "reference": c.reference,
                    "locataire": c.locataire,
                    "loyer_mensuel": c.loyer_mensuel,
                    "devise": c.devise,
                    "statut": c.statut,
                    "date_debut": c.date_debut,
                    "date_fin": c.date_fin,
                    "bien_reference": c.bien_reference,
                })),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "export_bien_report" => {
            let use_last = call
                .params
                .get("use_last")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let bien = resolve_bien_for_params(
                db,
                &call.params,
                if use_last { Some("dernier bien") } else { None },
            )?;
            let contrats = db
                .list_contrats_for_bien_id(&bien.id)
                .map_err(|e| e.to_string())?;
            let app = crate::entity::branding::ecosystem_name(&db.data_dir);
            let html = build_bien_report_html(&bien, &contrats, &app);
            let path = write_export_file(&db.data_dir, &format!("bien_{}", bien.reference), &html)?;
            let path_str = path.to_string_lossy().to_string();
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: format!(
                    "Fiche HTML générée pour {}. Ouvrez le fichier puis Ctrl+P pour enregistrer en PDF :\n{path_str}",
                    bien.reference
                ),
                data: Some(serde_json::json!({
                    "export_path": path_str,
                    "reference": bien.reference,
                    "format": "html",
                })),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "export_finances_month" => {
            let annee = call
                .params
                .get("annee")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .or_else(|| {
                    call.params
                        .get("year")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32)
                });
            let mois = call
                .params
                .get("mois")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .or_else(|| {
                    call.params
                        .get("month")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                });
            let (annee, mois) = match (annee, mois) {
                (Some(a), Some(m)) => (a, m),
                _ => {
                    let now = chrono::Local::now();
                    (now.year(), now.month())
                }
            };
            let rows = db.list_finances().map_err(|e| e.to_string())?;
            let filtered: Vec<_> = rows
                .iter()
                .filter(|f| {
                    f.periode_annee == Some(annee) && f.periode_mois == Some(mois as i32)
                        || f.date_echeance.starts_with(&format!("{annee}-{mois:02}"))
                })
                .cloned()
                .collect();
            let app = crate::entity::branding::ecosystem_name(&db.data_dir);
            let html = build_finances_month_html(annee, mois, &filtered, &app);
            let path =
                write_export_file(&db.data_dir, &format!("loyers_{annee}_{mois:02}"), &html)?;
            let path_str = path.to_string_lossy().to_string();
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: format!(
                    "Rapport loyers {mois:02}/{annee} ({n} écriture(s)). Ctrl+P pour PDF :\n{path_str}",
                    n = filtered.len(),
                ),
                data: Some(serde_json::json!({
                    "export_path": path_str,
                    "annee": annee,
                    "mois": mois,
                })),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        "finances_generate_month" => {
            let annee = call
                .params
                .get("annee")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            let mois = call
                .params
                .get("mois")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let (annee, mois) = match (annee, mois) {
                (Some(a), Some(m)) => (a, m),
                _ => {
                    let now = chrono::Local::now();
                    (now.year(), now.month())
                }
            };
            let res = db
                .generate_monthly_finances(annee, mois)
                .map_err(|e| e.to_string())?;
            ToolResult {
                tool: call.tool.clone(),
                success: true,
                message: format!(
                    "Mois {mois:02}/{annee} : {} créée(s), {} déjà existante(s), {} contrat(s) actif(s).",
                    res.created, res.skipped, res.contrats_eligibles
                ),
                data: Some(serde_json::to_value(&res).unwrap_or(Value::Null)),
                requires_confirmation: false,
                pending_id: None,
                confirm_privilege: None,
            }
        }
        _ => ToolResult {
            tool: call.tool.clone(),
            success: false,
            message: format!("Outil inconnu : {}", call.tool),
            data: None,
            requires_confirmation: false,
            pending_id: None,
            confirm_privilege: None,
        },
    };
    Ok(result)
}

pub fn queue_write_action(
    db: &Database,
    conversation_id: &str,
    call: &ToolCall,
    privileges: &[String],
) -> Result<ToolResult, String> {
    if is_retired_immobilier_tool(&call.tool) {
        return Ok(retired_tool_result(&call.tool));
    }
    let priv_req = tool_privilege(&call.tool);
    if !has_privilege(privileges, priv_req) {
        return Ok(ToolResult {
            tool: call.tool.clone(),
            success: false,
            message: format!("Privilège requis : {priv_req}"),
            data: None,
            requires_confirmation: false,
            pending_id: None,
            confirm_privilege: None,
        });
    }
    let pending_id = Uuid::new_v4().to_string();
    let params_json = serde_json::to_string(&call.params).map_err(|e| e.to_string())?;
    let confirm_req = tool_confirm_privilege(&call.tool);
    db.ai_insert_pending(
        &pending_id,
        conversation_id,
        &call.tool,
        &params_json,
        priv_req,
        confirm_req,
    )
    .map_err(|e| e.to_string())?;

    let default_msg = if call.tool == "delete_all_biens" {
        let n = count_biens_for_mass_delete(db).unwrap_or(0);
        format!(
            "⚠️ SUPPRESSION MASSIVE — {n} bien(s) seront supprimés définitivement (hors hangars).\n\n\
             Seul un utilisateur avec le rôle **Directeur** peut confirmer cette action."
        )
    } else if call.tool.starts_with("delete_") {
        format!(
            "⚠️ Suppression définitive — confirmez pour supprimer « {} ».",
            call.params
                .get("reference")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
        )
    } else {
        format!("Action « {} » en attente de confirmation.", call.tool)
    };
    Ok(ToolResult {
        tool: call.tool.clone(),
        success: true,
        message: call.explain.clone().unwrap_or(default_msg),
        data: Some(call.params.clone()),
        requires_confirmation: true,
        pending_id: Some(pending_id),
        confirm_privilege: confirm_req.map(str::to_string),
    })
}

pub fn execute_pending(
    db: &Database,
    pending_id: &str,
    privileges: &[String],
) -> Result<ToolResult, String> {
    let pending = db.ai_get_pending(pending_id).map_err(|e| e.to_string())?;
    if is_retired_immobilier_tool(&pending.tool_name) {
        return Err(RETIRED_IMMOBILIER_MSG.to_string());
    }
    let confirm_req = pending
        .confirm_privilege
        .as_deref()
        .unwrap_or(pending.privilege.as_str());
    if !has_privilege(privileges, confirm_req) {
        return Err(if pending.confirm_privilege.is_some() {
            format!(
                "Cette action sensible nécessite le rôle Directeur (privilège {confirm_req})."
            )
        } else {
            format!("Privilège requis : {confirm_req}")
        });
    }
    let params: Value =
        serde_json::from_str(&pending.params_json).map_err(|e| format!("Params invalides : {e}"))?;

    let result = execute_crud_write(db, &pending.tool_name, &params)?;

    db.ai_delete_pending(pending_id).map_err(|e| e.to_string())?;

    if let Ok(Some(user_msg)) = db.ai_last_user_message(&pending.conversation_id) {
        record_success(
            db,
            &user_msg,
            &pending.tool_name,
            &params,
            "confirmed",
        );
    }

    Ok(ToolResult {
        tool: pending.tool_name,
        success: true,
        message: result,
        data: None,
        requires_confirmation: false,
        pending_id: None,
        confirm_privilege: None,
    })
}
