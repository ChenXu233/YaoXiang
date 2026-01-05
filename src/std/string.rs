//! Standard String library

/// Get string length
pub fn string_len(s: &str) -> usize {
    s.len()
}

/// Get string length in characters
pub fn string_char_len(s: &str) -> usize {
    s.chars().count()
}

/// Check if string is empty
pub fn string_is_empty(s: &str) -> bool {
    s.is_empty()
}

/// Get substring
pub fn string_slice(
    s: &str,
    start: usize,
    end: usize,
) -> Option<&str> {
    s.get(start..end)
}

/// Get character at index
pub fn string_char_at(
    s: &str,
    index: usize,
) -> Option<char> {
    s.chars().nth(index)
}

/// Convert to uppercase
pub fn string_uppercase(s: &str) -> String {
    s.to_uppercase()
}

/// Convert to lowercase
pub fn string_lowercase(s: &str) -> String {
    s.to_lowercase()
}

/// Trim whitespace
pub fn string_trim(s: &str) -> String {
    s.trim().to_string()
}

/// Check if string starts with prefix
pub fn string_starts_with(
    s: &str,
    prefix: &str,
) -> bool {
    s.starts_with(prefix)
}

/// Check if string ends with suffix
pub fn string_ends_with(
    s: &str,
    suffix: &str,
) -> bool {
    s.ends_with(suffix)
}

/// Find substring
pub fn string_find(
    s: &str,
    sub: &str,
) -> Option<usize> {
    s.find(sub)
}

/// Replace substring
pub fn string_replace(
    s: &str,
    from: &str,
    to: &str,
) -> String {
    s.replace(from, to)
}

/// Split string
pub fn string_split(
    s: &str,
    sep: &str,
) -> Vec<String> {
    s.split(sep).map(|s| s.to_string()).collect()
}

/// Join strings
pub fn string_join(
    parts: &[String],
    sep: &str,
) -> String {
    parts.join(sep)
}
