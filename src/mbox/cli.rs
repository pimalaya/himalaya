use clap::{self, App, Arg, ArgMatches, SubCommand};
use error_chain::error_chain;

use crate::{config::Config, imap::model::ImapConnector, output::print};

error_chain! {
    links {
        Config(crate::config::Error, crate::config::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
        MsgCli(crate::msg::cli::Error, crate::msg::cli::ErrorKind);
        Output(crate::output::Error, crate::output::ErrorKind);
    }
}

pub fn mbox_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Selects a specific mailbox")
        .value_name("STRING")
        .default_value("INBOX")
}

pub fn mbox_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "m"])
        .about("Lists all mailboxes")]
}

pub fn mbox_matches(matches: &ArgMatches) -> Result<bool> {
    let config = Config::new_from_file()?;
    let account = config.find_account_by_name(matches.value_of("account"))?;
    let output_fmt = matches.value_of("output").unwrap();
    let mut imap_conn = ImapConnector::new(&account)?;

    if let Some(_) = matches.subcommand_matches("mailboxes") {
        let mboxes = imap_conn.list_mboxes()?;
        print(&output_fmt, mboxes)?;
        imap_conn.logout();
        return Ok(true);
    }

    Ok(false)
}
