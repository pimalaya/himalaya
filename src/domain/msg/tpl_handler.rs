//! Module related to message template handling.
//!
//! This module gathers all message template commands.  

use anyhow::Result;
use atty::Stream;
use imap::types::Flag;
use std::{
    convert::TryFrom,
    io::{self, BufRead},
};

use crate::{
    config::AccountConfig,
    domain::{
        imap::BackendService,
        msg::{Msg, TplOverride},
        Flags, Mbox, SmtpService,
    },
    output::PrinterService,
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
pub fn reply<'a, P: PrinterService, B: BackendService<'a> + ?Sized>(
    seq: &str,
    all: bool,
    opts: TplOverride<'a>,
    account: &'a AccountConfig,
    printer: &'a mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let tpl = backend
        .get_msg(account, seq)?
        .into_reply(all, account)?
        .to_tpl(opts, account)?;
    printer.print(tpl)
}

/// Generate a forward message template.
pub fn forward<'a, P: PrinterService, B: BackendService<'a> + ?Sized>(
    seq: &str,
    opts: TplOverride<'a>,
    account: &'a AccountConfig,
    printer: &'a mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let tpl = backend
        .get_msg(account, seq)?
        .into_forward(account)?
        .to_tpl(opts, account)?;
    printer.print(tpl)
}

/// Saves a message based on a template.
pub fn save<'a, P: PrinterService, B: BackendService<'a> + ?Sized>(
    mbox: &Mbox,
    account: &AccountConfig,
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
    let raw_msg = msg.into_sendable_msg(account)?.formatted();
    let flags = Flags::try_from(vec![Flag::Seen])?;
    backend.append_raw_msg_with_flags(mbox, &raw_msg, flags)?;
    printer.print("Template successfully saved")
}

/// Sends a message based on a template.
pub fn send<'a, P: PrinterService, B: BackendService<'a> + ?Sized, S: SmtpService>(
    mbox: &Mbox,
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
    let flags = Flags::try_from(vec![Flag::Seen])?;
    backend.append_raw_msg_with_flags(mbox, &sent_msg.formatted(), flags)?;
    printer.print("Template successfully sent")
}
