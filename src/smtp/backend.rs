//! # SMTP backend
//!
//! The SMTP adapter of the shared cross-protocol client, a send-only
//! transport for the storage backends that cannot send themselves.
//!
//! The RFC 5321 envelope comes from [`SubmissionEnvelope`], derived from
//! the message headers. io-smtp then removes the `Bcc:` field from what
//! it transmits.

use io_smtp::client::SmtpClient as _;
use std::borrow::Cow;

use anyhow::{Result, anyhow, bail};
use io_smtp::{
    message::SmtpMessageSendOptions,
    rfc5321::{
        SmtpDomain, SmtpEhloDomain, SmtpForwardPath, SmtpLocalPart, SmtpMailbox, SmtpReversePath,
    },
};

use crate::{email::submission::SubmissionEnvelope, smtp::client::SmtpClient};

impl SmtpClient {
    /// Runs the RFC 5321 mail transaction (MAIL FROM / RCPT TO / DATA)
    /// for `raw`, deriving the envelope from its headers.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let envelope = SubmissionEnvelope::parse(&raw)?;
        let reverse_path = SmtpReversePath::SmtpMailbox(parse_smtp_mailbox(&envelope.from)?);
        let forward_paths = envelope
            .rcpts
            .iter()
            .map(|rcpt| Ok(SmtpForwardPath::from(parse_smtp_mailbox(rcpt)?)))
            .collect::<Result<Vec<_>>>()?;

        self.send(
            reverse_path,
            forward_paths,
            raw,
            SmtpMessageSendOptions::default(),
        )?;
        Ok(())
    }
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
