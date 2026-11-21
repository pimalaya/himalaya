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
    client::SmtpClientError,
    message::{SmtpMessageSend, SmtpMessageSendError, SmtpMessageSendOptions},
    rfc5321::{SmtpForwardPath, SmtpMailbox, SmtpReversePath, rcpt::SmtpRcptError},
};
use pimalaya_cli::{error::CodedError, sanitize};

use crate::{
    error::ErrorCode, smtp::client::SmtpClient, unified::email::submission::SubmissionEnvelope,
};

impl SmtpClient {
    /// Runs the RFC 5321 mail transaction (MAIL FROM / RCPT TO / DATA)
    /// for `raw`, deriving the envelope from its headers.
    ///
    /// An address the transaction cannot carry fails the send before
    /// anything is written, and a recipient the server refuses fails it
    /// coded `recipient-refused`, the message sent to nobody.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let envelope = SubmissionEnvelope::parse(&raw)?;
        let reverse_path = SmtpReversePath::SmtpMailbox(parse_smtp_mailbox(&envelope.from)?);
        let forward_paths = envelope
            .rcpts
            .iter()
            .map(|rcpt| Ok(SmtpForwardPath::from(parse_smtp_mailbox(rcpt)?)))
            .collect::<Result<Vec<_>>>()?;
        let options = SmtpMessageSendOptions::default();

        let send = SmtpMessageSend::new(reverse_path, forward_paths, raw, options);
        self.run(send).map_err(send_error)?;
        Ok(())
    }
}

/// The failure of a send, coded `recipient-refused` when the server refused
/// a recipient, the first one: io-smtp offers none after it.
fn send_error(err: SmtpClientError) -> anyhow::Error {
    match err {
        SmtpClientError::MessageSend(SmtpMessageSendError::RcptTo {
            forward_path,
            error: SmtpRcptError::Rejected { code, message },
        }) => {
            let message = sanitize::text(message.trim_end());
            let message = format!("SMTP server refused recipient {forward_path}: {code} {message}");
            CodedError::new(ErrorCode::RecipientRefused, message).into()
        }
        err => err.into(),
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
    use io_smtp::{
        client::SmtpClientError,
        message::SmtpMessageSendError,
        rfc5321::{SmtpForwardPath, rcpt::SmtpRcptError},
    };
    use pimalaya_cli::error::code_of;

    use crate::{
        error::ErrorCode,
        smtp::backend::{parse_smtp_mailbox, send_error},
    };

    /// A refused `RCPT TO` fails the send coded `recipient-refused`,
    /// naming the address io-smtp stopped at.
    #[test]
    fn a_refused_recipient_is_coded_and_named() {
        let mailbox = parse_smtp_mailbox("bob@example.org").unwrap();
        let err = SmtpClientError::MessageSend(SmtpMessageSendError::RcptTo {
            forward_path: SmtpForwardPath::from(mailbox),
            error: SmtpRcptError::Rejected {
                code: 550,
                message: String::from("No such user"),
            },
        });

        let err = send_error(err);

        assert_eq!(code_of(&err), Some(ErrorCode::RecipientRefused));
        let message = err.to_string();
        assert!(message.contains("bob@example.org"), "{message}");
        assert!(message.contains("550 No such user"), "{message}");
    }

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
