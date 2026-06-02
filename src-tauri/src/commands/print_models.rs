use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::dda::crud;
use crate::db_io::{PrintModelDetail, PrintModelRow};
use crate::entity;
use crate::print_template::{
    build_fiche_html_from_config, render_data_table_html, substitute_list_document, substitute_row,
    table_token_for_entity, FICHE_CSS, LIST_CSS,
};
use crate::AppState;

fn require_print_models_view(state: &State<'_, AppState>) -> Result<(), String> {
    let session = state.desktop_sessions.require_session()?;
    if crate::privileges::has_privilege(&session.user.privileges, "documents:modeles_voir")
        || crate::privileges::has_privilege(&session.user.privileges, "ai:utiliser")
    {
        return Ok(());
    }
    Err("Privilège requis : documents:modeles_voir".to_string())
}

fn require_print_models_manage(state: &State<'_, AppState>) -> Result<(), String> {
    let session = state.desktop_sessions.require_session()?;
    if crate::privileges::has_privilege(&session.user.privileges, "documents:modeles_gerer")
        || crate::privileges::has_privilege(&session.user.privileges, "ai:utiliser")
    {
        return Ok(());
    }
    Err("Privilège requis : documents:modeles_gerer".to_string())
}

#[derive(Deserialize)]
pub struct PrintModelIdRequest {
    pub id: String,
}

#[derive(Deserialize)]
pub struct PrintModelUpsertRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub html_content: String,
    pub css_content: String,
    pub screen_key: Option<String>,
}

#[tauri::command]
pub fn print_models_list(state: State<'_, AppState>) -> Result<Vec<PrintModelRow>, String> {
    require_print_models_view(&state)?;
    let db = state.db.lock();
    db.list_print_models().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn print_models_get(
    state: State<'_, AppState>,
    payload: PrintModelIdRequest,
) -> Result<PrintModelDetail, String> {
    require_print_models_view(&state)?;
    let db = state.db.lock();
    db.get_print_model(&payload.id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn print_models_upsert(
    state: State<'_, AppState>,
    payload: PrintModelUpsertRequest,
) -> Result<PrintModelDetail, String> {
    require_print_models_manage(&state)?;
    let name = payload.name.trim();
    if name.is_empty() {
        return Err("Nom du modèle requis".to_string());
    }
    let db = state.db.lock();
    db.upsert_print_model(
        payload.id.as_deref(),
        name,
        payload.description.as_deref().unwrap_or("").trim(),
        &payload.html_content,
        &payload.css_content,
        payload.screen_key.as_deref().map(str::trim).filter(|s| !s.is_empty()),
    )
    .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct PrintModelDeleteResponse {
    pub success: bool,
}

#[tauri::command]
pub fn print_models_delete(
    state: State<'_, AppState>,
    payload: PrintModelIdRequest,
) -> Result<PrintModelDeleteResponse, String> {
    require_print_models_manage(&state)?;
    let db = state.db.lock();
    let success = db
        .delete_print_model(&payload.id)
        .map_err(|e| e.to_string())?;
    Ok(PrintModelDeleteResponse { success })
}

#[derive(Deserialize)]
pub struct PrintRowRenderPayload {
    pub screen_key: String,
    pub record_id: String,
}

#[derive(Serialize)]
pub struct PrintRowRenderResponse {
    pub html: String,
    pub css: String,
    pub file_name: String,
    pub model_name: String,
}

#[tauri::command]
pub fn print_row_render(
    state: State<'_, AppState>,
    payload: PrintRowRenderPayload,
) -> Result<PrintRowRenderResponse, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.screen_key)?
    };
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;

    let db = state.db.lock();
    let row_map = crud::get_row(&db, &cfg, &payload.record_id)?;
    let row: HashMap<String, Value> = row_map.into_iter().collect();

    let model = db
        .get_fiche_print_model_for_screen(&payload.screen_key)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Aucun modèle d'impression pour « {} ». Enregistrez le registre des entités ou créez un modèle dans Paramètres.",
                cfg.screen.label
            )
        })?;

    let html_body = substitute_row(
        &model.html_content,
        &row,
        &cfg.fields,
        &cfg.screen.key,
    );
    let file_label = row
        .get(&cfg.screen.label_field)
        .or_else(|| row.get(&cfg.screen.primary_key))
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => payload.record_id.clone(),
        })
        .unwrap_or_else(|| payload.record_id.clone());

    Ok(PrintRowRenderResponse {
        html: html_body,
        css: if model.css_content.trim().is_empty() {
            FICHE_CSS.to_string()
        } else {
            model.css_content.clone()
        },
        file_name: format!("Fiche-{}.pdf", sanitize_file_name(&file_label)),
        model_name: model.name,
    })
}

#[derive(Serialize)]
pub struct PrintTemplateDefaultsResponse {
    pub html: String,
    pub css: String,
}

#[tauri::command]
pub fn print_models_defaults(
    state: State<'_, AppState>,
    payload: super::entity::EntityKeyPayload,
) -> Result<PrintTemplateDefaultsResponse, String> {
    require_print_models_view(&state)?;
    let db = state.db.lock();
    let cfg = entity::load_screen_config(&db.data_dir, &payload.entity_key)?;
    Ok(PrintTemplateDefaultsResponse {
        html: build_fiche_html_from_config(&cfg),
        css: FICHE_CSS.to_string(),
    })
}

