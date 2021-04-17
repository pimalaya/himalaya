use clap::{self, App, Arg, ArgMatches, SubCommand};
use error_chain::error_chain;
use log::{debug, trace};

use crate::{config::model::Account, imap::model::ImapConnector, info};

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
        MsgCli(crate::msg::cli::Error, crate::msg::cli::ErrorKind);
        OutputUtils(crate::output::utils::Error, crate::output::utils::ErrorKind);
    }
}

pub fn mbox_source_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Selects a specific mailbox")
        .value_name("MAILBOX")
        .default_value("INBOX")
}

pub fn mbox_target_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("target")
        .help("Specifies the targetted mailbox")
        .value_name("TARGET")
}

pub fn mbox_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "m"])
        .about("Lists all mailboxes")]
}

pub fn mbox_matches(account: &Account, matches: &ArgMatches) -> Result<bool> {
    if let Some(_) = matches.subcommand_matches("mailboxes") {
        debug!("[mbox::cli::matches] mailboxes command matched");

        let mut imap_conn = ImapConnector::new(&account)?;
        let mboxes = imap_conn.list_mboxes()?;
        info!(&mboxes);
        trace!("[mbox::cli::matches] {:#?}", mboxes);

        imap_conn.logout();
        return Ok(true);
    }

    debug!("[mbox::cli::matches] nothing matched");
    Ok(false)
}
