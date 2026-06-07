pub fn escape_field(value: &str, delimiter: char) -> String {
    if value.contains(['"', delimiter, '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn push_row(writer: &mut String, fields: &[String]) {
    push_row_delim(writer, fields, ',');
}

pub fn push_row_semicolon(writer: &mut String, fields: &[String]) {
    push_row_delim(writer, fields, ';');
}

fn push_row_delim(writer: &mut String, fields: &[String], delimiter: char) {
    let line = fields
        .iter()
        .map(|f| escape_field(f, delimiter))
        .collect::<Vec<_>>()
        .join(delimiter.to_string().as_str());
    writer.push_str(&line);
    writer.push('\n');
}

pub fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}
