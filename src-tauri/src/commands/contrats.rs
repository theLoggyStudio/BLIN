use serde::Deserialize;
use tauri::State;

use crate::db::{ContratInput, ContratRow};
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateContratRequest {
    pub reference: String,
    pub bien_id: Option<String>,
    pub locataire: String,
    pub locataire_email: String,
    pub locataire_telephone: String,
    pub loyer_mensuel: f64,
    pub date_debut: String,
    pub date_fin: Option<String>,
    pub logement_cle: Option<String>,
    pub devise: Option<String>,
}

#[derive(Deserialize)]
pub struct ContratIdRequest {
    pub id: String,
}

fn request_to_input<'a>(p: &'a CreateContratRequest) -> ContratInput<'a> {
    ContratInput {
        reference: p.reference.trim(),
        bien_id: p.bien_id.as_deref().filter(|s| !s.is_empty()),
        locataire: p.locataire.trim(),
        locataire_email: p.locataire_email.trim(),
        locataire_telephone: p.locataire_telephone.trim(),
        loyer_mensuel: p.loyer_mensuel,
        date_debut: p.date_debut.trim(),
        date_fin: p.date_fin.as_deref().filter(|s| !s.is_empty()),
        logement_cle: p.logement_cle.as_deref().filter(|s| !s.is_empty()),
        devise: p.devise.as_deref().unwrap_or(""),
    }
}

#[tauri::command]
pub fn contrats_list(state: State<'_, AppState>) -> Result<Vec<ContratRow>, String> {
    state.desktop_sessions.require_privilege("contrats:voir")?;
    let db = state.db.lock();
    db.list_contrats().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn contrats_create(
    state: State<'_, AppState>,
    payload: CreateContratRequest,
) -> Result<ContratRow, String> {
    state.desktop_sessions.require_privilege("contrats:signer")?;
    let db = state.db.lock();
    db.create_contrat(request_to_input(&payload))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn contrats_sign(
    state: State<'_, AppState>,
    payload: ContratIdRequest,
) -> Result<ContratRow, String> {
    state.desktop_sessions.require_privilege("contrats:signer")?;
    let db = state.db.lock();
    db.sign_contrat(&payload.id).map_err(|e| e.to_string())
}
