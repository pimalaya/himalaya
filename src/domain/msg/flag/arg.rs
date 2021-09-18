//! Module related to message flag CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message flag.

use anyhow::Result;
use clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};
use log::debug;

use crate::domain::msg;

type Uid<'a> = &'a str;
type Flags<'a> = &'a str;

/// Message flag commands.
pub enum Command<'a> {
    Set(Uid<'a>, Flags<'a>),
    Add(Uid<'a>, Flags<'a>),
    Remove(Uid<'a>, Flags<'a>),
}

/// Message flag command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Command<'a>>> {
    if let Some(m) = m.subcommand_matches("set") {
        debug!("set command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", uid);
        let flags = m.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        return Ok(Some(Command::Set(uid, flags)));
    }

    if let Some(m) = m.subcommand_matches("add") {
        debug!("add command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", uid);
        let flags = m.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        return Ok(Some(Command::Add(uid, flags)));
    }

    if let Some(m) = m.subcommand_matches("remove") {
        debug!("remove command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", uid);
        let flags = m.value_of("flags").unwrap();
        debug!("flags: {}", flags);
        return Ok(Some(Command::Remove(uid, flags)));
    }

    Ok(None)
}

/// Message flag flags argument.
fn flags_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("flags")
        .help("IMAP flags (see https://tools.ietf.org/html/rfc3501#page-11). Just write the flag name without the backslash. Example: --flags \"Seen Answered\"")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

/// Message flag subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("flags")
        .about("Handles flags")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("set")
                .about("Replaces all message flags")
                .arg(msg::arg::uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("Appends flags to a message")
                .arg(msg::arg::uid_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .aliases(&["rm"])
                .about("Removes flags from a message")
                .arg(msg::arg::uid_arg())
                .arg(flags_arg()),
        )]
}
