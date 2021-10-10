//! Module related to message template handling.
//!
//! This module gathers all message template commands.  

use anyhow::Result;

use crate::{
    config::Account,
    domain::{
        imap::ImapServiceInterface,
        msg::{Msg, Tpl, TplOverride},
    },
    output::service::OutputServiceInterface,
};

/// Generate a new message template.
pub fn new<'a, OutputService: OutputServiceInterface>(
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
) -> Result<()> {
    let msg = Msg::default();
    let tpl = Tpl::from_msg(opts, &msg, account);
    output.print(tpl)
}

/// Generate a reply message template.
pub fn reply<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    seq: &str,
    all: bool,
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let msg = imap.find_msg(seq)?.into_reply(all, account)?;
    let tpl = Tpl::from_msg(opts, &msg, account);
    output.print(tpl)
}

/// Generate a forward message template.
pub fn forward<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    seq: &str,
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let msg = imap.find_msg(seq)?.into_forward(account)?;
    let tpl = Tpl::from_msg(opts, &msg, account);
    output.print(tpl)
}
