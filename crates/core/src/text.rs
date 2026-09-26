/// Caps an excerpt at 400 characters so evidence stays bounded regardless of source-line length.
pub fn bounded(value: &str) -> String {
    value.chars().take(400).collect()
}

/// Escapes Markdown-significant characters pulled verbatim from bugreport text.
pub fn markdown_escape(value: &str) -> String {
    value.replace('`', "\\`").replace('\n', " ")
}

pub fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|head| head.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}
