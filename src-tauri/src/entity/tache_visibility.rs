//! Visibilité des tâches : publique, privée (rôle valideur), personnalisée (rôles choisis).

use serde_json::{Map, Value};

use super::registry::{EntityAttribute, EntityRegistry};
use super::schema::{table_has_column, table_name};
use crate::db::Database;
use crate::privileges::has_privilege;

pub const TACHE_ENTITY_KEY: &str = "tache";
pub const COL_VISIBILITE: &str = "visibilite";
pub const COL_ROLES_VISIBLES: &str = "roles_visibles";

pub const VIS_PUBLIQUE: &str = "publique";
pub const VIS_PRIVEE: &str = "privee";
pub const VIS_PERSONNALISEE: &str = "personnalisee";

/// Ajoute les attributs visibilité au registre entité `tache` s'ils manquent.
pub fn ensure_tache_visibility_in_registry(registry: &mut EntityRegistry) {
    let Some(ent) = registry.entities.iter_mut().find(|e| e.nom == TACHE_ENTITY_KEY) else {
        return;
    };
    if ent.attributs.iter().any(|a| a.nom == COL_VISIBILITE) {
        return;
    }
    ent.attributs.push(EntityAttribute {
        nom: COL_VISIBILITE.into(),
        attr_type: "enum[publique,privee,personnalisee]".into(),
        label: Some("Visibilité".into()),
        required: false,
        r#ref: None,
        relation_multiple: false,
        relation_exclusive_parent: true,
        default: Some(Value::String(VIS_PUBLIQUE.into())),
        enum_options: None,
    });
    ent.attributs.push(EntityAttribute {
        nom: COL_ROLES_VISIBLES.into(),
        attr_type: "string".into(),
        label: Some("Rôles autorisés (visibilité personnalisée)".into()),
        required: false,
        r#ref: None,
        relation_multiple: false,
        relation_exclusive_parent: true,
        default: None,
        enum_options: None,
    });
}

pub fn ensure_visibility_columns(db: &Database) -> Result<(), String> {
    let table = table_name(TACHE_ENTITY_KEY);
    if !table_has_column(db, &table, COL_VISIBILITE)? {
        db.conn
            .execute(
                &format!(
                    "ALTER TABLE {table} ADD COLUMN {COL_VISIBILITE} TEXT NOT NULL DEFAULT '{VIS_PUBLIQUE}'"
                ),
                [],
            )
            .map_err(|e| format!("ALTER {table}.{COL_VISIBILITE} : {e}"))?;
    }
    if !table_has_column(db, &table, COL_ROLES_VISIBLES)? {
        db.conn
            .execute(
                &format!("ALTER TABLE {table} ADD COLUMN {COL_ROLES_VISIBLES} TEXT"),
                [],
            )
            .map_err(|e| format!("ALTER {table}.{COL_ROLES_VISIBLES} : {e}"))?;
    }
    Ok(())
}

/// Liste CSV de rôles → `,role-a,role-b,` pour recherche SQL.
pub fn encode_roles_csv(role_ids: &[String]) -> String {
    let mut ids: Vec<&str> = role_ids
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return String::new();
    }
    format!(",{},", ids.join(","))
}

pub fn parse_roles_csv(raw: Option<&str>) -> Vec<String> {
    let Some(s) = raw.map(str::trim).filter(|t| !t.is_empty()) else {
        return Vec::new();
    };
    s.trim_matches(',')
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect()
}

pub fn role_in_roles_csv(raw: Option<&str>, role_id: &str) -> bool {
    let needle = format!(",{},", role_id.trim());
    let hay = format!(",{},", parse_roles_csv(raw).join(","));
    hay.contains(&needle)
}

pub fn can_user_see_all_tasks(privileges: &[String]) -> bool {
    has_privilege(privileges, "*")
}

pub fn row_visible_to_role(
    row: &Map<String, Value>,
    role_id: &str,
    viewer_user_id: Option<&str>,
    see_all: bool,
) -> bool {
    if see_all {
        return true;
    }
    if let Some(target) = row
        .get(super::validation::COL_UTILISATEUR_CIBLE)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return viewer_user_id.is_some_and(|uid| uid == target);
    }
    let vis = row
        .get(COL_VISIBILITE)
        .and_then(|v| v.as_str())
        .unwrap_or(VIS_PUBLIQUE);
    match vis {
        VIS_PUBLIQUE => true,
        VIS_PRIVEE => row
            .get("role_signataire")
            .or_else(|| row.get("role_validateur"))
            .and_then(|v| v.as_str())
            .map(|r| r.trim() == role_id.trim())
            .unwrap_or(false),
        VIS_PERSONNALISEE => {
            let raw = row.get(COL_ROLES_VISIBLES).and_then(|v| v.as_str());
            role_in_roles_csv(raw, role_id)
        }
        _ => true,
    }
}

/// Clause SQL `AND (...)` pour filtrer les tâches visibles par un rôle / utilisateur.
pub fn sql_visibility_filter(role_id: &str, viewer_user_id: Option<&str>) -> String {
    let r = role_id.replace('\'', "''");
    let user_target = viewer_user_id
        .map(|uid| uid.replace('\'', "''"))
        .unwrap_or_default();
    let user_match = if user_target.is_empty() {
        "0".to_string()
    } else {
        format!(
            "(COALESCE({COL_UTILISATEUR_CIBLE}, '') != '' AND {COL_UTILISATEUR_CIBLE} = '{user_target}')"
        )
    };
    format!(
        " AND (
            {user_match}
            OR (
                COALESCE({COL_UTILISATEUR_CIBLE}, '') = ''
                AND (
                    COALESCE({COL_VISIBILITE}, '{VIS_PUBLIQUE}') = '{VIS_PUBLIQUE}'
                    OR ({COL_VISIBILITE} = '{VIS_PRIVEE}' AND (role_signataire = '{r}' OR role_validateur = '{r}'))
                    OR ({COL_VISIBILITE} = '{VIS_PERSONNALISEE}'
                        AND instr(',' || COALESCE({COL_ROLES_VISIBLES}, '') || ',', ',{r},') > 0)
                )
            )
        )"
    )
}

const COL_UTILISATEUR_CIBLE: &str = super::validation::COL_UTILISATEUR_CIBLE;

pub fn apply_create_defaults(data: &mut Map<String, Value>) {
    if !data.contains_key(COL_VISIBILITE) || data.get(COL_VISIBILITE).map(|v| v.is_null()).unwrap_or(true) {
        data.insert(COL_VISIBILITE.into(), Value::String(VIS_PUBLIQUE.into()));
    }
    if let Some(Value::String(vis)) = data.get(COL_VISIBILITE) {
        if vis == VIS_PERSONNALISEE {
            if let Some(Value::Array(arr)) = data.get("roles_visibles_list") {
                let ids: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                data.insert(COL_ROLES_VISIBLES.into(), Value::String(encode_roles_csv(&ids)));
            }
        } else {
            data.insert(COL_ROLES_VISIBLES.into(), Value::Null);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_csv_roundtrip() {
        let csv = encode_roles_csv(&["role-b".into(), "role-a".into()]);
        assert_eq!(csv, ",role-a,role-b,");
        assert!(role_in_roles_csv(Some(&csv), "role-a"));
        assert!(!role_in_roles_csv(Some(&csv), "role-x"));
    }
}
