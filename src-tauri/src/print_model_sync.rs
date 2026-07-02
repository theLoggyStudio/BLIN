//! Synchronisation des modèles d'impression de base (HTML/CSS) avec les colonnes d'entité.
//! Ne modifie jamais les modèles créés par les utilisateurs.

use std::path::Path;

use crate::dda::config::ScreenConfigFile;
use crate::db::Database;
use crate::entity::registry::{self, EntityRegistry};
use crate::entity::stock::STOCK_ENTITY_KEY;
use crate::print_seed::AUTO_PRINT_DESCRIPTION_PREFIX;
use crate::print_template::{
    auto_print_description, build_fiche_html_from_config, build_list_print_html,
    build_stock_list_print_html, FICHE_CSS, LIST_CSS,
};

/// Modèle généré automatiquement (préfixe description « Modèle auto DDA »).
pub fn is_base_print_model(description: &str) -> bool {
    description.trim().starts_with(AUTO_PRINT_DESCRIPTION_PREFIX)
}

/// Met à jour les modèles de base fiche + liste pour un écran après modification du registre.
pub fn sync_entity_base_print_models(db: &Database, cfg: &ScreenConfigFile) -> Result<(), String> {
    let screen_key = cfg.screen.key.as_str();
    let label = cfg.screen.label.as_str();

    if let Some(print) = &cfg.screen.print {
        if print.enabled && print.single_object {
            let base_name = print
                .template_name
                .clone()
                .unwrap_or_else(|| format!("Fiche {label}"));
            let html = build_fiche_html_from_config(cfg);
            let description = auto_print_description("fiche", screen_key);
            db.sync_base_print_model(
                screen_key,
                &base_name,
                &description,
                &html,
                FICHE_CSS,
                "fiche",
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let list_html = if screen_key == STOCK_ENTITY_KEY {
        build_stock_list_print_html()
    } else {
        build_list_print_html(cfg)
    };
    let list_name = format!("Liste — {label}");
    let list_desc = auto_print_description("liste", screen_key);
    db.sync_base_print_model(
        screen_key,
        &list_name,
        &list_desc,
        &list_html,
        LIST_CSS,
        "liste",
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Resynchronise tous les modèles auto DDA après enregistrement du registre (filet de sécurité).
pub fn resync_all_registry_print_models(
    db: &Database,
    data_dir: &Path,
    registry: &EntityRegistry,
) -> Result<(), String> {
    for ent in &registry.entities {
        if registry::is_orphan_entity_key(&ent.nom) {
            continue;
        }
        let cfg = crate::entity::config::build_screen_config(ent, registry, data_dir);
        sync_entity_base_print_models(db, &cfg)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dda::config::{
        FieldDef, FieldFormMeta, FieldListMeta, ListLayout, PrintMeta, ScreenConfigFile,
        ScreenLayout, ScreenMeta, ScreenPrivileges,
    };

    fn minimal_cfg(key: &str, fields: Vec<FieldDef>) -> ScreenConfigFile {
        ScreenConfigFile {
            screen: ScreenMeta {
                key: key.into(),
                label: "Test".into(),
                label_plural: None,
                icon: None,
                route: format!("/entite/{key}"),
                system: false,
                ai_editable: false,
                table: key.into(),
                primary_key: "id".into(),
                label_field: "nom".into(),
                default_order_by: None,
                privileges: ScreenPrivileges {
                    view: format!("{key}:voir"),
                    create: format!("{key}:creer"),
                    update: format!("{key}:modifier"),
                    delete: format!("{key}:supprimer"),
                    import: None,
                    export: None,
                },
                print: Some(PrintMeta {
                    enabled: true,
                    screen_key: key.into(),
                    single_object: true,
                    template_name: Some(format!("Fiche Test")),
                }),
                storage: None,
            },
            layout: ScreenLayout {
                list: ListLayout {
                    title: "Test".into(),
                    subtitle: None,
                    actions: vec![],
                    row_click: None,
                },
                forms: None,
            },
            fields,
        }
    }

    fn field(key: &str, label: &str) -> FieldDef {
        FieldDef {
            key: key.into(),
            column: key.into(),
            field_type: "text".into(),
            label: label.into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: true,
                sortable: true,
            }),
            filter: None,
            form: Some(FieldFormMeta {
                col_span: None,
                placeholder: None,
                min: None,
                step: None,
                read_only: None,
                auto_generated: None,
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: None,
                relation_exclusive_parent: None,
                relation_multiple: None,
                embed_parent: None,
            }),
            visible_when: None,
            validation: None,
        }
    }

    #[test]
    fn is_base_print_model_detects_auto_prefix() {
        assert!(is_base_print_model(
            "Modèle auto DDA — fiche — écran clients"
        ));
        assert!(!is_base_print_model("Mon modèle personnalisé"));
    }

    #[test]
    fn fiche_html_reflects_entity_columns() {
        let cfg = minimal_cfg(
            "clients",
            vec![field("nom", "Nom"), field("email", "E-mail")],
        );
        let html = crate::print_template::build_fiche_html_from_config(&cfg);
        assert!(html.contains("{{clients.nom}}"));
        assert!(html.contains("{{clients.email}}"));
        assert!(!html.contains("{{clients.telephone}}"));

        let cfg2 = minimal_cfg(
            "clients",
            vec![
                field("nom", "Nom"),
                field("email", "E-mail"),
                field("telephone", "Téléphone"),
            ],
        );
        let html2 = crate::print_template::build_fiche_html_from_config(&cfg2);
        assert!(html2.contains("{{clients.telephone}}"));
    }
}
