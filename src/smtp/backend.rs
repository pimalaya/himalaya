//! SMTP adapter for the shared cross-protocol client.
//!
//! SMTP is send-only: the dispatcher registers it as a sending transport
//! for storage backends that cannot send themselves (IMAP, Maildir,
//! m2dir). `send_message` derives the RFC 5321 envelope from the raw
//! message headers (From: as the reverse path; To:/Cc:/Bcc: as the
//! forward paths), then reuses [`SmtpClient`]'s `send`. The envelope
//! parsing is lifted from the retired io-email SMTP driver.

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

        self.send(reverse_path, forward_paths, raw)?;
        Ok(())
    }
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
