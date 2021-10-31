//! Module related to message template handling.
//!
//! This module gathers all message template commands.  

use anyhow::Result;

use crate::{
    config::Account,
    domain::{
        imap::ImapServiceInterface,
        msg::{Msg, TplOverride},
    },
    output::PrinterService,
};

/// Generate a new message template.
pub fn new<'a, Printer: PrinterService>(
    opts: TplOverride<'a>,
    account: &'a Account,
    printer: &'a mut Printer,
) -> Result<()> {
    let tpl = Msg::default().to_tpl(opts, account);
    printer.print(tpl)
}

/// Generate a reply message template.
pub fn reply<'a, Printer: PrinterService, ImapService: ImapServiceInterface<'a>>(
    seq: &str,
    all: bool,
    opts: TplOverride<'a>,
    account: &'a Account,
    printer: &'a mut Printer,
    imap: &'a mut ImapService,
) -> Result<()> {
    let tpl = imap
        .find_msg(seq)?
        .into_reply(all, account)?
        .to_tpl(opts, account);
    printer.print(tpl)
}

/// Generate a forward message template.
pub fn forward<'a, Printer: PrinterService, ImapService: ImapServiceInterface<'a>>(
    seq: &str,
    opts: TplOverride<'a>,
    account: &'a Account,
    printer: &'a mut Printer,
    imap: &'a mut ImapService,
) -> Result<()> {
    let tpl = imap
        .find_msg(seq)?
        .into_forward(account)?
        .to_tpl(opts, account);
    printer.print(tpl)
}
