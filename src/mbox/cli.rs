use clap;
use error_chain::error_chain;
use log::{debug, trace};

use crate::{app::App, imap::model::ImapConnector, mbox::model::Mboxes};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

pub fn mbox_source_arg<'a>() -> clap::Arg<'a, 'a> {
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

pub fn mbox_subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "m"])
        .about("Lists all mailboxes")]
}

pub fn mbox_matches(app: &App) -> Result<bool> {
    if let Some(_) = app.arg_matches.subcommand_matches("mailboxes") {
        debug!("mailboxes command matched");

        let mut imap_conn = ImapConnector::new(&app.account)?;
        let names = imap_conn.list_mboxes()?;
        let mboxes = Mboxes::from(&names);
        debug!("found {} mailboxes", mboxes.0.len());
        trace!("mailboxes: {:?}", mboxes);
        app.output.print(mboxes);

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
