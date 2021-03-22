use clap::{App, Arg, ArgMatches, SubCommand};
use error_chain::error_chain;

use crate::{
    config::{self, Config},
    imap::{self, ImapConnector},
};

error_chain! {
    links {
        Config(config::Error, config::ErrorKind);
        Imap(imap::Error, imap::ErrorKind);
    }
}

fn uid_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("uid")
        .help("Message UID")
        .value_name("UID")
        .required(true)
}

fn flags_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("flags")
        .help("IMAP flags (see https://tools.ietf.org/html/rfc3501#page-11)")
        .value_name("FLAGS")
        .multiple(true)
        .required(true)
}

pub fn flags_subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("flags")
        .aliases(&["flag", "fg"])
        .about("Manages flags")
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
        )
}

pub fn flags_matches(account: Option<&str>, mbox: &str, matches: &ArgMatches) -> Result<()> {
    let config = Config::new_from_file()?;
    let account = config.find_account_by_name(account)?;
    let mut imap_conn = ImapConnector::new(&account)?;

    if let Some(matches) = matches.subcommand_matches("set") {
        let uid = matches.value_of("uid").unwrap();
        let flags = matches.value_of("flags").unwrap();
        imap_conn.set_flags(mbox, uid, flags)?;
    }

    if let Some(matches) = matches.subcommand_matches("add") {
        let uid = matches.value_of("uid").unwrap();
        let flags = matches.value_of("flags").unwrap();
        imap_conn.add_flags(mbox, uid, flags)?;
    }

    if let Some(matches) = matches.subcommand_matches("remove") {
        let uid = matches.value_of("uid").unwrap();
        let flags = matches.value_of("flags").unwrap();
        imap_conn.remove_flags(mbox, uid, flags)?;
    }

    Ok(())
}
