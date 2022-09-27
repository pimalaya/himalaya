//! Email flag CLI module.
//!
//! This module provides subcommands, arguments and a command matcher
//! related to the email flag domain.

use anyhow::Result;
use clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};
use log::{debug, info};

use crate::email;

const ARG_FLAGS: &str = "flag";

const CMD_ADD: &str = "add";
const CMD_DEL: &str = "remove";
const CMD_SET: &str = "set";

pub(crate) const CMD_FLAG: &str = "flag";

type Flags = String;

/// Represents the flag commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Add(email::args::Ids<'a>, Flags),
    Set(email::args::Ids<'a>, Flags),
    Del(email::args::Ids<'a>, Flags),
}

/// Represents the flag command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_ADD) {
        debug!("add subcommand matched");
        let ids = email::args::parse_ids_arg(m);
        let flags: String = parse_flags_arg(m);
        Some(Cmd::Add(ids, flags))
    } else if let Some(m) = m.subcommand_matches(CMD_SET) {
        debug!("set subcommand matched");
        let ids = email::args::parse_ids_arg(m);
        let flags: String = parse_flags_arg(m);
        Some(Cmd::Set(ids, flags))
    } else if let Some(m) = m.subcommand_matches(CMD_DEL) {
        info!("remove subcommand matched");
        let ids = email::args::parse_ids_arg(m);
        let flags: String = parse_flags_arg(m);
        Some(Cmd::Del(ids, flags))
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
                .about("Adds email flags")
                .arg(email::args::ids_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name(CMD_SET)
                .aliases(&["s", "change", "c"])
                .about("Sets email flags")
                .arg(email::args::ids_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name(CMD_DEL)
                .aliases(&["rem", "rm", "r", "delete", "del", "d"])
                .about("Removes email flags")
                .arg(email::args::ids_arg())
                .arg(flags_arg()),
        )]
}

/// Represents the flags argument.
pub fn flags_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_FLAGS)
        .long_help("Flags are case-insensitive, and they do not need to be prefixed with `\\`.")
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
