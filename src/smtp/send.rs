//! # SMTP send
//!
//! The `smtp send` command, an RFC 5321 `MAIL FROM`, `RCPT TO` and
//! `DATA` exchange, optionally requesting delivery status notifications
//! (RFC 3461).

use io_smtp::client::SmtpClient as _;
use std::fmt;

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use io_smtp::{
    client::SmtpClientError,
    message::{SmtpMessageSend, SmtpMessageSendError, SmtpMessageSendOptions},
    rfc3461::{
        capability::DSN,
        parameter::{SmtpDsnNotify, SmtpDsnRet},
    },
    rfc5321::{
        SmtpForwardPath, SmtpMailbox, SmtpParameter, SmtpReversePath, data::SmtpDataError,
        mail::SmtpMailError, rcpt::SmtpRcptError,
    },
};
use pimalaya_cli::{
    error::{PartialOutcome, usage_error},
    printer::Printer,
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{smtp::client::SmtpClient, unified::message::arg::MessageArg};

/// Send a raw RFC 5322 message over SMTP.
///
/// The envelope is explicit, `--mail-from` being the reverse path and each
/// `--rcpt-to` a forward path, so the flags match the transaction exactly.
/// The message is the DATA payload, from a file path, an inline string or
/// piped standard input.
///
/// `--notify`, `--ret` and `--envid` request delivery status notifications
/// (RFC 3461), refused when the server does not announce `DSN`.
///
/// The unified `message send` derives the envelope from the headers
/// instead.
#[derive(Debug, Parser)]
pub struct SmtpSendCommand {
    /// The envelope sender (MAIL FROM reverse path).
    ///
    /// Pass an empty value or `<>` for the null reverse path.
    #[arg(long, short = 'f', value_name = "ADDR", value_parser = reverse_path_parser)]
    pub mail_from: String,
    /// The envelope recipient(s) (RCPT TO forward path); repeatable.
    #[arg(long, short = 't', value_name = "ADDR", required = true, value_parser = mailbox_parser)]
    pub rcpt_to: Vec<String>,
    /// The conditions a notification is sent on, for every recipient
    /// (RFC 3461 4.1); comma-separated.
    ///
    /// `never` cannot be combined with another condition.
    #[arg(long, value_name = "WHEN", value_delimiter = ',')]
    pub notify: Vec<NotifyArg>,
    /// How much of the message a failure notification returns (RFC 3461
    /// 4.3).
    #[arg(long, value_name = "WHAT")]
    pub ret: Option<RetArg>,
    /// The envelope identifier a notification carries back (RFC 3461
    /// 4.4).
    ///
    /// Printable ASCII without `+`, `=` or space, at most 100 characters.
    #[arg(long, value_name = "ID", value_parser = envid_parser)]
    pub envid: Option<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl SmtpSendCommand {
    /// Refuses `never` beside another notify condition, as clap would
    /// refuse two conflicting flags, before the session opens.
    pub fn check(&self) -> Result<()> {
        if self.notify.len() > 1 && self.notify.contains(&NotifyArg::Never) {
            let message = "notify condition `never` cannot be combined with another one";
            let err = clap::Error::raw(clap::error::ErrorKind::ArgumentConflict, message);
            return Err(usage_error(&err));
        }

        Ok(())
    }

    /// Sends the message over the explicit envelope, byte for byte.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        let mut mail_parameters = Vec::new();
        let mut rcpt_parameters = Vec::new();

        if let Some(ret) = self.ret {
            mail_parameters.push(SmtpDsnRet::from(ret).into_parameter());
        }

        if let Some(envid) = self.envid {
            mail_parameters.push(SmtpParameter::envid(envid)?);
        }

        if !self.notify.is_empty() {
            let notify = self
                .notify
                .into_iter()
                .map(SmtpDsnNotify::from)
                .fold(SmtpDsnNotify::NEVER, SmtpDsnNotify::or);

            rcpt_parameters.push(notify.into_parameter());
        }

        let wants_dsn = !mail_parameters.is_empty() || !rcpt_parameters.is_empty();
        let has_dsn = client.capabilities.iter().any(|capability| {
            let keyword = capability.split_whitespace().next().unwrap_or_default();
            keyword.eq_ignore_ascii_case(DSN)
        });

        if wants_dsn && !has_dsn {
            bail!(
                "SMTP server does not announce DSN, cannot request delivery status notifications"
            );
        }

        let message = self.message.parse()?;
        let options = SmtpMessageSendOptions {
            keep_bcc: true,
            mail_parameters,
            rcpt_parameters,
            ..Default::default()
        };

        let mail_from = match self.mail_from.as_str() {
            "" => SmtpReversePath::Null,
            addr => SmtpReversePath::SmtpMailbox(SmtpMailbox::try_from(addr)?),
        };
        let rcpt_to = self
            .rcpt_to
            .iter()
            .map(|addr| Ok(SmtpForwardPath(SmtpMailbox::try_from(addr.as_str())?)))
            .collect::<Result<Vec<_>>>()?;

        let output = SmtpSendOutput {
            mail_from: mail_from.to_string(),
            rcpt_to: rcpt_to.iter().map(ToString::to_string).collect(),
        };

        let send = SmtpMessageSend::new(mail_from, rcpt_to, message, options);

        match client.run(send) {
            Ok(_) => printer.out(output),
            Err(err) => match SmtpSendRefusalOutput::of(&err) {
                Some(refusal) => {
                    printer.out(refusal)?;
                    Err(PartialOutcome::default().into())
                }
                None => Err(err.into()),
            },
        }
    }
}

/// The `smtp send` output: the envelope the server accepted, the
/// transaction returning no id.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SmtpSendOutput {
    /// The reverse path, as sent in `MAIL FROM`.
    mail_from: String,
    /// The forward paths, as sent in `RCPT TO`.
    rcpt_to: Vec<String>,
}

