//! # CRLF
//!
//! The line-ending normalisation every raw input goes through.

/// Rewrites bare line feeds to CRLF, idempotently.
///
/// A `\n` already preceded by `\r` is left alone, so content read from a
/// Unix source becomes RFC 5322 compliant without corrupting content that
/// already is.
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
