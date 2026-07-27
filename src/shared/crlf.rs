//! Shared CRLF line-ending helper.

/// Rewrites bare line feeds to CRLF, idempotently: a `\n` already
/// preceded by `\r` is left untouched, every other `\n` gets a `\r`
/// inserted before it. Content read from a Unix-LF source or passed
/// inline with real newlines is thus made RFC 5322 / IMAP `APPEND`
/// compliant without corrupting content that is already CRLF.
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev = '\0';

    for ch in input.chars() {
        if ch == '\n' && prev != '\r' {
            out.push('\r');
        }
        out.push(ch);
        prev = ch;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn bare_lf_gains_cr() {
        assert_eq!(normalize("a\nb\n"), "a\r\nb\r\n");
    }

    #[test]
    fn existing_crlf_is_untouched() {
        assert_eq!(normalize("a\r\nb\r\n"), "a\r\nb\r\n");
    }

    #[test]
    fn mixed_endings_converge_to_crlf() {
        assert_eq!(normalize("a\r\nb\nc"), "a\r\nb\r\nc");
    }

    #[test]
    fn lone_cr_is_preserved() {
        assert_eq!(normalize("a\rb"), "a\rb");
    }
}
