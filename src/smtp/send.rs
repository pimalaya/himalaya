//! # SMTP send
//!
//! The `smtp send` command, an RFC 5321 `MAIL FROM`, `RCPT TO` and
//! `DATA` exchange.

use io_smtp::client::SmtpClient as _;
use std::borrow::Cow;

use anyhow::Result;
use clap::Parser;
use io_smtp::rfc5321::{
    SmtpDomain, SmtpEhloDomain, SmtpForwardPath, SmtpLocalPart, SmtpMailbox, SmtpReversePath,
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
    #[command(flatten)]
    pub message: MessageArg,
}

impl SmtpSendCommand {
    /// Derives the envelope from the headers, then sends the message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        let message = self.message.parse()?;
        client.send(self.mail_from, self.rcpt_to, message.into_bytes())?;
        printer.out(Message::new("Message successfully sent"))
    }
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
