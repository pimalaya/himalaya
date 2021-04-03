use clap::{self, App, Arg, ArgMatches, SubCommand};
use error_chain::error_chain;

use crate::{config::model::Config, imap::model::ImapConnector, output::utils::print};

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

pub fn mbox_matches(matches: &ArgMatches) -> Result<bool> {
    let config = Config::new_from_file()?;
    let account = config.find_account_by_name(matches.value_of("account"))?;
    let output_fmt = matches.value_of("output").unwrap();

    if let Some(_) = matches.subcommand_matches("mailboxes") {
        let mut imap_conn = ImapConnector::new(&account)?;
        let mboxes = imap_conn.list_mboxes()?;
        print(&output_fmt, mboxes)?;
        imap_conn.logout();
        return Ok(true);
    }

    Ok(false)
}
