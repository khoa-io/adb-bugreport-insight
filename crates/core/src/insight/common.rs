//! Shared parsing, measurement, and evidence primitives.
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone};
use regex_automata::meta::Regex;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

pub type Clock = DateTime<FixedOffset>;
type PatternCache = Mutex<BTreeMap<&'static str, Option<Arc<Regex>>>>;

/// Static patterns are cached; captures retain source spelling.
pub fn captures(pattern: &'static str, text: &str) -> Option<Vec<String>> {
    static CACHE: OnceLock<PatternCache> = OnceLock::new();
    let re = {
        let mut cache = CACHE.get_or_init(Mutex::default).lock().ok()?;
        cache
            .entry(pattern)
            .or_insert_with(|| Regex::new(pattern).ok().map(Arc::new))
            .clone()?
    };
    let mut caps = re.create_captures();
    re.captures(text, &mut caps);
    caps.is_match().then(|| {
        (0..caps.group_len())
            .map(|i| {
                caps.get_group(i)
                    .map(|m| text[m.start..m.end].to_string())
                    .unwrap_or_default()
            })
            .collect()
    })
}

pub fn object(keys: &str) -> Value {
    Value::Object(
        keys.split_whitespace()
            .map(|k| (k.to_string(), Value::Null))
            .collect(),
    )
}
pub fn number(s: &str) -> Option<i64> {
    s.trim()
        .trim_end_matches(['K', 'k', ','])
        .replace(',', "")
        .parse()
        .ok()
}
pub fn decimal(s: &str) -> Option<f64> {
    s.trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}
pub fn normal(s: &str) -> Option<String> {
    let s = s.trim().trim_matches('\'');
    (!s.is_empty() && !["(unknown)", "<unknown>", "unknown", "NULL", "null"].contains(&s))
        .then(|| s.to_string())
}
pub fn token<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let tail = text.split_once(key)?.1.trim_start();
    Some(
        tail.split_whitespace()
            .next()?
            .trim_end_matches([',', '}', ']']),
    )
}
pub fn token_num(text: &str, key: &str) -> Option<i64> {
    number(token(text, key)?)
}
pub fn token_bool(text: &str, key: &str) -> Option<bool> {
    token(text, key)?.parse().ok()
}
pub fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    Some(text.split_once(start)?.1.split_once(end)?.0)
}
pub fn words(text: &str) -> Vec<String> {
    text.split([' ', ',', '|'])
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
pub fn round(value: f64, precision: i32) -> f64 {
    let factor = 10f64.powi(precision);
    (value * factor).round() / factor
}
pub fn ratio(n: Option<f64>, d: Option<f64>) -> Option<f64> {
    let d = d?;
    let n = n?;
    (d > 0.).then(|| round(n / d, 4))
}
pub fn stamp(t: &Clock) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
}
pub fn parse_clock(text: &str, offset: Option<FixedOffset>) -> Option<Clock> {
    let text = text.trim().trim_matches(['\'', '"']);
    if let Ok(t) = DateTime::parse_from_rfc3339(&text.replace(' ', "T")) {
        return Some(t);
    }
    for fmt in [
        "%Y-%m-%d %H:%M:%S%.f%z",
        "%Y-%m-%d %H:%M:%S%.f %z",
        "%Y-%m-%d %H:%M:%S%.f%:z",
    ] {
        if let Ok(t) = DateTime::parse_from_str(text, fmt) {
            return Some(t);
        }
    }
    for fmt in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d-%H-%M-%S-%3f",
        "%Y-%m-%d-%H-%M-%S",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(t) = NaiveDateTime::parse_from_str(text, fmt) {
            return offset?.from_local_datetime(&t).single();
        }
    }
    None
}
pub fn log_clock(raw: &str, capture: Option<&Clock>) -> Option<Clock> {
    use chrono::Datelike;
    let capture = capture?;
    let mut year = capture.year();
    let candidate =
        NaiveDate::parse_from_str(&format!("{year}-{}", raw.get(..5)?), "%Y-%m-%d").ok()?;
    if candidate
        .signed_duration_since(capture.date_naive())
        .num_days()
        > 1
    {
        year -= 1;
    }
    parse_clock(&format!("{year}-{raw}"), Some(*capture.offset()))
}
pub fn uid(raw: &str) -> Option<i64> {
    if let Ok(v) = raw.parse() {
        return Some(v);
    }
    if let Some(c) = captures(r"^u(\d+)([ais])(\d+)$", raw) {
        let base = match c[2].as_str() {
            "a" => 10000,
            "i" => 99000,
            _ => return None,
        };
        return c[1]
            .parse::<i64>()
            .ok()?
            .checked_mul(100000)?
            .checked_add(base)?
            .checked_add(c[3].parse::<i64>().ok()?);
    }
    match raw {
        "root" => Some(0),
        "system" => Some(1000),
        "radio" => Some(1001),
        "bluetooth" => Some(1002),
        "graphics" => Some(1003),
        "input" => Some(1004),
        "audio" => Some(1005),
        "camera" => Some(1006),
        "log" => Some(1007),
        "wifi" => Some(1010),
        "media" => Some(1013),
        "dhcp" => Some(1014),
        "gps" => Some(1021),
        "logd" => Some(1036),
        "lmkd" => Some(1069),
        "shell" => Some(2000),
        _ => None,
    }
}

/// Keep excerpts byte-bounded without splitting UTF-8 or silently losing NULs.
pub fn excerpt(text: &str) -> (String, bool, bool) {
    let mut end = text.len().min(512);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let truncated = end < text.len();
    let end = if truncated && end < 480 { 0 } else { end };
    (
        text[..end].to_string(),
        truncated,
        !truncated && !text.contains('\0'),
    )
}
pub fn evidence(path: &str, line: u64, offset: u64, text: &str, full_len: u64) -> Value {
    let (excerpt, truncated, complete) = excerpt(text);
    json!({"archive_path":path,"kind":"line_range","locator":{"line_start":line,"line_end":line,"byte_offset":offset,"byte_length":full_len},"excerpt_bytes":excerpt.len(),"excerpt":excerpt,"excerpt_truncated":truncated,"excerpt_complete":complete && full_len==text.len() as u64})
}
pub fn gap(field: &str, reason: &str, detail: &str) -> Value {
    json!({"field":field,"reason":reason,"detail":detail})
}
pub fn merge(dst: &mut Value, src: Value) {
    if let (Some(dst), Value::Object(src)) = (dst.as_object_mut(), src) {
        dst.extend(src);
    }
}
pub fn integer_map(keys: &str) -> Value {
    Value::Object(
        keys.split_whitespace()
            .map(|key| (key.to_string(), json!(0)))
            .collect::<Map<_, _>>(),
    )
}
pub fn hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(text.as_bytes()))[..16].to_string()
}
