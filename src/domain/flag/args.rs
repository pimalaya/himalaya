//! Message flag CLI module.
//!
//! This module provides subcommands, arguments and a command matcher related to the message flag
//! domain.

use anyhow::Result;
use clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};
use log::{debug, info};

use crate::email;

type SeqRange<'a> = &'a str;
type Flags = String;

/// Represents the flag commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    /// Represents the add flags command.
    Add(SeqRange<'a>, Flags),
    /// Represents the set flags command.
    Set(SeqRange<'a>, Flags),
    /// Represents the remove flags command.
    Remove(SeqRange<'a>, Flags),
}

/// Defines the flag command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    info!("entering message flag command matcher");

    if let Some(m) = m.subcommand_matches("add") {
        info!("add subcommand matched");
        let seq_range = m.value_of("seq-range").unwrap();
        debug!("seq range: {}", seq_range);
        let flags: String = m
            .values_of("flags")
            .unwrap_or_default()
            .collect::<Vec<_>>()
            .join(" ");
        debug!("flags: {:?}", flags);
        return Ok(Some(Cmd::Add(seq_range, flags)));
    }

    if let Some(m) = m.subcommand_matches("set") {
        info!("set subcommand matched");
        let seq_range = m.value_of("seq-range").unwrap();
        debug!("seq range: {}", seq_range);
        let flags: String = m
            .values_of("flags")
            .unwrap_or_default()
            .collect::<Vec<_>>()
            .join(" ");
        debug!("flags: {:?}", flags);
        return Ok(Some(Cmd::Set(seq_range, flags)));
    }

    if let Some(m) = m.subcommand_matches("remove") {
        info!("remove subcommand matched");
        let seq_range = m.value_of("seq-range").unwrap();
        debug!("seq range: {}", seq_range);
        let flags: String = m
            .values_of("flags")
            .unwrap_or_default()
            .collect::<Vec<_>>()
            .join(" ");
        debug!("flags: {:?}", flags);
        return Ok(Some(Cmd::Remove(seq_range, flags)));
    }

    Ok(None)
}

/// Defines the flags argument.
fn flags_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("flags")
        .help("IMAP flags")
        .long_help("IMAP flags. Flags are case-insensitive, and they do not need to be prefixed with `\\`.")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

/// Contains flag subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("flag")
        .aliases(&["flags", "flg"])
        .about("Handles flags")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("add")
                .aliases(&["a"])
                .about("Adds flags to a message")
                .arg(email::args::seq_range_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("set")
                .aliases(&["s", "change", "c"])
                .about("Replaces all message flags")
                .arg(email::args::seq_range_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .aliases(&["rem", "rm", "r", "delete", "del", "d"])
                .about("Removes flags from a message")
                .arg(email::args::seq_range_arg())
                .arg(flags_arg()),
        )]
}
