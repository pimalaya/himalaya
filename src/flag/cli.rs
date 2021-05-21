use clap;
use error_chain::error_chain;
use log::debug;

use crate::{app::App, imap::model::ImapConnector, msg::cli::uid_arg};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

fn flags_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("flags")
        .help("IMAP flags (see https://tools.ietf.org/html/rfc3501#page-11)")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

pub fn flag_subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("flags")
        .aliases(&["flag"])
        .about("Handles flags")
        .subcommand(
            clap::SubCommand::with_name("set")
                .aliases(&["s"])
                .about("Replaces all message flags")
                .arg(uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            clap::SubCommand::with_name("add")
                .aliases(&["a"])
                .about("Appends flags to a message")
                .arg(uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            clap::SubCommand::with_name("remove")
                .aliases(&["rm", "r"])
                .about("Removes flags from a message")
                .arg(uid_arg())
                .arg(flags_arg()),
        )]
}

pub fn flag_matches(app: &App) -> Result<bool> {
    if let Some(matches) = app.arg_matches.subcommand_matches("set") {
        debug!("set command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&app.account)?;
        imap_conn.set_flags(app.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = app.arg_matches.subcommand_matches("add") {
        debug!("add command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&app.account)?;
        imap_conn.add_flags(app.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = app.arg_matches.subcommand_matches("remove") {
        debug!("remove command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&app.account)?;
        imap_conn.remove_flags(app.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
