use clap;
use error_chain::error_chain;
use log::debug;

use crate::{ctx::Ctx, imap::model::ImapConnector, msg::cli::uid_arg, flag::model::Flags};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

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

pub fn matches(ctx: &Ctx) -> Result<bool> {
    if let Some(matches) = ctx.arg_matches.subcommand_matches("set") {
        debug!("set command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        let flags = Flags::from(flags);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.set_flags(&ctx.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = ctx.arg_matches.subcommand_matches("add") {
        debug!("add command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        let flags = Flags::from(flags);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.add_flags(&ctx.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = ctx.arg_matches.subcommand_matches("remove") {
        debug!("remove command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        let flags = Flags::from(flags);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.remove_flags(&ctx.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
