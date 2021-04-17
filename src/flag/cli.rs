use clap::{App, Arg, ArgMatches, SubCommand};
use error_chain::error_chain;
use log::debug;

use crate::{config::model::Account, imap::model::ImapConnector, msg::cli::uid_arg};

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

fn flags_arg<'f>() -> Arg<'f, 'f> {
    Arg::with_name("flags")
        .help("IMAP flags (see https://tools.ietf.org/html/rfc3501#page-11)")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

pub fn flag_subcmds<'f>() -> Vec<App<'f, 'f>> {
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

pub fn flag_matches(account: &Account, mbox: &str, matches: &ArgMatches) -> Result<bool> {
    if let Some(matches) = matches.subcommand_matches("set") {
        debug!("[flag::cli::matches] set command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("[flag::cli::matches] uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("[flag::cli::matches] flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&account)?;
        imap_conn.set_flags(mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = matches.subcommand_matches("add") {
        debug!("[flag::cli::matches] add command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("[flag::cli::matches] uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("[flag::cli::matches] flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&account)?;
        imap_conn.add_flags(mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = matches.subcommand_matches("remove") {
        debug!("[flag::cli::matches] remove command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("[flag::cli::matches] uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("[flag::cli::matches] flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&account)?;
        imap_conn.remove_flags(mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    debug!("[flag::cli::matches] nothing matched");
    Ok(false)
}
