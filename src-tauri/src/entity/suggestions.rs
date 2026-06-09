use std::fs;
use std::path::Path;

use crate::entity::registry::EntityRegistry;
use crate::privileges::has_any_entity_privilege;
use crate::session::SessionUser;

fn humanize_suggestion_label(nom: &str) -> String {
    let s = nom.replace('_', " ");
    let mut chars = s.chars();
    match chars.next() {
        None => nom.to_string(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntitySuggestion {
    pub key: String,
    pub label: String,
    pub phrase: String,
    pub privilege: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DashboardSuggestionsCatalog {
    pub version: u32,
    pub items: Vec<EntitySuggestion>,
}

pub fn catalog_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("entities").join("dashboard_suggestions.json")
}

/// Catalogue complet dérivé du registre (une entrée par entité).
pub fn build_catalog(registry: &EntityRegistry) -> DashboardSuggestionsCatalog {
    let mut items = Vec::new();
    for ent in &registry.entities {
        if super::registry::is_orphan_entity_key(&ent.nom) {
            continue;
        }
        if !ent.ai_suggestions {
            continue;
        }
        let label = ent
            .label
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| humanize_suggestion_label(&ent.nom));
        items.push(EntitySuggestion {
            key: ent.nom.clone(),
            phrase: format!("Gérer les {label}"),
            label,
            privilege: format!("{}:voir", ent.nom),
        });
    }
    items.sort_by(|a, b| {
        a.phrase
            .to_lowercase()
            .cmp(&b.phrase.to_lowercase())
    });
    DashboardSuggestionsCatalog {
        version: 1,
        items,
    }
}

pub fn list_for_user(_data_dir: &Path, registry: &EntityRegistry, user: &SessionUser) -> Vec<EntitySuggestion> {
    let mut items: Vec<EntitySuggestion> = build_catalog(registry)
        .items
        .into_iter()
        .filter(|s| has_any_entity_privilege(&user.privileges, &s.key))
        .collect();
    items.sort_by(|a, b| {
        a.phrase
            .to_lowercase()
            .cmp(&b.phrase.to_lowercase())
    });
    items
}

/// Trigger auto : catalogue JSON + mémoire IA (jamais de suggestions codées à la main).
pub fn write_dashboard_suggestions_trigger(
    data_dir: &Path,
    registry: &EntityRegistry,
) -> Result<(), String> {
    let catalog = build_catalog(registry);
    let entities_dir = data_dir.join("entities");
    fs::create_dir_all(&entities_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?;
    fs::write(catalog_path(data_dir), json).map_err(|e| e.to_string())?;

    let dir = data_dir.join("dda").join("knowledge");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut s = String::from(
        "=== SUGGESTIONS BARRE DE COMMANDE (généré automatiquement) ===\n\
         Fichier : entities/dashboard_suggestions.json — une phrase par entité du registre.\n\
         Seules les entités avec ai_suggestions=true (défaut) sont listées.\n\
         Affichage UI : filtré par privilège {nom}:voir du rôle connecté.\n\
         L'inventaire « stock » n'est pas dans cette liste : menu latéral Stock (si module actif).\n\n",
    );
    for item in &catalog.items {
        s.push_str(&format!(
            "- `{}` → `{}` → « {} »\n",
            item.key, item.privilege, item.phrase
        ));
    }
    fs::write(
        dir.join("MASTER_entity_dashboard_suggestions.txt"),
        &s,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
