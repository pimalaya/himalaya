use anyhow::Result;
use clap;
use log::debug;

use crate::{
    domain::{imap::service::ImapServiceInterface, msg::arg::uid_arg},
    flag::model::Flags,
};

fn flags_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("flags")
        .help("IMAP flags (see https://tools.ietf.org/html/rfc3501#page-11). Just write the flag name without the backslash. Example: --flags \"Seen Answered\"")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

pub fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("flags")
        .about("Handles flags")
        .subcommand(
            clap::SubCommand::with_name("set")
                .about("Replaces all message flags")
                .arg(uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            clap::SubCommand::with_name("add")
                .about("Appends flags to a message")
                .arg(uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            clap::SubCommand::with_name("remove")
                .aliases(&["rm"])
                .about("Removes flags from a message")
                .arg(uid_arg())
                .arg(flags_arg()),
        )]
}

pub fn matches<ImapService: ImapServiceInterface>(
    arg_matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    if let Some(matches) = arg_matches.subcommand_matches("set") {
        debug!("set command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        let flags = Flags::from(flags);

        imap.set_flags(uid, flags)?;
        imap.logout()?;
        return Ok(true);
    }

    if let Some(matches) = arg_matches.subcommand_matches("add") {
        debug!("add command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        let flags = Flags::from(flags);

        imap.add_flags(uid, flags)?;
        imap.logout()?;
        return Ok(true);
    }

    if let Some(matches) = arg_matches.subcommand_matches("remove") {
        debug!("remove command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        let flags = Flags::from(flags);

        imap.remove_flags(uid, flags)?;
        imap.logout()?;
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
