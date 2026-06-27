//! Shared IMAP helpers: RFC 2047 header decoding and envelope address
//! formatting, used by the `fetch` and `thread` commands.

use io_imap::types::envelope::Address;
use log::debug;
use rfc2047_decoder::{Decoder, RecoverStrategy};

/// Decode an RFC 2047 MIME-encoded string, falling back to the input
/// on error.
pub fn decode_mime(s: &str) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    match decoder.decode(s.as_bytes()) {
        Ok(s) => s,
        Err(err) => {
            debug!("cannot decode rfc2047 string `{s}`: {err}");
            s.to_string()
        }
    }
}

/// Format an envelope address as `Name <local@host>`, or just the
/// email when the personal name is absent.
pub fn format_address(addr: &Address<'_>) -> String {
    let email = format_email(addr);

    if let Some(name) = &addr.name.0 {
        let name = decode_mime(&String::from_utf8_lossy(name.as_ref()));
        if !name.is_empty() {
            return format!("{name} <{email}>");
        }
    }

    email
}

/// Join the `local@host` parts of an envelope address.
fn format_email(addr: &Address<'_>) -> String {
    let mailbox = addr
        .mailbox
        .0
        .as_ref()
        .map(|m| String::from_utf8_lossy(m.as_ref()).to_string())
        .unwrap_or_default();
    let host = addr
        .host
        .0
        .as_ref()
        .map(|h| String::from_utf8_lossy(h.as_ref()).to_string())
        .unwrap_or_default();

    if !mailbox.is_empty() && !host.is_empty() {
        format!("{mailbox}@{host}")
    } else {
        mailbox
    }
}
