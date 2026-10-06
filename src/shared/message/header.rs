//! # Header decoding
//!
//! Reads a header value from the raw bytes of a message rather than from
//! mail-parser's decoded string, so that what mail goes out with in
//! practice reads right:
//!
//! - raw 8-bit bytes are UTF-8 when they are valid UTF-8 (RFC 6532) and
//!   Windows-1252 otherwise;
//! - adjacent encoded words of one charset are joined as bytes before
//!   their charset is decoded, so a character split across two words is
//!   rebuilt, and the space between adjacent encoded words is dropped
//!   (RFC 2047 §6.2).

use mail_parser::{
    Header, Message, decoders::base64::base64_decode, decoders::charsets::map::charset_decoder,
};

/// The raw value of the first header of that name among `headers`, its
/// folding and trailing line break removed, raw 8-bit bytes decoded.
pub fn raw_text(message: &Message, headers: &[Header], name: &str) -> Option<String> {
    let header = headers
        .iter()
        .find(|header| header.name.as_str().eq_ignore_ascii_case(name))?;
    let bytes = message
        .raw_message
        .get(header.offset_start as usize..header.offset_end as usize)?;
    Some(eight_bit(bytes).replace(['\r', '\n'], ""))
}

/// The value of an unstructured header, encoded words decoded and
/// trimmed; `None` when absent or empty.
pub fn unstructured(message: &Message, headers: &[Header], name: &str) -> Option<String> {
    let text = decode_words(&raw_text(message, headers, name)?);
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// Header bytes as text: UTF-8 when they are valid UTF-8, Windows-1252
/// otherwise.
pub fn eight_bit(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => decode_charset("windows-1252", bytes),
    }
}

/// Bytes decoded by a charset label, lossily, UTF-8 when the label is
/// unknown.
pub fn decode_charset(label: &str, bytes: &[u8]) -> String {
    let label = label.trim().trim_matches('"').to_ascii_lowercase();

    // NOTE: mail-parser has no UTF-8 entry in its table, its own reading
    // being the fallback.
    if label == "utf-8" || label == "utf8" {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    match charset_decoder(label.as_bytes()) {
        Some(decode) => decode(bytes),
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// The value of a parameter written plainly (`name=value` or
/// `name="value"`, not the RFC 2231 forms) in a `Content-Type` or
/// `Content-Disposition` value, unquoted; `None` when absent or when an
/// RFC 2231 form of it is there too.
pub fn plain_param(value: &str, name: &str) -> Option<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut escaped) = (false, false);

    for c in value.chars() {
        match (quoted, escaped, c) {
            (true, true, _) => escaped = false,
            (true, false, '\\') => escaped = true,
            (_, false, '"') => quoted = !quoted,
            (false, _, ';') => {
                segments.push(std::mem::take(&mut current));
                continue;
            }
            _ => (),
        }
        current.push(c);
    }
    segments.push(current);

    // NOTE: an RFC 2231 form of the parameter wins over the plain one.
    let extended = format!("{name}*");
    let keys = segments
        .iter()
        .skip(1)
        .filter_map(|segment| segment.split_once('='));
    if keys
        .map(|(key, _)| key.trim().to_ascii_lowercase())
        .any(|key| key.starts_with(&extended.to_ascii_lowercase()))
    {
        return None;
    }

    segments.iter().skip(1).find_map(|segment| {
        let (key, value) = segment.split_once('=')?;
        if !key.trim().eq_ignore_ascii_case(name) {
            return None;
        }
        let value = value.trim();
        let Some(quoted) = value.strip_prefix('"') else {
            return Some(value.to_owned());
        };
        let quoted = quoted.strip_suffix('"').unwrap_or(quoted);
        let mut out = String::new();
        let mut escaped = false;
        for c in quoted.chars() {
            match (escaped, c) {
                (false, '\\') => escaped = true,
                _ => {
                    out.push(c);
                    escaped = false;
                }
            }
        }
        Some(out)
    })
}

/// One stretch of a header value: literal text, or the bytes of an
/// encoded word with its charset.
enum Segment {
    Text(String),
    Encoded { charset: String, bytes: Vec<u8> },
}

/// Decodes the RFC 2047 encoded words of a value.
///
/// Whitespace alone between two encoded words is dropped, and adjacent
/// encoded words of one charset are joined as bytes before decoding. A
/// sequence that is not a well-formed encoded word stays as it is.
pub fn decode_words(value: &str) -> String {
    let mut segments: Vec<Segment> = Vec::new();
    let mut rest = value;

    while !rest.is_empty() {
        let Some(start) = rest.find("=?") else {
            segments.push(Segment::Text(rest.to_owned()));
            break;
        };

        match encoded_word(&rest[start..]) {
            Some((charset, bytes, len)) => {
                if start > 0 {
                    segments.push(Segment::Text(rest[..start].to_owned()));
                }
                segments.push(Segment::Encoded { charset, bytes });
                rest = &rest[start + len..];
            }
            None => {
                segments.push(Segment::Text(rest[..start + 2].to_owned()));
                rest = &rest[start + 2..];
            }
        }
    }

    let mut merged: Vec<Segment> = Vec::new();
    let mut segments = segments.into_iter().peekable();

    while let Some(segment) = segments.next() {
        let segment = match segment {
            Segment::Text(text)
                if text.chars().all(|c| c == ' ' || c == '\t')
                    && matches!(merged.last(), Some(Segment::Encoded { .. }))
                    && matches!(segments.peek(), Some(Segment::Encoded { .. })) =>
            {
                continue;
            }
            segment => segment,
        };

        match (merged.last_mut(), segment) {
            (
                Some(Segment::Encoded { charset, bytes }),
                Segment::Encoded {
                    charset: next,
                    bytes: more,
                },
            ) if *charset == next => bytes.extend(more),
            (Some(Segment::Text(text)), Segment::Text(more)) => text.push_str(&more),
            (_, segment) => merged.push(segment),
        }
    }

    merged
        .into_iter()
        .map(|segment| match segment {
            Segment::Text(text) => text,
            Segment::Encoded { charset, bytes } => decode_charset(&charset, &bytes),
        })
        .collect()
}

/// Reads an encoded word `=?charset?B|Q?text?=` at the start of `input`,
/// returning its charset (lowercased, RFC 2231 language dropped), its
/// decoded bytes and its length.
fn encoded_word(input: &str) -> Option<(String, Vec<u8>, usize)> {
    let body = input.strip_prefix("=?")?;
    let (charset, body) = body.split_once('?')?;
    let (encoding, body) = body.split_once('?')?;
    let (text, rest) = body.split_once("?=")?;

    let clean = |s: &str| !s.contains(|c: char| c.is_whitespace() || c == '?');
    if charset.is_empty() || !clean(charset) || !clean(text) {
        return None;
    }

    let bytes = match encoding {
        "B" | "b" => base64_decode(text.as_bytes())?,
        "Q" | "q" => q_decode(text),
        _ => return None,
    };

    let charset = charset.split('*').next()?.to_ascii_lowercase();
    Some((charset, bytes, input.len() - rest.len()))
}

/// Decodes the Q encoding of RFC 2047 §4.2, an escape that is not two
/// hex digits staying as it is.
fn q_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        let escaped = match bytes.get(i + 1..i + 3) {
            Some(&[high, low]) if bytes[i] == b'=' => hex(high).zip(hex(low)),
            _ => None,
        };

        match (escaped, bytes[i]) {
            (Some((high, low)), _) => {
                out.push(high << 4 | low);
                i += 3;
                continue;
            }
            (None, b'_') => out.push(b' '),
            (None, byte) => out.push(byte),
        }
        i += 1;
    }

    out
}

