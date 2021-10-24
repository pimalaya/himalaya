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
    print::PrinterServiceInterface,
};

/// Generate a new message template.
pub fn new<'a, PrinterService: PrinterServiceInterface>(
    opts: TplOverride<'a>,
    account: &'a Account,
    printer: &'a mut PrinterService,
) -> Result<()> {
    let tpl = Msg::default().to_tpl(opts, account);
    printer.print(tpl)
}

/// Generate a reply message template.
pub fn reply<'a, PrinterService: PrinterServiceInterface, ImapService: ImapServiceInterface<'a>>(
    seq: &str,
    all: bool,
    opts: TplOverride<'a>,
    account: &'a Account,
    printer: &'a mut PrinterService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let tpl = imap
        .find_msg(seq)?
        .into_reply(all, account)?
        .to_tpl(opts, account);
    printer.print(tpl)
}

/// Generate a forward message template.
pub fn forward<
    'a,
    PrinterService: PrinterServiceInterface,
    ImapService: ImapServiceInterface<'a>,
>(
    seq: &str,
    opts: TplOverride<'a>,
    account: &'a Account,
    printer: &'a mut PrinterService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let tpl = imap
        .find_msg(seq)?
        .into_forward(account)?
        .to_tpl(opts, account);
    printer.print(tpl)
}
