//! Module related to message template handling.
//!
//! This module gathers all message template commands.  

use anyhow::Result;
use atty::Stream;
use himalaya_lib::{AccountConfig, Backend, Email, Sender, TplOverride};
use std::io::{self, BufRead};

use crate::printer::Printer;

/// Generate a new message template.
pub fn new<'a, P: Printer>(
    opts: TplOverride<'a>,
    config: &'a AccountConfig,
    printer: &'a mut P,
) -> Result<()> {
    let tpl = Email::default().to_tpl(opts, config)?;
    printer.print_struct(tpl)
}

/// Generate a reply message template.
pub fn reply<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq: &str,
    all: bool,
    opts: TplOverride<'_>,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let tpl = backend
        .email_get(mbox, seq)?
        .into_reply(all, config)?
        .to_tpl(opts, config)?;
    printer.print_struct(tpl)
}

/// Generate a forward message template.
pub fn forward<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq: &str,
    opts: TplOverride<'_>,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let tpl = backend
        .email_get(mbox, seq)?
        .into_forward(config)?
        .to_tpl(opts, config)?;
    printer.print_struct(tpl)
}

/// Saves a message based on a template.
pub fn save<'a, P: Printer, B: Backend<'a> + ?Sized>(
    mbox: &str,
    config: &AccountConfig,
    attachments_paths: Vec<&str>,
    tpl: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let tpl = if atty::is(Stream::Stdin) || printer.is_json() {
        tpl.replace("\r", "")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\n")
    };
    let msg = Email::from_tpl(&tpl)?.add_attachments(attachments_paths)?;
    let raw_msg = msg.into_sendable_msg(config)?.formatted();
    backend.email_add(mbox, &raw_msg, "seen")?;
    printer.print_struct("Template successfully saved")
}

/// Sends a message based on a template.
pub fn send<'a, P: Printer, B: Backend<'a> + ?Sized, S: Sender + ?Sized>(
    mbox: &str,
    account: &AccountConfig,
    attachments_paths: Vec<&str>,
    tpl: &str,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
) -> Result<()> {
    let tpl = if atty::is(Stream::Stdin) || printer.is_json() {
        tpl.replace("\r", "")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\n")
    };
    let msg = Email::from_tpl(&tpl)?.add_attachments(attachments_paths)?;
    let sent_msg = sender.send(account, &msg)?;
    backend.email_add(mbox, &sent_msg, "seen")?;
    printer.print_struct("Template successfully sent")
}
