use clap::{App, Arg, ArgMatches, SubCommand};
use error_chain::error_chain;

use crate::msg::cli::uid_arg;
use crate::{config::model::Config, imap::model::ImapConnector};

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

fn flags_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("flags")
        .help("IMAP flags (see https://tools.ietf.org/html/rfc3501#page-11)")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

pub fn flag_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("flags")
        .aliases(&["flag"])
        .about("Handles flags")
        .subcommand(
            SubCommand::with_name("set")
                .aliases(&["s"])
                .about("Replaces all message flags")
                .arg(uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("add")
                .aliases(&["a"])
                .about("Appends flags to a message")
                .arg(uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .aliases(&["rm", "r"])
                .about("Removes flags from a message")
                .arg(uid_arg())
                .arg(flags_arg()),
        )]
}

pub fn flag_matches(matches: &ArgMatches) -> Result<bool> {
    let config = Config::new_from_file()?;
    let account = config.find_account_by_name(matches.value_of("account"))?;
    let mbox = matches.value_of("mailbox").unwrap();
    let mut imap_conn = ImapConnector::new(&account)?;

    if let Some(matches) = matches.subcommand_matches("set") {
        let uid = matches.value_of("uid").unwrap();
        let flags = matches.value_of("flags").unwrap();
        imap_conn.set_flags(mbox, uid, flags)?;
        return Ok(true);
    }

    if let Some(matches) = matches.subcommand_matches("add") {
        let uid = matches.value_of("uid").unwrap();
        let flags = matches.value_of("flags").unwrap();
        imap_conn.add_flags(mbox, uid, flags)?;
        return Ok(true);
    }

    if let Some(matches) = matches.subcommand_matches("remove") {
        let uid = matches.value_of("uid").unwrap();
        let flags = matches.value_of("flags").unwrap();
        imap_conn.remove_flags(mbox, uid, flags)?;
        return Ok(true);
    }

    Ok(false)
}
