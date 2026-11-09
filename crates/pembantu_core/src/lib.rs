pub mod agent;
pub mod error;
pub mod llm;
pub mod prompt;
pub mod provider;

/// Returns at most `max_chars` characters of `s`, cutting on a char boundary.
pub fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