#[derive(Deserialize)]
pub struct PrintListRenderPayload {
    pub screen_key: String,
    /// Clés de champs à afficher (vide = colonnes liste par défaut).
    #[serde(default)]
    pub visible_columns: Vec<String>,
    #[serde(default)]
    pub filters: HashMap<String, String>,
    #[serde(default)]
    pub date_field: Option<String>,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    /// Filtre entité source (écran stock : champ entite_source).
    #[serde(default)]
    pub entity_source_filter: Option<String>,
    #[serde(default)]
    pub titre: Option<String>,
    #[serde(default)]
    pub sous_titre: Option<String>,
}

fn filter_rows_for_print(
    rows: Vec<HashMap<String, Value>>,
    cfg: &crate::dda::config::ScreenConfigFile,
    payload: &PrintListRenderPayload,
) -> Vec<HashMap<String, Value>> {
    rows.into_iter()
        .filter(|row| row_matches_print_filters(row, cfg, payload))
        .collect()
}

fn row_matches_print_filters(
    row: &HashMap<String, Value>,
    cfg: &crate::dda::config::ScreenConfigFile,
    payload: &PrintListRenderPayload,
) -> bool {
    if let Some(ref src) = payload.entity_source_filter {
        if !src.trim().is_empty() {
            let v = row
                .get("entite_source")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            if v != src.trim() {
                return false;
            }
        }
    }
    if let (Some(field_key), Some(from), Some(to)) = (
        payload.date_field.as_deref().filter(|s| !s.is_empty()),
        payload.date_from.as_deref().filter(|s| !s.is_empty()),
        payload.date_to.as_deref().filter(|s| !s.is_empty()),
    ) {
        let raw = row.get(field_key).and_then(|v| v.as_str()).unwrap_or("");
        if !raw.is_empty() {
            let d = &raw[..raw.len().min(10)];
            if d < from || d > to {
                return false;
            }
        }
    } else if let (Some(field_key), Some(from)) = (
        payload.date_field.as_deref().filter(|s| !s.is_empty()),
        payload.date_from.as_deref().filter(|s| !s.is_empty()),
    ) {
        let raw = row.get(field_key).and_then(|v| v.as_str()).unwrap_or("");
        if !raw.is_empty() {
            let d = &raw[..raw.len().min(10)];
            if d < from {
                return false;
            }
        }
    } else if let (Some(field_key), Some(to)) = (
        payload.date_field.as_deref().filter(|s| !s.is_empty()),
        payload.date_to.as_deref().filter(|s| !s.is_empty()),
    ) {
        let raw = row.get(field_key).and_then(|v| v.as_str()).unwrap_or("");
        if !raw.is_empty() {
            let d = &raw[..raw.len().min(10)];
            if d > to {
                return false;
            }
        }
    }
    for (key, val) in &payload.filters {
        if val.trim().is_empty() {
            continue;
        }
        let needle = val.trim().to_lowercase();
        let field = cfg.fields.iter().find(|f| f.key == *key || f.column == *key);
        let raw = row
            .get(key)
            .or_else(|| field.map(|f| row.get(&f.key)).flatten())
            .or_else(|| field.map(|f| row.get(&f.column)).flatten());
        let hay = value_to_filter_string(raw);
        if !hay.to_lowercase().contains(&needle) {
            return false;
        }
    }
    true
}

fn value_to_filter_string(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

#[tauri::command]
pub fn print_list_render(
    state: State<'_, AppState>,
    payload: PrintListRenderPayload,
) -> Result<PrintRowRenderResponse, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.screen_key)?
    };
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;

    let db = state.db.lock();
    let rows_maps = crud::list_rows(&db, &cfg, &HashMap::new())?;
    let rows: Vec<HashMap<String, Value>> = rows_maps.into_iter().map(|m| m.into_iter().collect()).collect();
    let filtered = filter_rows_for_print(rows, &cfg, &payload);

    let model = db
        .get_list_print_model_for_screen(&payload.screen_key)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Aucun modèle « Liste » pour « {} ». Enregistrez le registre des entités ou créez un modèle dans Paramètres.",
                cfg.screen.label
            )
        })?;

    let table_html = render_data_table_html(&filtered, &cfg.fields, &payload.visible_columns);
    let (app_name, slogan) = entity::branding::load_branding(&db.data_dir);
    let titre = payload
        .titre
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&cfg.screen.label);
    let sous_titre = payload.sous_titre.as_deref().unwrap_or("").to_string();
    let sous_titre = if sous_titre.is_empty() {
        format!(
            "{} ligne(s) — variable {{{}}}",
            filtered.len(),
            table_token_for_entity(&payload.screen_key)
        )
    } else {
        sous_titre
    };

    let html_body = substitute_list_document(
        &model.html_content,
        &payload.screen_key,
        &table_html,
        titre,
        &sous_titre,
        &app_name,
        &slogan,
    );

    Ok(PrintRowRenderResponse {
        html: html_body,
        css: if model.css_content.trim().is_empty() {
            LIST_CSS.to_string()
        } else {
            model.css_content.clone()
        },
        file_name: format!(
            "Liste-{}-{}.pdf",
            sanitize_file_name(&payload.screen_key),
            chrono::Local::now().format("%Y%m%d")
        ),
        model_name: model.name,
    })
}

fn sanitize_file_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();
    let t = cleaned.trim_matches('-').trim_matches('_');
    if t.is_empty() {
        "document".to_string()
    } else {
        t.to_string()
    }
}
