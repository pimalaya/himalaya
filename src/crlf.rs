//! # CRLF
//!
//! The line-ending normalisation every raw input goes through.

/// Rewrites bare line feeds to CRLF, idempotently.
///
/// A `\n` after `\r` is left alone, so compliant RFC 5322 content is not
/// corrupted. Works on bytes, as a raw message may carry 8-bit content in
/// any charset.
pub fn normalize(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut prev = 0;

    for &byte in input {
        if byte == b'\n' && prev != b'\r' {
            out.push(b'\r');
        }
        out.push(byte);
        prev = byte;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn bare_lf_gains_cr() {
        assert_eq!(normalize(b"a\nb\n"), b"a\r\nb\r\n");
    }

    #[test]
    fn existing_crlf_is_untouched() {
        assert_eq!(normalize(b"a\r\nb\r\n"), b"a\r\nb\r\n");
    }

    #[test]
    fn mixed_endings_converge_to_crlf() {
        assert_eq!(normalize(b"a\r\nb\nc"), b"a\r\nb\r\nc");
    }

    #[test]
    fn lone_cr_is_preserved() {
        assert_eq!(normalize(b"a\rb"), b"a\rb");
    }

    #[test]
    fn non_utf8_bytes_are_kept() {
        assert_eq!(normalize(b"a\xff\nb"), b"a\xff\r\nb");
    }
}
