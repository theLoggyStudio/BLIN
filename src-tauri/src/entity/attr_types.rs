use super::registry::EntityAttribute;

/// Attribut système déjà géré par la table (`id` UUID généré par DDA).
pub fn is_reserved_attribute(attr: &EntityAttribute) -> bool {
    let col = super::schema::attr_column(attr);
    (col == "id" || attr.nom == "id")
        && matches!(
            attr.attr_type.as_str(),
            "uuid" | "hidden" | "pk" | "primary_key"
        )
}

/// Parse `enum[a,b,c]` → options ; autres types inchangés.
pub fn parse_enum_options(attr_type: &str) -> Option<Vec<String>> {
    let t = attr_type.trim();
    let inner = t.strip_prefix("enum[")?;
    let end = inner.rfind(']')?;
    let body = inner[..end].trim();
    if body.is_empty() {
        return None;
    }
    let opts: Vec<String> = body
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if opts.is_empty() {
        None
    } else {
        Some(opts)
    }
}

pub fn normalize_attr_type(raw: &str) -> String {
    let t = raw.trim().to_lowercase();
    if parse_enum_options(&t).is_some() || t.starts_with("enum[") {
        return "enum".into();
    }
    match t.as_str() {
        "uuid" => "uuid".into(),
        "bool" => "boolean".into(),
        "int" => "integer".into(),
        "float" | "decimal" => "float".into(),
        "photo" | "image" | "avatar" => "photo".into(),
        "mail" => "email".into(),
        "heure" => "time".into(),
        "stock" | "quantite_stock" => "stock".into(),
        "compteur" | "counter" | "compteur_auto" => "compteur".into(),
        _ => t,
    }
}

pub fn normalize_attribute(attr: &mut EntityAttribute) {
    if let Some(opts) = parse_enum_options(&attr.attr_type) {
        attr.attr_type = "enum".into();
        attr.enum_options = Some(opts);
    } else {
        attr.attr_type = normalize_attr_type(&attr.attr_type);
    }
    if let Some(ref r) = attr.r#ref {
        if r.trim().is_empty() {
            attr.r#ref = None;
        }
    }
}
