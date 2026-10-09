//! # SMTP send
//!
//! The `smtp send` command, an RFC 5321 `MAIL FROM`, `RCPT TO` and
//! `DATA` exchange, optionally requesting delivery status notifications
//! (RFC 3461).

use io_smtp::client::SmtpClient as _;
use std::borrow::Cow;

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use io_smtp::{
    message::SmtpMessageSendOptions,
    rfc3461::{
        capability::DSN,
        parameter::{SmtpDsnNotify, SmtpDsnRet},
    },
    rfc5321::{
        SmtpDomain, SmtpEhloDomain, SmtpForwardPath, SmtpLocalPart, SmtpMailbox, SmtpParameter,
        SmtpReversePath,
    },
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{shared::message::arg::MessageArg, smtp::client::SmtpClient};

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
/// The shared `message send` derives the envelope from the headers
/// instead.
#[derive(Debug, Parser)]
pub struct SmtpSendCommand {
    /// The envelope sender (MAIL FROM reverse path).
    ///
    /// Pass an empty value or `<>` for the null reverse path.
    #[arg(long, short = 'f', value_name = "ADDR", value_parser = reverse_path_parser)]
    pub mail_from: SmtpReversePath<'static>,
    /// The envelope recipient(s) (RCPT TO forward path); repeatable.
    #[arg(long, short = 't', value_name = "ADDR", required = true, value_parser = forward_path_parser)]
    pub rcpt_to: Vec<SmtpForwardPath<'static>>,
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
    /// Sends the message over the explicit envelope, byte for byte.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        let mut mail_parameters = Vec::new();
        let mut rcpt_parameters = Vec::new();

        if let Some(ret) = self.ret {
            mail_parameters.push(SmtpDsnRet::from(ret).into_parameter());
        }

        if let Some(envid) = self.envid {
            mail_parameters.push(SmtpParameter::envid(envid));
        }

        if !self.notify.is_empty() {
            if self.notify.len() > 1 && self.notify.contains(&NotifyArg::Never) {
                bail!("Notify condition `never` cannot be combined with another one");
            }

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
        };

        client.send(self.mail_from, self.rcpt_to, message.into_bytes(), options)?;
        printer.out(Message::new("Message successfully sent"))
    }
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

/// Clap value parser for ENVID: printable US-ASCII without `+` and `=`,
/// so the value needs no xtext encoding, and at most 100 characters
/// (RFC 3461 4.4).
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
/// null reverse path, otherwise parses a `local-part@domain` mailbox.
fn reverse_path_parser(addr: &str) -> Result<SmtpReversePath<'static>, String> {
    let addr = addr.trim();

    if addr.is_empty() || addr == "<>" {
        return Ok(SmtpReversePath::Null);
    }

    Ok(SmtpReversePath::SmtpMailbox(mailbox_parser(addr)?))
}

/// Clap value parser for RCPT TO: parses a `local-part@domain` mailbox.
fn forward_path_parser(addr: &str) -> Result<SmtpForwardPath<'static>, String> {
    Ok(SmtpForwardPath(mailbox_parser(addr)?))
}

/// Builds an SMTP [`SmtpMailbox`] from a `local-part@domain` string.
fn mailbox_parser(addr: &str) -> Result<SmtpMailbox<'static>, String> {
    let Some((local, domain)) = addr.trim().rsplit_once('@') else {
        return Err(format!("expected local-part@domain, got `{addr}`"));
    };

    if local.is_empty() || domain.is_empty() {
        return Err(format!("expected local-part@domain, got `{addr}`"));
    }

    Ok(SmtpMailbox {
        local_part: SmtpLocalPart(Cow::Owned(local.to_owned())),
        domain: SmtpEhloDomain::SmtpDomain(SmtpDomain(Cow::Owned(domain.to_owned()))),
    })
}
