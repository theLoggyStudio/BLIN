use serde::Deserialize;
use tauri::State;

use crate::bien_detail::BienEtageNom;
use crate::db::{BienInput, BienRow};
use crate::AppState;

#[derive(Deserialize)]
pub struct BienWritePayload {
    pub reference: String,
    pub adresse: String,
    pub type_bien: String,
    pub statut: Option<String>,
    pub surface_m2: Option<f64>,
    pub prix_defaut: Option<f64>,
    pub nb_etages: Option<i32>,
    pub nb_chambres: Option<i32>,
    pub nb_pieces: Option<i32>,
    pub domaine: Option<String>,
    pub devise: Option<String>,
    pub zone: Option<String>,
    pub capacite_m3: Option<f64>,
    #[serde(default)]
    pub nomenclature: Vec<BienEtageNom>,
}

#[derive(Deserialize)]
pub struct UpdateBienRequest {
    pub id: String,
    pub reference: String,
    pub adresse: String,
    pub type_bien: String,
    pub statut: Option<String>,
    pub surface_m2: Option<f64>,
    pub prix_defaut: Option<f64>,
    pub nb_etages: Option<i32>,
    pub nb_chambres: Option<i32>,
    pub nb_pieces: Option<i32>,
    pub domaine: Option<String>,
    pub devise: Option<String>,
    pub zone: Option<String>,
    pub capacite_m3: Option<f64>,
    #[serde(default)]
    pub nomenclature: Vec<BienEtageNom>,
}

#[derive(Deserialize)]
pub struct DeleteBienRequest {
    pub id: String,
}

fn write_to_input<'a>(p: &'a BienWritePayload) -> BienInput<'a> {
    BienInput {
        reference: p.reference.trim(),
        adresse: p.adresse.trim(),
        type_bien: p.type_bien.trim(),
        statut: p.statut.as_deref().map(str::trim),
        surface_m2: p.surface_m2,
        prix_defaut: p.prix_defaut,
        nb_etages: p.nb_etages,
        nb_chambres: p.nb_chambres,
        nb_pieces: p.nb_pieces,
        nomenclature: &p.nomenclature,
        domaine: p.domaine.as_deref().unwrap_or(""),
        devise: p.devise.as_deref().unwrap_or(""),
        zone: p.zone.as_deref(),
        capacite_m3: p.capacite_m3,
    }
}

#[tauri::command]
pub fn biens_list(state: State<'_, AppState>) -> Result<Vec<BienRow>, String> {
    state.desktop_sessions.require_privilege("biens:voir")?;
    let db = state.db.lock();
    db.list_biens().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn biens_create(
    state: State<'_, AppState>,
    payload: BienWritePayload,
) -> Result<BienRow, String> {
    state.desktop_sessions.require_privilege("biens:creer")?;
    let db = state.db.lock();
    db.create_bien(write_to_input(&payload))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn biens_update(
    state: State<'_, AppState>,
    payload: UpdateBienRequest,
) -> Result<BienRow, String> {
    state.desktop_sessions.require_privilege("biens:modifier")?;
    let db = state.db.lock();
    let inner = BienWritePayload {
        reference: payload.reference,
        adresse: payload.adresse,
        type_bien: payload.type_bien,
        statut: payload.statut,
        surface_m2: payload.surface_m2,
        prix_defaut: payload.prix_defaut,
        nb_etages: payload.nb_etages,
        nb_chambres: payload.nb_chambres,
        nb_pieces: payload.nb_pieces,
        domaine: payload.domaine,
        devise: payload.devise,
        zone: payload.zone,
        capacite_m3: payload.capacite_m3,
        nomenclature: payload.nomenclature,
    };
    db.update_bien(&payload.id, write_to_input(&inner))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn biens_delete(
    state: State<'_, AppState>,
    payload: DeleteBienRequest,
) -> Result<(), String> {
    state.desktop_sessions.require_privilege("biens:modifier")?;
    let db = state.db.lock();
    db.delete_bien(&payload.id).map_err(|e| e.to_string())
}
