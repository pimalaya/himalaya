//! Email flag CLI module.
//!
//! This module contains the command matcher, the subcommands and the
//! arguments related to the email flag domain.

use anyhow::Result;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use log::{debug, info};

use crate::email;

const ARG_FLAGS: &str = "flag";

const CMD_ADD: &str = "add";
const CMD_REMOVE: &str = "remove";
const CMD_SET: &str = "set";

pub(crate) const CMD_FLAG: &str = "flag";

type Flags = String;

/// Represents the flag commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Add(email::args::Id<'a>, Flags),
    Remove(email::args::Id<'a>, Flags),
    Set(email::args::Id<'a>, Flags),
}

/// Represents the flag command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_ADD) {
        debug!("add flags command matched");
        let id = email::args::parse_id_arg(m);
        let flags = parse_flags_arg(m);
        Some(Cmd::Add(id, flags))
    } else if let Some(m) = m.subcommand_matches(CMD_REMOVE) {
        info!("remove flags command matched");
        let id = email::args::parse_id_arg(m);
        let flags = parse_flags_arg(m);
        Some(Cmd::Remove(id, flags))
    } else if let Some(m) = m.subcommand_matches(CMD_SET) {
        debug!("set flags command matched");
        let id = email::args::parse_id_arg(m);
        let flags = parse_flags_arg(m);
        Some(Cmd::Set(id, flags))
    } else {
        None
    };

    Ok(cmd)
}

/// Represents the flag subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name(CMD_FLAG)
        .aliases(&["flags", "flg"])
        .about("Handles email flags")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name(CMD_ADD)
                .aliases(&["a"])
                .about("Adds flags to an email")
                .arg(email::args::id_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name(CMD_REMOVE)
                .aliases(&["rem", "rm", "r", "delete", "del", "d"])
                .about("Removes flags from an email")
                .arg(email::args::id_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name(CMD_SET)
                .aliases(&["s", "change", "c"])
                .about("Sets flags of an email")
                .arg(email::args::id_arg())
                .arg(flags_arg()),
        )]
}

/// Represents the flags argument.
pub fn flags_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_FLAGS)
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

/// Represents the flags argument parser.
pub fn parse_flags_arg<'a>(matches: &'a ArgMatches<'a>) -> String {
    matches
        .values_of(ARG_FLAGS)
        .unwrap_or_default()
        .collect::<Vec<_>>()
        .join(" ")
}