impl fmt::Display for SmtpSendOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Message successfully sent to {}",
            self.rcpt_to.join(", ")
        )
    }
}

/// The result of an `smtp send` the server refused: each refusing reply,
/// the message reaching nobody.
#[derive(Debug, Eq, PartialEq, Serialize, JsonSchema)]
pub(crate) struct SmtpSendRefusalOutput {
    /// The refusing replies in order received: one for `MAIL FROM` or
    /// `DATA`, one per recipient when the server refused them all.
    refusals: Vec<SmtpSendRefusal>,
}

/// One reply refusing an `smtp send`.
#[derive(Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SmtpSendRefusal {
    /// The forward path the server refused, `null` for a refused `MAIL
    /// FROM` or `DATA`.
    rcpt_to: Option<String>,
    /// The reply code, 4xx or 5xx.
    code: u16,
    /// The text of the reply.
    text: String,
}

impl SmtpSendRefusalOutput {
    /// The replies a send failed on, if the server refused it rather than
    /// the exchange failing.
    fn of(err: &SmtpClientError) -> Option<Self> {
        let SmtpClientError::MessageSend(err) = err else {
            return None;
        };

        let refusal = |rcpt_to: Option<String>, code: u16, text: &str| SmtpSendRefusal {
            rcpt_to,
            code,
            text: text.trim_end().to_owned(),
        };

        let refusals = match err {
            SmtpMessageSendError::MailFrom(SmtpMailError::Rejected { code, message }) => {
                vec![refusal(None, *code, message)]
            }
            SmtpMessageSendError::RcptTo {
                forward_path,
                error: SmtpRcptError::Rejected { code, message },
            } => vec![refusal(Some(forward_path.to_string()), *code, message)],
            SmtpMessageSendError::NoRecipient(refused) => refused
                .iter()
                .map(|refused| {
                    let rcpt_to = Some(refused.forward_path.to_string());
                    refusal(rcpt_to, refused.code, &refused.message)
                })
                .collect(),
            SmtpMessageSendError::Data(
                SmtpDataError::CommandRejected { code, message }
                | SmtpDataError::BodyRejected { code, message },
            ) => vec![refusal(None, *code, message)],
            _ => return None,
        };

        Some(Self { refusals })
    }
}

