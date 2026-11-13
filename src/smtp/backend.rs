//! # SMTP backend
//!
//! The SMTP adapter of the unified client, a send-only
//! transport for the storage backends that cannot send themselves.
//!
//! The RFC 5321 envelope comes from [`SubmissionEnvelope`], derived from
//! the message headers. io-smtp then removes the `Bcc:` field from what
//! it transmits.

use io_smtp::client::SmtpClient as _;

use anyhow::{Result, anyhow};
use io_smtp::{
    message::{SmtpMessageSend, SmtpMessageSendOptions},
    rfc5321::{SmtpForwardPath, SmtpMailbox, SmtpReversePath},
};

use crate::{email::submission::SubmissionEnvelope, smtp::client::SmtpClient};

impl SmtpClient {
    /// Runs the RFC 5321 mail transaction (MAIL FROM / RCPT TO / DATA)
    /// for `raw`, deriving the envelope from its headers.
    ///
    /// An address the transaction cannot carry fails the send before
    /// anything is written.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let envelope = SubmissionEnvelope::parse(&raw)?;
        let reverse_path = SmtpReversePath::SmtpMailbox(parse_smtp_mailbox(&envelope.from)?);
        let forward_paths = envelope
            .rcpts
            .iter()
            .map(|rcpt| Ok(SmtpForwardPath::from(parse_smtp_mailbox(rcpt)?)))
            .collect::<Result<Vec<_>>>()?;
        let options = SmtpMessageSendOptions::default();

        self.run(SmtpMessageSend::new(
            reverse_path,
            forward_paths,
            raw,
            options,
        ))?;
        Ok(())
    }
}

/// Parses `local-part@domain` into an SMTP mailbox (RFC 5321 4.1.2).
fn parse_smtp_mailbox(address: &str) -> Result<SmtpMailbox<'_>> {
    SmtpMailbox::try_from(address).map_err(|err| {
        anyhow!(
            "Invalid email address `{}` in envelope: {err}",
            address.escape_debug()
        )
    })
}

#[cfg(test)]
mod tests {
    use crate::smtp::backend::parse_smtp_mailbox;

    #[test]
    fn an_address_the_transaction_cannot_carry_is_refused() {
        assert!(parse_smtp_mailbox("alice@example.org").is_ok());

        for refused in [
            "alice",
            "@example.org",
            "alice@",
            "alice smith@example.org",
            "alice@example.org\r\nRCPT TO:<eve@example.net>",
            "alice\u{0}@example.org",
        ] {
            let err = parse_smtp_mailbox(refused).unwrap_err().to_string();
            assert!(err.starts_with("Invalid email address `"), "{err}");
            assert!(!err.contains(['\r', '\n', '\u{0}']), "{err:?}");
        }
    }
}
