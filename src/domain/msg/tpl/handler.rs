use anyhow::Result;
use log::{debug, trace};

use crate::{
    config::entity::Account,
    domain::{
        imap::ImapServiceInterface,
        msg::{Tpl, TplOverride},
    },
    output::service::OutputServiceInterface,
};

pub fn new<'a, OutputService: OutputServiceInterface>(
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
) -> Result<()> {
    let tpl = Tpl::new(&opts, account);
    trace!("template: {:#?}", tpl);
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
    let msg = imap.find_msg(seq)?;
    trace!("message: {:#?}", msg);
    let tpl = Tpl::reply(all, &msg, &opts, account);
    trace!("template: {:#?}", tpl);
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
    let msg = imap.find_msg(seq)?;
    trace!("message: {:#?}", msg);
    let tpl = Tpl::forward(&msg, &opts, account);
    trace!("template: {:#?}", tpl);
    output.print(tpl)?;
    Ok(())
}