/// The value of one hex digit.
fn hex(digit: u8) -> Option<u8> {
    (digit as char).to_digit(16).map(|value| value as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_words_of_one_charset_rebuild_a_split_character() {
        let subject = "=?utf-8?B?Q2Fmw6kg?= =?utf-8?B?ww==?= =?utf-8?B?qXTDqQ==?= fin \
                       =?iso-8859-1?Q?=E9t=E9?=";
        assert_eq!(
            decode_words(subject),
            "Caf\u{e9} \u{e9}t\u{e9} fin \u{e9}t\u{e9}"
        );
    }

    #[test]
    fn words_of_different_charsets_join_without_their_space() {
        assert_eq!(
            decode_words("=?UTF-8?B?w4l0w6k=?= =?ISO-8859-1?Q?_=E0_Paris?="),
            "\u{c9}t\u{e9} \u{e0} Paris"
        );
    }

    #[test]
    fn text_between_words_is_kept() {
        assert_eq!(
            decode_words("Re: =?utf-8?Q?caf=C3=A9?= au lait"),
            "Re: caf\u{e9} au lait"
        );
    }

    #[test]
    fn a_malformed_word_stays_as_it_is() {
        assert_eq!(decode_words("=?utf-8?X?abc?= =?"), "=?utf-8?X?abc?= =?");
        assert_eq!(decode_words("2 =? 3"), "2 =? 3");
    }

    #[test]
    fn q_keeps_a_truncated_escape() {
        assert_eq!(q_decode("a=4"), b"a=4");
        assert_eq!(q_decode("a=41_b"), b"aA b");
    }

    #[test]
    fn raw_bytes_are_utf8_or_windows_1252() {
        assert_eq!(eight_bit(b"Caf\xc3\xa9"), "Caf\u{e9}");
        assert_eq!(eight_bit(b"Caf\xe9 \x80"), "Caf\u{e9} \u{20ac}");
    }

    #[test]
    fn a_plain_param_is_read_unquoted_unless_an_rfc2231_one_is_there() {
        let value = "attachment; size=3; filename=\"a \\\"b\\\"; c.pdf\"";
        assert_eq!(
            plain_param(value, "filename").as_deref(),
            Some("a \"b\"; c.pdf")
        );
        assert_eq!(
            plain_param("inline; FileName=x.png", "filename").as_deref(),
            Some("x.png")
        );
        assert_eq!(plain_param("attachment", "filename"), None);
        let both = "attachment; filename=\"old.pdf\"; filename*=utf-8''new.pdf";
        assert_eq!(plain_param(both, "filename"), None);
    }
}
