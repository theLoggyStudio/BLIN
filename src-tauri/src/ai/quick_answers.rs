//! Réponses instantanées sans appel au modèle (date, heure, calculs simples).

use chrono::{Datelike, Local};

use super::intent_filters::normalize_message;

/// Réponse immédiate si la question est simple et ne nécessite pas le LLM.
pub fn try_quick_answer(message: &str) -> Option<String> {
    if let Some(reply) = try_math_answer(message) {
        return Some(reply);
    }

    let n = normalize_message(message);
    if n.is_empty() {
        return None;
    }

    if wants_datetime(&n) {
        return Some(format_datetime_reply());
    }

    None
}

/// Calcul arithmétique simple (ex. `1+1`, `22-56`, `3*4`) — texte brut, pas `normalize_message`.
fn try_math_answer(raw: &str) -> Option<String> {
    let expr = extract_arithmetic_expr(raw)?;
    let value = eval_arithmetic(&expr)?;
    Some(format_math_reply(&expr, value))
}

fn extract_arithmetic_expr(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return None;
    }
    while s.ends_with(|c: char| c == '?' || c == '!' || c == '.' || c == ',') {
        s.pop();
    }
    let s = s.trim();
    if s.is_empty() || s.len() > 48 {
        return None;
    }
    let mut has_digit = false;
    let mut has_op = false;
    for c in s.chars() {
        match c {
            '0'..='9' | '.' => has_digit = true,
            '+' | '-' | '*' | '/' | 'x' | 'X' | '×' | '÷' => has_op = true,
            ' ' | '(' | ')' => {}
            _ => return None,
        }
    }
    if !has_digit || !has_op {
        return None;
    }
    let normalized: String = s
        .chars()
        .map(|c| match c {
            '×' | 'x' | 'X' => '*',
            '÷' => '/',
            c => c,
        })
        .collect();
    Some(normalized.replace(' ', ""))
}

fn eval_arithmetic(expr: &str) -> Option<f64> {
    if expr.is_empty() {
        return None;
    }
    let chars: Vec<char> = expr.chars().collect();
    let mut pos = 0;
    let v = parse_add_sub(&chars, &mut pos)?;
    skip_ws(&chars, &mut pos);
    if pos != chars.len() {
        return None;
    }
    if !v.is_finite() {
        return None;
    }
    Some(v)
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_add_sub(chars: &[char], pos: &mut usize) -> Option<f64> {
    let mut acc = parse_mul_div(chars, pos)?;
    loop {
        skip_ws(chars, pos);
        if *pos >= chars.len() {
            break;
        }
        let op = chars[*pos];
        if op != '+' && op != '-' {
            break;
        }
        *pos += 1;
        let rhs = parse_mul_div(chars, pos)?;
        acc = if op == '+' { acc + rhs } else { acc - rhs };
    }
    Some(acc)
}

fn parse_mul_div(chars: &[char], pos: &mut usize) -> Option<f64> {
    let mut acc = parse_unary(chars, pos)?;
    loop {
        skip_ws(chars, pos);
        if *pos >= chars.len() {
            break;
        }
        let op = chars[*pos];
        if op != '*' && op != '/' {
            break;
        }
        *pos += 1;
        let rhs = parse_unary(chars, pos)?;
        if op == '/' && rhs == 0.0 {
            return None;
        }
        acc = if op == '*' { acc * rhs } else { acc / rhs };
    }
    Some(acc)
}

fn parse_unary(chars: &[char], pos: &mut usize) -> Option<f64> {
    skip_ws(chars, pos);
    let mut sign = 1.0;
    while *pos < chars.len() && (chars[*pos] == '+' || chars[*pos] == '-') {
        if chars[*pos] == '-' {
            sign *= -1.0;
        }
        *pos += 1;
        skip_ws(chars, pos);
    }
    parse_atom(chars, pos).map(|v| sign * v)
}

fn parse_atom(chars: &[char], pos: &mut usize) -> Option<f64> {
    skip_ws(chars, pos);
    if *pos < chars.len() && chars[*pos] == '(' {
        *pos += 1;
        let v = parse_add_sub(chars, pos)?;
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != ')' {
            return None;
        }
        *pos += 1;
        return Some(v);
    }
    let start = *pos;
    while *pos < chars.len() && (chars[*pos].is_ascii_digit() || chars[*pos] == '.') {
        *pos += 1;
    }
    if start == *pos {
        return None;
    }
    chars[start..*pos].iter().collect::<String>().parse().ok()
}

fn format_math_reply(expr: &str, value: f64) -> String {
    let display = if (value.fract().abs() < f64::EPSILON) || value.fract().abs() > 1.0 - f64::EPSILON {
        format!("{}", value.round() as i64)
    } else {
        let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
        format!("{rounded}")
    };
    format!("{expr} = {display}")
}

fn wants_datetime(n: &str) -> bool {
    const KW: &[&str] = &[
        "quelle heure",
        "quelle est l heure",
        "donne l heure",
        "donne moi l heure",
        "donne moi l heure",
        "peux tu me donner l heure",
        "peut tu me donner l heure",
        "il est quelle heure",
        "heure actuelle",
        "heure maintenant",
        "quelle date",
        "quelle est la date",
        "date du jour",
        "date aujourd hui",
        "aujourd hui on est",
        "on est quel jour",
        "quel jour sommes nous",
        "quel jour on est",
    ];
    if KW.iter().any(|k| n.contains(k)) {
        return true;
    }
    (n.contains("heure") || n.contains("date") || n.contains("jour"))
        && (n.contains("quelle") || n.contains("donne") || n.contains("maintenant") || n.contains("actuel"))
}

fn format_datetime_reply() -> String {
    let now = Local::now();
    const WEEKDAYS: [&str; 7] = [
        "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
    ];
    const MONTHS: [&str; 12] = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    let wd = WEEKDAYS[now.weekday().num_days_from_monday() as usize];
    let month = MONTHS[(now.month() - 1) as usize];
    let time = now.format("%H:%M").to_string();
    format!(
        "Il est {time} (heure locale de votre ordinateur). Nous sommes le {wd} {} {month} {}.",
        now.day(),
        now.year()
    )
}
