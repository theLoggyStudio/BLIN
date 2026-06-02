/// Privilèges du module immobilier retiré (biens, finances, hangars, contrats).
pub fn is_legacy_immo_privilege(privilege: &str) -> bool {
    privilege.starts_with("biens:")
        || privilege.starts_with("finances:")
        || privilege.starts_with("hangars:")
        || privilege.starts_with("contrats:")
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

pub fn require_privilege(user_privileges: &[String], required: &str) -> Result<(), String> {
    if has_privilege(user_privileges, required) {
        Ok(())
    } else {
        Err(format!("Privilège requis : {required}"))
    }
}
