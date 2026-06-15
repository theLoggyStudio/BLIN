/// Privilèges applicatifs (hors entités DDA) — toujours proposés dans le catalogue rôles.
pub fn system_privileges() -> &'static [&'static str] {
    &[
        "*",
        "ai:utiliser",
        "directeur:confirmer",
        "users:voir",
        "users:modifier",
        "parametres:assistant",
        "parametres:compte",
        "parametres:theme",
        "parametres:impression",
        "parametres:entites",
        "parametres:entites:creer",
        "parametres:roles",
        "parametres:utilisateurs",
        "parametres:voir",
        "documents:voir",
        "documents:importer",
        "documents:exporter",
        "documents:supprimer",
        "documents:modeles_voir",
        "documents:modeles_gerer",
    ]
}

/// Privilèges de visibilité des panneaux Paramètres (un par section repliable).
pub fn parametres_panel_privileges() -> &'static [&'static str] {
    &[
        "parametres:voir",
        "parametres:assistant",
        "parametres:compte",
        "parametres:theme",
        "parametres:impression",
        "parametres:entites",
        "parametres:entites:creer",
        "parametres:roles",
        "parametres:utilisateurs",
    ]
}

/// Privilèges Paramètres accordés par défaut au rôle Directeur.
pub fn default_directeur_parametres_privileges() -> &'static [&'static str] {
    &[
        "parametres:assistant",
        "parametres:compte",
        "parametres:theme",
        "parametres:impression",
        "parametres:entites",
        "parametres:entites:creer",
        "parametres:roles",
        "parametres:utilisateurs",
        "parametres:voir",
    ]
}

/// Privilèges du module immobilier retiré (biens, finances, hangars, contrats).
pub fn is_legacy_immo_privilege(privilege: &str) -> bool {
    privilege.starts_with("biens:")
        || privilege.starts_with("finances:")
        || privilege.starts_with("hangars:")
        || privilege.starts_with("contrats:")
}

/// Au moins un privilège lié à l'entité (`{entité}:voir|creer|…` ou `{entité}:*`).
pub fn has_any_entity_privilege(user_privileges: &[String], entity_key: &str) -> bool {
    if user_privileges.iter().any(|p| p == "*") {
        return true;
    }
    let wildcard = format!("{entity_key}:*");
    if user_privileges.iter().any(|p| p == &wildcard) {
        return true;
    }
    let prefix = format!("{entity_key}:");
    user_privileges
        .iter()
        .any(|p| p.starts_with(&prefix))
}

pub fn has_privilege(user_privileges: &[String], required: &str) -> bool {
    if user_privileges.iter().any(|p| p == "*") {
        return true;
    }
    if user_privileges.iter().any(|p| p == required) {
        return true;
    }
    if let Some((module, _)) = required.split_once(':') {
        let wildcard = format!("{module}:*");
        if user_privileges.iter().any(|p| p == &wildcard) {
            return true;
        }
    }
    false
}

/// Création d'une définition d'entité via Loggy (ou édition complète du registre).
pub fn can_create_registry_entity(user_privileges: &[String]) -> bool {
    has_privilege(user_privileges, "parametres:entites")
        || has_privilege(user_privileges, "parametres:entites:creer")
}

pub fn require_privilege(user_privileges: &[String], required: &str) -> Result<(), String> {
    if has_privilege(user_privileges, required) {
        Ok(())
    } else {
        Err(format!("Privilège requis : {required}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_entity_privilege_accepts_voir_creer_or_wildcard() {
        assert!(has_any_entity_privilege(&["clients:voir".into()], "clients"));
        assert!(has_any_entity_privilege(&["clients:creer".into()], "clients"));
        assert!(has_any_entity_privilege(&["clients:*".into()], "clients"));
        assert!(has_any_entity_privilege(&["*".into()], "clients"));
        assert!(!has_any_entity_privilege(&["stock:voir".into()], "clients"));
    }
}