impl fmt::Display for SmtpSendRefusalOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SMTP server refused the message, sent to nobody:")?;

        for refusal in &self.refusals {
            let text = sanitize::text(&refusal.text);
            match &refusal.rcpt_to {
                Some(rcpt) => writeln!(f, "  {}: {} {text}", sanitize::text(rcpt), refusal.code)?,
                None => writeln!(f, "  {} {text}", refusal.code)?,
            }
        }

        Ok(())
    }
}

/// The schema of `smtp send`: the envelope sent, or the refusals, told
/// apart by `refusals`.
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
pub(crate) enum SmtpSendOutcome {
    /// The server accepted the message.
    Sent(SmtpSendOutput),
    /// The server refused it.
    Refused(SmtpSendRefusalOutput),
}

/// One condition a delivery status notification is sent on (RFC 3461
/// 4.1).
#[derive(Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum NotifyArg {
    /// No notification at all.
    Never,
    /// A notification on successful delivery.
    Success,
    /// A notification on delivery failure.
    Failure,
    /// A notification when delivery is delayed.
    Delay,
}

impl From<NotifyArg> for SmtpDsnNotify {
    fn from(notify: NotifyArg) -> Self {
        match notify {
            NotifyArg::Never => SmtpDsnNotify::NEVER,
            NotifyArg::Success => SmtpDsnNotify::SUCCESS,
            NotifyArg::Failure => SmtpDsnNotify::FAILURE,
            NotifyArg::Delay => SmtpDsnNotify::DELAY,
        }
    }
}

/// How much of the message a failure notification returns (RFC 3461
/// 4.3).
#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum RetArg {
    /// The whole message.
    Full,
    /// The header section only.
    Hdrs,
}

impl From<RetArg> for SmtpDsnRet {
    fn from(ret: RetArg) -> Self {
        match ret {
            RetArg::Full => SmtpDsnRet::Full,
            RetArg::Hdrs => SmtpDsnRet::Hdrs,
        }
    }
}

/// Parses an ENVID: at most 100 printable US-ASCII characters (RFC 3461
/// 4.4), no `+` or `=` so it needs no xtext encoding.
fn envid_parser(envid: &str) -> Result<String, String> {
    if envid.is_empty() || envid.len() > 100 {
        return Err(format!("expected 1 to 100 characters, got {}", envid.len()));
    }

    if let Some(c) = envid
        .chars()
        .find(|c| !matches!(c, '!'..='~') || matches!(c, '+' | '='))
    {
        return Err(format!("unexpected character `{c}`"));
    }

    Ok(envid.to_owned())
}

/// Clap value parser for MAIL FROM: maps an empty value or `<>` to the
/// null reverse path, kept empty, otherwise checks a mailbox.
fn reverse_path_parser(addr: &str) -> Result<String, String> {
    let addr = addr.trim();

    if addr.is_empty() || addr == "<>" {
        return Ok(String::new());
    }

    mailbox_parser(addr)
}

/// Clap value parser for RCPT TO: checks a `local-part@domain` mailbox
/// as the transaction takes it (RFC 5321 4.1.2), and keeps it trimmed.
fn mailbox_parser(addr: &str) -> Result<String, String> {
    let addr = addr.trim();

    match SmtpMailbox::try_from(addr) {
        Ok(_) => Ok(addr.to_owned()),
        Err(err) => Err(format!(
            "expected local-part@domain, got `{}`: {err}",
            addr.escape_debug()
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::smtp::send::{mailbox_parser, reverse_path_parser};

    #[test]
    fn an_envelope_address_is_checked_as_the_transaction_takes_it() {
        assert_eq!(mailbox_parser(" a@example.org ").unwrap(), "a@example.org");
        assert_eq!(reverse_path_parser("<>").unwrap(), "");

        for refused in [
            "a",
            "@example.org",
            "a@",
            "a b@example.org",
            "a@example.org\r\nRCPT TO:<eve@example.net>",
            "a\u{7}@example.org",
        ] {
            let err = mailbox_parser(refused).unwrap_err();
            assert!(err.starts_with("expected local-part@domain"), "{err}");
            assert!(!err.contains(['\r', '\n', '\u{7}']), "{err:?}");
        }
    }
}
