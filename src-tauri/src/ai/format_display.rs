//! Mise en forme lisible des données pour l'utilisateur final (pas de JSON brut).

use serde_json::{Map, Value};

const MAX_TABLE_ROWS: usize = 50;
const MAX_CELL_LEN: usize = 48;

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{cut}…")
}

fn cell_str(v: &Value) -> String {
    match v {
        Value::Null => "—".into(),
        Value::Bool(b) => if *b { "Oui" } else { "Non" }.into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => truncate(s.trim(), MAX_CELL_LEN),
        Value::Array(a) if a.is_empty() => "—".into(),
        Value::Array(a) => truncate(
            &a.iter()
                .map(cell_str)
                .collect::<Vec<_>>()
                .join(", "),
            MAX_CELL_LEN,
        ),
        Value::Object(_) => truncate(&v.to_string(), MAX_CELL_LEN),
    }
}

fn object_label(obj: &Map<String, Value>) -> String {
    for key in ["label", "nom", "name", "reference", "titre", "title", "id"] {
        if let Some(v) = obj.get(key).and_then(|x| x.as_str()) {
            let t = v.trim();
            if !t.is_empty() {
                return truncate(t, MAX_CELL_LEN);
            }
        }
    }
    obj.iter()
        .find_map(|(_, v)| v.as_str().map(|s| truncate(s.trim(), MAX_CELL_LEN)))
        .unwrap_or_else(|| "—".to_string())
}

fn column_keys(objects: &[&Map<String, Value>]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for obj in objects {
        for k in obj.keys() {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
    }
    if keys.len() > 8 {
        keys.truncate(8);
    }
    keys
}

fn format_array_table(arr: &[Value]) -> String {
    let objects: Vec<&Map<String, Value>> = arr
        .iter()
        .filter_map(|v| v.as_object())
        .take(MAX_TABLE_ROWS)
        .collect();
    if objects.is_empty() {
        return format_array_list(arr);
    }
    let keys = column_keys(&objects);
    if keys.is_empty() {
        return format_array_list(arr);
    }
    let mut lines = vec![keys.join(" | ")];
    lines.push(keys.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
    for obj in &objects {
        let row: Vec<String> = keys
            .iter()
            .map(|k| cell_str(obj.get(k).unwrap_or(&Value::Null)))
            .collect();
        lines.push(row.join(" | "));
    }
    if arr.len() > MAX_TABLE_ROWS {
        lines.push(format!("… ({}/{} lignes affichées)", MAX_TABLE_ROWS, arr.len()));
    }
    lines.join("\n")
}

fn format_array_list(arr: &[Value]) -> String {
    arr.iter()
        .take(MAX_TABLE_ROWS)
        .map(|v| {
            if let Some(obj) = v.as_object() {
                format!("• {}", object_label(obj))
            } else {
                format!("• {}", cell_str(v))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_object(obj: &Map<String, Value>) -> String {
    obj.iter()
        .map(|(k, v)| format!("• {k} : {}", cell_str(v)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Formate une valeur JSON en texte lisible (liste ou tableau).
pub fn format_value_for_user(value: &Value) -> String {
    match value {
        Value::Array(arr) if arr.is_empty() => "Aucun résultat.".into(),
        Value::Array(arr) if arr.iter().all(|v| v.is_object()) => format_array_table(arr),
        Value::Array(arr) => format_array_list(arr),
        Value::Object(obj) => format_object(obj),
        other => cell_str(other),
    }
}

/// Retire les blocs ```json``` et les remplace par du texte lisible si possible.
pub fn replace_json_fences(text: &str) -> String {
    let mut out = text.to_string();
    while let Some(start) = out.find("```json") {
        let after = start + 7;
        let rest = &out[after..];
        let end_rel = rest.find("```").unwrap_or(rest.len());
        let json_raw = rest[..end_rel].trim();
        let replacement = serde_json::from_str::<Value>(json_raw)
            .map(|v| format!("\n{}\n", format_value_for_user(&v)))
            .unwrap_or_default();
        let end = after + end_rel + 3;
        out.replace_range(start..end.min(out.len()), &replacement);
    }
    out
}

/// Retire une éventuelle ligne d'appel d'outil JSON en tête ou fin de message.
pub fn strip_tool_call_line(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with('{')
        && trimmed.ends_with('}')
        && trimmed.contains("\"tool\"")
    {
        return String::new();
    }
    text.lines()
        .filter(|line| {
            let t = line.trim();
            !(t.starts_with('{') && t.contains("\"tool\"") && t.ends_with('}'))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Nettoie un message assistant avant affichage utilisateur.
pub fn sanitize_assistant_message(text: &str) -> String {
    let mut s = strip_tool_call_line(text);
    s = replace_json_fences(&s);
    s.trim().to_string()
}
