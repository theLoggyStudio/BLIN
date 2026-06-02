use std::fs;
use std::path::Path;

use super::config::{FieldDef, ScreenConfigFile};

/// Génère les fichiers knowledge par écran + catalogue maître pour Loggy.
pub fn write_screen_knowledge(data_dir: &Path, cfg: &ScreenConfigFile) -> Result<(), String> {
    let dir = data_dir.join("dda").join("knowledge");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let tools = format_screen_tools(cfg);
    let schema = format_screen_schema(cfg);
    let layout = format_screen_layout(cfg);

    fs::write(dir.join(format!("{}_tools.txt", cfg.screen.key)), tools)
        .map_err(|e| e.to_string())?;
    fs::write(
        dir.join(format!("{}_schema.txt", cfg.screen.key)),
        schema,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.join(format!("{}_layout.txt", cfg.screen.key)),
        layout,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Agrège tous les écrans DDA + règles médias — appelé après chaque sync globale.
pub fn finalize_master_knowledge(
    data_dir: &Path,
    configs: &[ScreenConfigFile],
) -> Result<(), String> {
    let dir = data_dir.join("dda").join("knowledge");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut master_tools = String::from(
        "=== LOGGY — OUTILS (généré automatiquement au démarrage / sync DDA) ===\n\
         Ne pas inventer de champs : respecter le schéma DDA de chaque écran.\n\
         Format: {\"tool\":\"nom\",\"params\":{...},\"explain\":\"...\"}\n\
         Toute écriture = confirmation utilisateur dans l'interface.\n\n",
    );
    let mut master_schema = String::from(
        "=== LOGGY — SCHÉMA MÉTIER DDA (auto) ===\n\
         Les écrans JSON dans src/constante/json sont la source de vérité.\n\n",
    );

    for cfg in configs {
        if cfg.screen.system {
            continue;
        }
        master_tools.push_str(&format_screen_tools(cfg));
        master_tools.push('\n');
        master_schema.push_str(&format_screen_schema(cfg));
        master_schema.push('\n');
    }

    master_tools.push_str(&entity_only_tools_footer());
    master_schema.push_str(&entity_only_schema_footer());

    let media = format_media_guide(configs);
    let layout_master = format_master_layout_guide(configs);

    fs::write(dir.join("MASTER_ia_tools.txt"), &master_tools).map_err(|e| e.to_string())?;
    fs::write(dir.join("MASTER_ia_schema.txt"), &master_schema).map_err(|e| e.to_string())?;
    fs::write(dir.join("MASTER_ia_media.txt"), &media).map_err(|e| e.to_string())?;
    fs::write(dir.join("MASTER_ia_layout.txt"), &layout_master).map_err(|e| e.to_string())?;
    Ok(())
}

fn format_screen_tools(cfg: &ScreenConfigFile) -> String {
    let key = &cfg.screen.key;
    let mut s = format!(
        "=== ÉCRAN {key} (DDA — table {}) ===\n",
        cfg.screen.table
    );
    s.push_str(&format!(
        "dda_list / list_{key}: filtres dynamiques — priv. {}\n",
        cfg.screen.privileges.view
    ));
    s.push_str(&format!(
        "dda_create / create_{key}: tous champs écran — priv. {}\n",
        cfg.screen.privileges.create
    ));
    s.push_str(&format!(
        "dda_update / update_{key}: id ou {} + champs à modifier — priv. {}\n",
        cfg.screen.label_field, cfg.screen.privileges.update
    ));
    s.push_str(&format!(
        "dda_delete / delete_{key}: id ou {} — priv. {}\n",
        cfg.screen.label_field, cfg.screen.privileges.delete
    ));
    if cfg.screen.storage.is_some() {
        s.push_str(
            "Médias : upload fichier = interface desktop/mobile uniquement (pas de base64 dans le chat).\n\
             L'IA peut renseigner photo_principale (chemin relatif) ou photos (tableau de chemins) si déjà uploadés.\n",
        );
    }
    s
}

fn format_screen_schema(cfg: &ScreenConfigFile) -> String {
    let mut s = format!(
        "=== {} (table {}, clé {}) ===\n",
        cfg.screen.label.to_uppercase(),
        cfg.screen.table,
        cfg.screen.primary_key
    );
    if let Some(storage) = &cfg.screen.storage {
        s.push_str(&format!(
            "Stockage fichiers : {}\n",
            storage.folders.join(", ")
        ));
    }
    for f in cfg.writable_columns() {
        s.push_str(&format_field_line(f));
    }
    for f in cfg.fields.iter().filter(|f| f.field_type == "hidden") {
        if f.column == "id" {
            s.push_str(&format!("{} (hidden) — identifiant interne\n", f.column));
        }
    }
    s
}

fn format_field_line(f: &FieldDef) -> String {
    let mut line = format!(
        "{} [{}] ({})",
        f.key,
        f.column,
        f.field_type
    );
    if f.required {
        line.push_str(" requis");
    }
    if !f.options.is_empty() {
        let opts: Vec<_> = f.options.iter().map(|o| o.value.as_str()).collect();
        line.push_str(&format!(" options={}", opts.join("|")));
    }
    if let Some(form) = &f.form {
        if let Some(folder) = &form.storage_folder {
            line.push_str(&format!(" dossier={folder}"));
        }
        if let Some(max) = form.max_files {
            line.push_str(&format!(" max_fichiers={max}"));
        }
    }
    if let Some(vw) = &f.visible_when {
        line.push_str(&format!(
            " visible_si {}={}",
            vw.field,
            serde_json::to_string(&vw.equals).unwrap_or_default()
        ));
    }
    if f.field_type == "image" {
        line.push_str(" — une image (chemin relatif photos/...)");
    }
    if f.field_type == "images" {
        line.push_str(" — galerie JSON [\"chemin1\",...]");
    }
    line.push('\n');
    line
}

/// Résumé layout DDA pour Loggy : liste (Table) vs formulaires (objet unique).
fn format_screen_layout(cfg: &ScreenConfigFile) -> String {
    let key = &cfg.screen.key;
    let list_cols: Vec<_> = cfg.list_columns().iter().map(|f| f.key.as_str()).collect();
    let filter_cols: Vec<_> = cfg.filter_fields().iter().map(|f| f.key.as_str()).collect();

    let mut s = format!("=== LAYOUT ÉCRAN {key} ===\n");
    s.push_str("Rappel : layout.list → item Table (liste) ; layout.forms → Modal/Offpanel (objet unique).\n\n");

    s.push_str(&format!(
        "layout.list.title = « {} »\n",
        cfg.layout.list.title
    ));
    if let Some(sub) = &cfg.layout.list.subtitle {
        s.push_str(&format!("layout.list.subtitle = « {sub} »\n"));
    }
    s.push_str(&format!(
        "layout.list.actions = {:?}\n",
        cfg.layout.list.actions
    ));
    s.push_str(&format!(
        "layout.list.rowClick = {:?}\n",
        cfg.layout.list.row_click
    ));

    if list_cols.is_empty() {
        s.push_str("⚠ AUCUNE colonne list.enabled — la Table serait vide. Activer list sur les champs à afficher.\n");
    } else {
        s.push_str(&format!(
            "Colonnes Table (fields[].list.enabled) : {}\n",
            list_cols.join(", ")
        ));
    }
    if !filter_cols.is_empty() {
        s.push_str(&format!(
            "Filtres FilterBar (fields[].filter.enabled) : {}\n",
            filter_cols.join(", ")
        ));
    }

    if let Some(forms) = &cfg.layout.forms {
        s.push_str("\nFormulaires objet unique (layout.forms) :\n");
        if let Some(c) = &forms.create {
            s.push_str(&format!(
                "  create : « {} » mode={} → Modal si modal, Offpanel si offpanel\n",
                c.title, c.mode
            ));
        }
        if let Some(e) = &forms.edit {
            s.push_str(&format!(
                "  edit   : « {} » mode={}\n",
                e.title, e.mode
            ));
        }
        if let Some(d) = &forms.detail {
            s.push_str(&format!(
                "  detail : « {} » mode={} readOnly={:?}\n",
                d.title, d.mode, d.read_only
            ));
        }
    } else {
        s.push_str("\n⚠ layout.forms absent — pas de création/édition objet unique via UI.\n");
    }

    if let Some(p) = &cfg.screen.print {
        if p.single_object {
            s.push_str("\nImpression : fiche OBJET UNIQUE (singleObject: true) — pas une liste.\n");
        }
    }

    s
}

fn format_master_layout_guide(configs: &[ScreenConfigFile]) -> String {
    let mut s = include_str!("../ai/knowledge_layout.txt").to_string();
    s.push_str("\n\n=== ÉTAT LAYOUT PAR ÉCRAN (auto sync DDA) ===\n");
    for cfg in configs {
        if cfg.screen.system {
            continue;
        }
        s.push_str(&format_screen_layout(cfg));
        s.push('\n');
    }
    s
}

fn format_media_guide(configs: &[ScreenConfigFile]) -> String {
    let mut s = String::from(
        "=== LOGGY — GUIDE MÉDIAS (auto DDA) ===\n\
         Les champs type image / photo stockent des chemins relatifs sous le dossier données app.\n\
         Upload binaire : formulaire entité sur le tableau de bord, PAS via le chat Loggy.\n\n",
    );
    for cfg in configs {
        let media_fields: Vec<_> = cfg
            .media_fields()
            .iter()
            .map(|f| format!("  - {} ({})", f.label, f.field_type))
            .collect();
        if media_fields.is_empty() {
            continue;
        }
        s.push_str(&format!("Écran « {} » :\n", cfg.screen.label));
        if let Some(st) = &cfg.screen.storage {
            s.push_str(&format!("  Dossiers : {}\n", st.folders.join(", ")));
        }
        for line in media_fields {
            s.push_str(&line);
            s.push('\n');
        }
        s.push('\n');
    }
    s
}

fn entity_only_tools_footer() -> String {
    include_str!("../ai/knowledge_tools.txt").to_string()
}

fn entity_only_schema_footer() -> String {
    include_str!("../ai/knowledge_schema.txt").to_string()
}
