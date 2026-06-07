use std::fs;
use std::path::Path;

use super::registry::{EntityDef, EntityRegistry};
use super::schema::{attr_column, table_name};
use super::stock::{self, STOCK_ATTR_TYPE, STOCK_ENTITY_KEY};

pub fn finalize_entity_knowledge(data_dir: &Path, registry: &EntityRegistry) -> Result<(), String> {
    let dir = data_dir.join("dda").join("knowledge");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut schema = String::from(
        "=== BLIN — ENTITÉS MÉTIER (registre Paramètres, auto) ===\n\
         Source : {app_data}/entities/registry.json\n\
         Chaque entité = une table SQLite ent_{nom} + formulaire/liste sur le tableau de bord.\n\
         Ne pas créer de nouvel écran ni de menu : utiliser le tableau de bord.\n\
         Champs registre entité : nom, label, description, ai_suggestions (bool, défaut true), attributs[].\n\
         Champs attribut : nom, type, label, required (bool), ref (liaison entity), default, enum_options.\n\
         Types : string, number, integer, float, boolean, date, time, datetime, email, photo, enum, entity, stock, compteur.\n\
         compteur : un attribut → 3 colonnes auto ({nom}_libelle, {nom}_jjmmaaaa, {nom}_numero) — non saisies par l'utilisateur.\n\
         ai_suggestions=true uniquement si le formulaire a une liaison entity vers une entité ai_suggestions=false (fiche technique).\n\
         ai_suggestions=false → absent de la barre « Que souhaitez-vous faire ? » (ex. users, matiere sans lien technique).\n\
         required=true → champ obligatoire à la création/édition (validation + NOT NULL SQLite si texte).\n\
         requires_validation=true → trigger à chaque création (create_row) : une tâche validation privée par rôle valideur ; entité tache requise.\n\
         is_session=true → contexte métier : session active (fichier active_business_session.json) ; listes filtrées et création préremplie sur liaison entity vers cette session.\n\
         Tâche type validation/destockage : entite_a_valider + enregistrement_id obligatoires ; role_validateur si validation.\n\
         Tâche visibilite : publique (tous) | privee (role_validateur) | personnalisee (roles_visibles = liste d'IDs rôles).\n\
         Liste Tâches filtrée par rôle de l'utilisateur connecté (admin * voit tout).\n\n",
    );
    let mut tools = String::from(
        "=== LOGGY — GESTION ENTITÉS VIA TABLEAU DE BORD ===\n\
         Quand l'utilisateur demande de gérer une entité (ex. users, tache, clients) :\n\
         1) Confirmer l'entité cible (nom exact du registre).\n\
         2) Indiquer que l'interface affiche liste + formulaire modal (champs = attributs).\n\
         3) CRUD via outils DDA : dda_list, dda_get, dda_create, dda_update, dda_delete avec screen_key = nom entité.\n\
         4) Liste dans le chat : « liste les {entité} » ou « liste les {entité} avec nom, prix » — tableau HTML read-only + bouton ouvrir écran.\n\
         5) Jointures : si l'utilisateur cite une entité liée (entity ref), inclure la colonne libellé de l'entité cible (voir MASTER_entities_relations.txt).\n\
         6) Paramètres > Entités : ajouter/modifier le JSON du registre (tables synchronisées automatiquement).\n\n",
    );

    if stock::registry_has_stock(registry) {
        schema.push_str(STOCK_MODULE_SCHEMA);
        tools.push_str(STOCK_MODULE_TOOLS);
    }

    for ent in &registry.entities {
        schema.push_str(&format_entity_schema(ent, registry));
        schema.push('\n');
        tools.push_str(&format_entity_tools(ent));
        tools.push('\n');
    }

    fs::write(dir.join("MASTER_entities_schema.txt"), &schema).map_err(|e| e.to_string())?;
    fs::write(dir.join("MASTER_entities_tools.txt"), &tools).map_err(|e| e.to_string())?;

    let relations = format_relations_catalog(registry);
    fs::write(dir.join("MASTER_entities_relations.txt"), &relations).map_err(|e| e.to_string())?;

    if stock::registry_has_stock(registry) {
        fs::write(dir.join("MASTER_stock_module.txt"), STOCK_MODULE_SCHEMA)
            .map_err(|e| e.to_string())?;
        fs::write(
            dir.join("MASTER_stock_tools.txt"),
            STOCK_MODULE_TOOLS,
        )
        .map_err(|e| e.to_string())?;
    }

    for ent in &registry.entities {
        let s = format_entity_schema(ent, registry);
        fs::write(dir.join(format!("{}_entity_schema.txt", ent.nom)), s).map_err(|e| e.to_string())?;
        let rel = format_entity_relations(ent, registry);
        fs::write(
            dir.join(format!("{}_entity_relations.txt", ent.nom)),
            rel,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

const STOCK_MODULE_SCHEMA: &str = "\
=== MODULE STOCK (registre : au moins un attribut type « stock ») ===\n\
L'entité « stock » est injectée automatiquement — NE PAS la déclarer dans registry.json.\n\
Écran : menu latéral « Stock » (modal), privilèges stock:voir / stock:modifier / …\n\
Ligne inventaire : entite_source, enregistrement_id, libelle, quantite, article_perissable, date_peremption.\n\
Sync : fiche métier (attribut stock) → inventaire ; modification/déstockage inventaire → fiche métier.\n\
Périssable + date_peremption obligatoire si article_perissable=true.\n\
Péremption ≤ 30 jours → tâche type_tache=destockage (entité tache requise, enum avec destockage).\n\n";

const STOCK_MODULE_TOOLS: &str = "\
=== OUTILS STOCK (complément DDA) ===\n\
  entity_stock_status {} — module actif + droits utilisateur\n\
  entity_stock_scan_destock {} — scan alertes péremption\n\
  entity_stock_destock {{ stock_id, quantity? }} — retirer du stock (quantity absent = tout)\n\
  dda_list / dda_update {{ screen_key: \"stock\" }} — consulter ou éditer péremption / périssable\n\
  Les lignes stock se créent via dda_create/dda_update sur l'entité source ayant un attribut type stock.\n\
  Ne pas dda_create sur stock pour inventaire initial (synchro auto).\n\n";

fn format_entity_schema(ent: &EntityDef, registry: &EntityRegistry) -> String {
    let table = table_name(&ent.nom);
    let suggestions = if ent.ai_suggestions {
        "oui (barre suggestions IA)"
    } else {
        "non"
    };
    let mut s = format!(
        "ENTITÉ « {} » (label: {}) — table {} — suggestions IA : {}\nAttributs :\n",
        ent.nom,
        ent.label.as_deref().unwrap_or(&ent.nom),
        table,
        suggestions
    );
    if ent.nom == STOCK_ENTITY_KEY {
        s.push_str(
            "  [SYSTÈME] Générée automatiquement — ne pas ajouter au JSON importé.\n\
  article_perissable (boolean) ; date_peremption (date) requise si périssable.\n",
        );
    }
    let has_stock_module = stock::registry_has_stock(registry);
    for attr in &ent.attributs {
        let req = if attr.required { " — obligatoire" } else { "" };
        let mut line = format!(
            "  - {} : {} (colonne {}){}\n",
            attr.nom,
            attr.attr_type,
            attr_column(attr),
            req
        );
        if has_stock_module
            && ent.nom != STOCK_ENTITY_KEY
            && attr.attr_type == STOCK_ATTR_TYPE
        {
            line.push_str(
                "    → sync inventaire « stock » à chaque enregistrement (menu Stock).\n",
            );
        }
        s.push_str(&line);
    }
    s.push_str(&format!(
        "Privilèges : {}:voir, {}:creer, {}:modifier, {}:supprimer\n",
        ent.nom, ent.nom, ent.nom, ent.nom
    ));
    s
}

fn format_entity_tools(ent: &EntityDef) -> String {
    let key = &ent.nom;
    if key == STOCK_ENTITY_KEY {
        return format!(
            "Entité {key} (système, menu Stock) :\n\
  dda_list {{ screen_key: \"{key}\", filters: {{}} }}\n\
  dda_update {{ screen_key: \"{key}\", id, data }} — quantité, périssable, péremption\n\
  entity_stock_destock {{ stock_id, quantity? }}\n\
  Pas de dda_create manuel : lignes créées par sync depuis entités avec attribut stock.\n"
        );
    }
    format!(
        "Entité {key} :\n  dda_list {{ screen_key: \"{key}\", filters: {{}} }}\n  dda_get {{ screen_key: \"{key}\", id }}\n  dda_create {{ screen_key: \"{key}\", data: {{...attributs}} }}\n  dda_update {{ screen_key: \"{key}\", id, data }}\n  dda_delete {{ screen_key: \"{key}\", id }}\n  entity_export_csv {{ entity_key: \"{key}\" }} — CSV (;), 1ère ligne = noms des champs\n  entity_import_csv {{ entity_key: \"{key}\", csv }} — import CSV uniquement ; si entité non précisée, demander laquelle\n  Chat liste : « liste les {key} » puis choix colonnes ; « avec <entité liée> » pour jointure.\n  Export multi-entités : appeler entity_export_csv pour chaque entité demandée.\n"
    )
}

fn format_entity_relations(ent: &EntityDef, registry: &EntityRegistry) -> String {
    let mut s = format!(
        "=== RELATIONS — entité « {} » ===\n\
         Trigger auto à la création / sauvegarde du registre.\n\
         Jointures possibles (liaisons entity, one-to-one / many-to-one) :\n",
        ent.nom
    );
    let mut any = false;
    for attr in &ent.attributs {
        if attr.attr_type != "entity" {
            continue;
        }
        let Some(ref_key) = attr.r#ref.as_deref().map(str::trim).filter(|x| !x.is_empty()) else {
            continue;
        };
        any = true;
        let multiple = attr.relation_multiple;
        let card = if multiple {
            "one-to-many (liste JSON)"
        } else {
            "many-to-one (FK id)"
        };
        let ref_label = registry
            .find(ref_key)
            .and_then(|e| e.label.clone())
            .unwrap_or_else(|| ref_key.to_string());
        s.push_str(&format!(
            "  - champ « {} » ({}) → entité « {} » ({}) — colonne SQLite `{}`\n\
             Jointure chat : « liste les {} avec {} » affiche `{}` + libellé `{}`.\n",
            attr.nom,
            card,
            ref_key,
            ref_label,
            super::schema::attr_column(attr),
            ent.nom,
            ref_key,
            attr.nom,
            ref_key
        ));
    }
    if !any {
        s.push_str("  (aucune liaison entity)\n");
    }
    s
}

fn format_relations_catalog(registry: &EntityRegistry) -> String {
    let mut s = String::from(
        "=== BLIN — CATALOGUE JOINTURES ENTITÉS (auto, trigger registre) ===\n\
         Généré à chaque apply_registry / création d'entité.\n\
         Utilisation chat : listes read-only avec colonnes demandées + jointures implicites.\n\
         Fichiers par entité : {nom}_entity_relations.txt\n\n",
    );
    for ent in &registry.entities {
        s.push_str(&format_entity_relations(ent, registry));
        s.push('\n');
    }
    s
}
