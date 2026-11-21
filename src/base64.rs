//! # Base64
//!
//! The RFC 4648 standard alphabet bytes travel in through JSON: the
//! outputs carrying a body or a part, the JMAP `Basic` header and the
//! data the MCP server takes, on base64-codec.

use base64_codec::codec::Base64Codec;

/// The standard alphabet of `bytes`, padded.
pub fn encode(bytes: &[u8]) -> String {
    let mut encoded = Vec::new();
    Base64Codec::default().encode_vec(bytes, &mut encoded);

    // NOTE: the base64 alphabet is ASCII, so this never falls back.
    String::from_utf8(encoded).unwrap_or_default()
}

/// The bytes `data` encodes in the standard alphabet, padding optional,
/// or `None` when it holds anything else: whitespace, a byte outside the
/// alphabet, non-zero trailing bits, a lone last symbol or padding past
/// the last quantum.
pub fn decode_strict(data: &str) -> Option<Vec<u8>> {
    let symbols = data.trim_end_matches('=');
    let padding = data.len() - symbols.len();

    if padding > (4 - symbols.len() % 4) % 4 {
        return None;
    }

    let codec = Base64Codec {
        pad: false,
        ..Base64Codec::default()
    };

    // NOTE: the codec decodes leniently and never fails, so the data is
    // taken only when it is the very encoding of what it decodes to.
    let mut bytes = Vec::new();
    codec.decode_vec(symbols.as_bytes(), &mut bytes);
    let mut canonical = Vec::new();
    codec.encode_vec(&bytes, &mut canonical);

    (canonical == symbols.as_bytes()).then_some(bytes)
}
