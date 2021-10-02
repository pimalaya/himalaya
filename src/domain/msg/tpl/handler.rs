use anyhow::Result;
use log::debug;

use crate::{
    config::entity::Account,
    domain::{
        imap::ImapServiceInterface,
        msg::{Msg, Tpl, TplOverride},
    },
    output::service::OutputServiceInterface,
};

pub fn new<'a, OutputService: OutputServiceInterface>(
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
) -> Result<()> {
    debug!("entering new handler");
    let msg = Msg::default();
    let tpl = Tpl::from_msg(opts, &msg, account);
    output.print(tpl)?;
    Ok(())
}

pub fn reply<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    seq: &str,
    all: bool,
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    debug!("entering reply handler");
    let msg = imap.find_msg(seq)?.into_reply(all, account)?;
    let tpl = Tpl::from_msg(opts, &msg, account);
    output.print(tpl)?;
    Ok(())
}

pub fn forward<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    seq: &str,
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    debug!("entering forward handler");
    let msg = imap.find_msg(seq)?.into_forward(account)?;
    let tpl = Tpl::from_msg(opts, &msg, account);
    output.print(tpl)?;
    Ok(())
}
