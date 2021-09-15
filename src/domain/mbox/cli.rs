use anyhow::Result;
use clap;
use log::{debug, trace};

use crate::{
    domain::{imap::service::ImapServiceInterface, mbox::entity::Mboxes},
    output::service::{OutputService, OutputServiceInterface},
};

pub fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "m"])
        .about("Lists all mailboxes")]
}

pub fn matches<ImapService: ImapServiceInterface>(
    arg_matches: &clap::ArgMatches,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<bool> {
    if let Some(_) = arg_matches.subcommand_matches("mailboxes") {
        debug!("mailboxes command matched");
        let names = imap.list_mboxes()?;
        let mboxes = Mboxes::from(&names);
        debug!("mboxes len: {}", mboxes.0.len());
        trace!("{:#?}", mboxes);
        output.print(mboxes)?;
        imap.logout()?;
        return Ok(true);
    }

    Ok(false)
}

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
