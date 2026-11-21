//! # IMAP utils
//!
//! Envelope address conversion, shared by the `fetch` and `thread`
//! commands, the sequence set argument the message commands take, and
//! the reading of a command's output.

use anyhow::{Result, anyhow};
use io_imap::{
    send::ImapOutput,
    types::{envelope::Address, sequence::SequenceSet},
};
use log::warn;
use pimalaya_cli::sanitize;

use crate::unified::message::header;

/// Takes the result out of the output of `command`, warning of the
/// server's alerts and of the response lines that did not decode whole.
pub fn output_data<T>(command: &str, output: ImapOutput<T>) -> T {
    for alert in &output.alerts {
        warn!("server sent an alert: {}", sanitize::text(alert));
    }

    if !output.skipped.is_empty() {
        let count = output.skipped.len();
        warn!(
            "skipped {count} unreadable line(s) of the {command} response, the result may be incomplete"
        );
    }

    if !output.partial.is_empty() {
        let count = output.partial.len();
        warn!(
            "decoded {count} line(s) of the {command} response in part, the result may be incomplete"
        );
    }

    output.data
}

/// Turns an ENVELOPE address list into unified addresses, the markers of
/// a group (RFC 3501 §7.4.2, a NIL host) left out and its members kept.
pub fn to_addresses(list: &[Address<'_>]) -> Vec<crate::unified::email::address::Address> {
    list.iter()
        .filter(|addr| addr.host.0.is_some())
        .map(to_address)
        .collect()
}

/// The unified address, its name decoded as `message read` decodes a
/// header, `None` when absent or empty.
fn to_address(addr: &Address<'_>) -> crate::unified::email::address::Address {
    let name = addr
        .name
        .0
        .as_ref()
        .map(|name| header::decode_bytes(name.as_ref()))
        .filter(|name| !name.is_empty());

    crate::unified::email::address::Address {
        name,
        email: format_email(addr),
    }
}

/// Joins the `local@host` parts of an envelope address.
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

/// Checks a sequence set argument as clap takes it, so a value that is
/// none is refused with the command line, before the session opens.
pub fn sequence_set_parser(raw: &str) -> Result<String, String> {
    match parse_sequence_set(raw) {
        Ok(_) => Ok(raw.to_owned()),
        Err(err) => Err(err.to_string()),
    }
}

/// Parses a sequence set argument, naming it when it is not one.
pub fn parse_sequence_set(raw: &str) -> Result<SequenceSet> {
    SequenceSet::try_from(raw).map_err(|_| {
        anyhow!("Invalid sequence set `{raw}`: expected numbers as in `1`, `1,3` or `2:*`")
    })
}

#[cfg(test)]
mod tests {
    use crate::imap::utils::parse_sequence_set;

    #[test]
    fn a_bad_sequence_set_is_named() {
        assert!(parse_sequence_set("1,3:*").is_ok());

        let err = parse_sequence_set("abc").unwrap_err().to_string();
        assert!(err.starts_with("Invalid sequence set `abc`"), "{err}");
    }
}

#[cfg(test)]
mod parser_tests {
    use clap::Parser;

    use crate::cli::Cli;

    /// A sequence set that is none is refused with the command line, so
    /// it exits 2 before the secret command runs and the session opens.
    #[test]
    fn a_bad_sequence_set_is_refused_by_the_grammar() {
        for args in [
            &["imap", "fetch", "abc"][..],
            &["imap", "store", "abc", "-f", "\\Seen"],
            &["imap", "copy", "abc", "Archive"],
            &["imap", "move", "abc", "Archive"],
        ] {
            let argv = [&["himalaya"][..], args].concat();
            let err = Cli::try_parse_from(&argv).err();
            let err = err.unwrap_or_else(|| panic!("{args:?} parsed"));
            assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation, "{err}");

            let argv = [&["himalaya"][..], args].concat();
            let argv: Vec<String> = argv.iter().map(|arg| arg.replace("abc", "1:*")).collect();
            assert!(Cli::try_parse_from(&argv).is_ok(), "{argv:?}");
        }
    }
}
