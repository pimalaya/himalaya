//! # SMTP backend
//!
//! The SMTP adapter of the shared cross-protocol client, a send-only
//! transport for the storage backends that cannot send themselves.
//!
//! The RFC 5321 envelope is derived from the message headers: `From:`
//! becomes the reverse path, and `To:`, `Cc:` and `Bcc:` the forward
//! paths. The `Bcc:` field itself is then removed from the transmitted
//! message (RFC 5322 section 3.6.3), so blind recipients stay blind.

use io_smtp::client::SmtpClient as _;
use std::borrow::Cow;

use anyhow::{Result, anyhow, bail};
use io_smtp::rfc5321::{
    SmtpDomain, SmtpEhloDomain, SmtpForwardPath, SmtpLocalPart, SmtpMailbox, SmtpReversePath,
};
use mail_parser::{Address as MailParserAddress, MessageParser};

use crate::smtp::client::SmtpClient;

impl SmtpClient {
    /// Runs the RFC 5321 mail transaction (MAIL FROM / RCPT TO / DATA)
    /// for `raw`, deriving the envelope from its headers.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let (reverse, forwards) = {
            let parsed = MessageParser::default()
                .parse_headers(&raw)
                .ok_or_else(|| anyhow!("Could not parse raw RFC 5322 message"))?;

            let reverse = parsed
                .from()
                .and_then(first_address)
                .ok_or_else(|| anyhow!("No `From:` header found in raw message"))?;
            let reverse = parse_smtp_mailbox(&reverse)?;

            let mut forwards = Vec::new();
            for group in [parsed.to(), parsed.cc(), parsed.bcc()]
                .into_iter()
                .flatten()
            {
                for address in addresses(group) {
                    forwards.push(parse_smtp_mailbox(&address)?);
                }
            }

            (reverse, forwards)
        };

        if forwards.is_empty() {
            bail!("No `To:` / `Cc:` / `Bcc:` recipients found in raw message");
        }

        let reverse_path = SmtpReversePath::SmtpMailbox(reverse);
        let forward_paths: Vec<SmtpForwardPath<'static>> =
            forwards.into_iter().map(SmtpForwardPath::from).collect();

        self.send(reverse_path, forward_paths, strip_bcc(&raw))?;
        Ok(())
    }
}

/// Removes every `Bcc:` field, continuation lines included, from the
/// header section of `raw`, leaving the body untouched.
///
/// The envelope already carries the blind recipients; transmitting the
/// field would disclose them to every other recipient.
fn strip_bcc(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut skipping = false;
    let mut rest = raw;

    while !rest.is_empty() {
        let end = rest
            .iter()
            .position(|&b| b == b'\n')
            .map_or(rest.len(), |i| i + 1);
        let (line, tail) = rest.split_at(end);

        // The first empty line ends the header section.
        if line == b"\r\n" || line == b"\n" {
            out.extend_from_slice(rest);
            return out;
        }

        let folded = matches!(line.first(), Some(b' ' | b'\t'));
        if !folded {
            // RFC 5322 obsolete syntax allows whitespace before the colon.
            skipping = line
                .iter()
                .position(|&b| b == b':')
                .is_some_and(|colon| line[..colon].trim_ascii_end().eq_ignore_ascii_case(b"bcc"));
        }
        if !skipping {
            out.extend_from_slice(line);
        }
        rest = tail;
    }

    out
}

/// Flattens a mail-parser address group into bare `local-part@domain`
/// strings.
fn addresses(group: &MailParserAddress<'_>) -> Vec<String> {
    group
        .clone()
        .into_list()
        .into_iter()
        .filter_map(|address| {
            let email = address.address?.into_owned();
            (!email.is_empty()).then_some(email)
        })
        .collect()
}

/// First address in a group; picks the `From:` envelope sender.
fn first_address(group: &MailParserAddress<'_>) -> Option<String> {
    addresses(group).into_iter().next()
}

/// Parses `local-part@domain` into an owned SMTP mailbox.
fn parse_smtp_mailbox(address: &str) -> Result<SmtpMailbox<'static>> {
    let (local, domain) = address
        .rsplit_once('@')
        .ok_or_else(|| anyhow!("Invalid email address `{address}` in envelope"))?;
    if local.is_empty() || domain.is_empty() {
        bail!("Invalid email address `{address}` in envelope");
    }

    Ok(SmtpMailbox {
        local_part: SmtpLocalPart(Cow::Owned(local.to_string())),
        domain: SmtpEhloDomain::SmtpDomain(SmtpDomain(Cow::Owned(domain.to_string()))),
    })
}

#[cfg(test)]
mod tests {
    use super::strip_bcc;

    #[test]
    fn removes_the_bcc_field_and_keeps_everything_else() {
        let raw =
            b"From: a@x\r\nTo: b@x\r\nBcc: c@x\r\nSubject: s\r\n\r\nBcc: in the body stays\r\n";
        assert_eq!(
            strip_bcc(raw),
            b"From: a@x\r\nTo: b@x\r\nSubject: s\r\n\r\nBcc: in the body stays\r\n"
        );
    }

    #[test]
    fn removes_folded_continuation_lines_and_any_case() {
        let raw = b"From: a@x\r\nBCC: c@x,\r\n d@x,\r\n\te@x\r\nTo: b@x\r\n\r\nbody\r\n";
        assert_eq!(strip_bcc(raw), b"From: a@x\r\nTo: b@x\r\n\r\nbody\r\n");
    }

    #[test]
    fn removes_the_obsolete_spelling_with_space_before_the_colon() {
        let raw = b"From: a@x\r\nBcc : c@x\r\nTo: b@x\r\n\r\nbody\r\n";
        assert_eq!(strip_bcc(raw), b"From: a@x\r\nTo: b@x\r\n\r\nbody\r\n");
    }

    #[test]
    fn leaves_a_message_without_bcc_byte_identical() {
        let raw = b"From: a@x\nTo: b@x\nX-Bccish: keep\n\nbody\n";
        assert_eq!(strip_bcc(raw), raw.to_vec());
    }
}
