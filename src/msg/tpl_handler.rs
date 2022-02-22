//! Module related to message template handling.
//!
//! This module gathers all message template commands.  

use anyhow::Result;
use atty::Stream;
use std::io::{self, BufRead};

use crate::{
    backends::Backend,
    config::AccountConfig,
    msg::{Msg, TplOverride},
    output::PrinterService,
    smtp::SmtpService,
};

/// Generate a new message template.
pub fn new<'a, P: PrinterService>(
    opts: TplOverride<'a>,
    account: &'a AccountConfig,
    printer: &'a mut P,
) -> Result<()> {
    let tpl = Msg::default().to_tpl(opts, account)?;
    printer.print(tpl)
}

/// Generate a reply message template.
pub fn reply<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq: &str,
    all: bool,
    opts: TplOverride<'a>,
    mbox: &str,
    config: &'a AccountConfig,
    printer: &'a mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let tpl = backend
        .get_msg(mbox, seq)?
        .into_reply(all, config)?
        .to_tpl(opts, config)?;
    printer.print(tpl)
}

/// Generate a forward message template.
pub fn forward<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq: &str,
    opts: TplOverride<'a>,
    mbox: &str,
    config: &'a AccountConfig,
    printer: &'a mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let tpl = backend
        .get_msg(mbox, seq)?
        .into_forward(config)?
        .to_tpl(opts, config)?;
    printer.print(tpl)
}

/// Saves a message based on a template.
pub fn save<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    mbox: &str,
    config: &AccountConfig,
    attachments_paths: Vec<&str>,
    tpl: &str,
    printer: &mut P,
    backend: Box<&mut B>,
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
    let msg = Msg::from_tpl(&tpl)?.add_attachments(attachments_paths)?;
    let raw_msg = msg.into_sendable_msg(config)?.formatted();
    backend.add_msg(mbox, &raw_msg, "seen")?;
    printer.print("Template successfully saved")
}

/// Sends a message based on a template.
pub fn send<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
    mbox: &str,
    account: &AccountConfig,
    attachments_paths: Vec<&str>,
    tpl: &str,
    printer: &mut P,
    backend: Box<&mut B>,
    smtp: &mut S,
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
    let msg = Msg::from_tpl(&tpl)?.add_attachments(attachments_paths)?;
    let sent_msg = smtp.send_msg(account, &msg)?;
    backend.add_msg(mbox, &sent_msg.formatted(), "seen")?;
    printer.print("Template successfully sent")
}
