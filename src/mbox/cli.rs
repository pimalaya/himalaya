use anyhow::Result;
use clap;
use log::{debug, trace};

use crate::{ctx::Ctx, imap::model::ImapConnector, mbox::model::Mboxes};

pub fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "m"])
        .about("Lists all mailboxes")]
}

pub fn matches(ctx: &Ctx) -> Result<bool> {
    if let Some(_) = ctx.arg_matches.subcommand_matches("mailboxes") {
        debug!("mailboxes command matched");

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        let names = imap_conn.list_mboxes()?;
        let mboxes = Mboxes::from(&names);
        debug!("found {} mailboxes", mboxes.0.len());
        trace!("mailboxes: {:?}", mboxes);
        ctx.output.print(mboxes);

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}

// == Argument Functions ==
pub fn source_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Selects a specific mailbox")
        .value_name("MAILBOX")
        .default_value("INBOX")
}

pub fn mbox_target_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("target")
        .help("Specifies the targetted mailbox")
        .value_name("TARGET")
}
