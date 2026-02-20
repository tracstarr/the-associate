/// Truncate a string to at most `max_chars` Unicode scalar values.
/// Returns a borrowed slice if possible; no allocation when not truncated.
pub fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
