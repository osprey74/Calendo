/// RFC 3986 unreserved-set percent encoding for URL path/query segments.
///
/// Used for embedding calendar IDs (which may contain `@`, `=`, `+`, `/`, etc.) into
/// API URLs. Microsoft Graph and Google Calendar both accept properly percent-encoded
/// segments here — encoding is the safe default rather than relying on the upstream
/// to never produce reserved characters.
pub fn percent_encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if safe {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}
