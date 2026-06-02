pub fn escape_field(value: &str) -> String {
    if value.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn push_row(writer: &mut String, fields: &[String]) {
    let line = fields.iter().map(|f| escape_field(f)).collect::<Vec<_>>().join(",");
    writer.push_str(&line);
    writer.push('\n');
}

pub fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}
