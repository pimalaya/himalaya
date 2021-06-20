use clap;
use error_chain::error_chain;
use log::debug;

use crate::{ctx::Ctx, imap::model::ImapConnector};
use crate::cli;

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

// ===================
// Main Functions
// ===================
/// Provdes the following **subcommands**:
/// - `himalaya flags set`
/// - `himalaya flags add`
/// - `himalaya flags remove`
pub fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("flags")
        .about("Handles mail-flags")
        .subcommand(
            clap::SubCommand::with_name("set")
                .about("Replaces all message flags")
                .arg(cli::msg::uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            clap::SubCommand::with_name("add")
                .about("Appends flags to a message")
                .arg(cli::msg::uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            clap::SubCommand::with_name("remove")
                .aliases(&["rm"])
                .about("Removes flags from a message")
                .arg(cli::msg::uid_arg())
                .arg(flags_arg()),
        )]
}

/// The appropriate match function for the subcommands listed above in the
/// `subcmds()` function.
pub fn matches(ctx: &Ctx) -> Result<bool> {
    if let Some(matches) = ctx.arg_matches.subcommand_matches("set") {
        debug!("set command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.set_flags(ctx.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = ctx.arg_matches.subcommand_matches("add") {
        debug!("add command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.add_flags(ctx.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = ctx.arg_matches.subcommand_matches("remove") {
        debug!("remove command matched");

        let uid = matches.value_of("uid").unwrap();
        debug!("uid: {}", uid);

        let flags = matches.value_of("flags").unwrap();
        debug!("flags: {}", flags);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.remove_flags(ctx.mbox, uid, flags)?;

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}

// ==================
// Arg functions
// ==================
/// Returns the `<FLAGS>` argument.
fn flags_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("flags")
        .help("IMAP flags (see https://tools.ietf.org/html/rfc3501#page-11)")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

