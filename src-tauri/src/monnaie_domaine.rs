/// Devises ISO 4217 reconnues.
pub const DEVISES: &[&str] = &[
    "EUR", "USD", "GBP", "CHF", "CAD", "XOF", "XAF", "MAD", "TND", "DZD",
];

/// Domaines métier.
pub const DOMAINES: &[&str] = &["habitation", "commercial", "foncier", "mixte"];

pub fn normalize_devise(code: &str) -> String {
    let c = code.trim().to_uppercase();
    if DEVISES.contains(&c.as_str()) {
        c
    } else {
        "EUR".to_string()
    }
}

pub fn normalize_domaine(code: &str, type_bien: &str) -> String {
    let c = code.trim();
    let allowed: &[&str] = match type_bien {
        "appartement" | "maison" => &["habitation", "mixte"],
        "immeuble" => &["habitation", "commercial", "mixte"],
        "local_commercial" | "hangar" => &["commercial", "mixte"],
        "terrain" => &["foncier", "commercial"],
        _ => &["habitation", "commercial", "foncier", "mixte"],
    };
    if allowed.contains(&c) {
        c.to_string()
    } else {
        allowed[0].to_string()
    }
}

pub fn default_domaine_for_type(type_bien: &str) -> &'static str {
    match type_bien {
        "local_commercial" | "hangar" => "commercial",
        "terrain" => "foncier",
        "immeuble" => "habitation",
        _ => "habitation",
    }
}
