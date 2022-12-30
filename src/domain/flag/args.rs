//! Email flag CLI module.
//!
//! This module contains the command matcher, the subcommands and the
//! arguments related to the email flag domain.

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use himalaya_lib::{Flag, Flags};
use log::{debug, info};

use crate::email;

const ARG_FLAGS: &str = "flag";

const CMD_ADD: &str = "add";
const CMD_REMOVE: &str = "remove";
const CMD_SET: &str = "set";

pub(crate) const CMD_FLAG: &str = "flags";

/// Represents the flag commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Add(email::args::Ids<'a>, Flags),
    Remove(email::args::Ids<'a>, Flags),
    Set(email::args::Ids<'a>, Flags),
}

/// Represents the flag command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_ADD) {
        debug!("add flags command matched");
        let ids = email::args::parse_ids_arg(m);
        let flags = parse_flags_arg(m);
        Some(Cmd::Add(ids, flags))
    } else if let Some(m) = m.subcommand_matches(CMD_REMOVE) {
        info!("remove flags command matched");
        let ids = email::args::parse_ids_arg(m);
        let flags = parse_flags_arg(m);
        Some(Cmd::Remove(ids, flags))
    } else if let Some(m) = m.subcommand_matches(CMD_SET) {
        debug!("set flags command matched");
        let ids = email::args::parse_ids_arg(m);
        let flags = parse_flags_arg(m);
        Some(Cmd::Set(ids, flags))
    } else {
        None
    };

    Ok(cmd)
}

/// Represents the flag subcommands.
pub fn subcmds<'a>() -> Vec<Command> {
    vec![Command::new(CMD_FLAG)
        .about("Handles email flags")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new(CMD_ADD)
                .about("Adds flags to an email")
                .arg(email::args::ids_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            Command::new(CMD_REMOVE)
                .aliases(["delete", "del", "d"])
                .about("Removes flags from an email")
                .arg(email::args::ids_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            Command::new(CMD_SET)
                .aliases(["change", "c"])
                .about("Sets flags of an email")
                .arg(email::args::ids_arg())
                .arg(flags_arg()),
        )]
}

/// Represents the flags argument.
pub fn flags_arg() -> Arg {
    Arg::new(ARG_FLAGS)
        .value_name("FLAGS")
        .help("The flags")
        .long_help("The list of flags. It can be one of: seen, answered, flagged, deleted, draft, recent. Other flags are considered custom.")
        .num_args(1..)
        .required(true)
        .last(true)
}

/// Represents the flags argument parser.
pub fn parse_flags_arg(matches: &ArgMatches) -> Flags {
    Flags::from_iter(
        matches
            .get_many::<String>(ARG_FLAGS)
            .unwrap_or_default()
            .map(String::as_str)
            .map(Flag::from),
    )
}
