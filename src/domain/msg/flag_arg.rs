//! Module related to message flag CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message flag.

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, SubCommand};
use log::{debug, trace};

use crate::domain::msg::msg_arg;

type SeqRange<'a> = &'a str;
type Flags<'a> = Vec<&'a str>;

/// Message flag commands.
pub enum Command<'a> {
    Set(SeqRange<'a>, Flags<'a>),
    Add(SeqRange<'a>, Flags<'a>),
    Remove(SeqRange<'a>, Flags<'a>),
}

/// Message flag command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Command<'a>>> {
    if let Some(m) = m.subcommand_matches("add") {
        debug!("add command matched");
        let seq_range = m.value_of("seq-range").unwrap();
        trace!(r#"seq range: "{:?}""#, seq_range);
        let flags: Vec<&str> = m.values_of("flags").unwrap_or_default().collect();
        trace!(r#"flags: "{:?}""#, flags);
        return Ok(Some(Command::Add(seq_range, flags)));
    }

    if let Some(m) = m.subcommand_matches("set") {
        debug!("set command matched");
        let seq_range = m.value_of("seq-range").unwrap();
        trace!(r#"seq range: "{:?}""#, seq_range);
        let flags: Vec<&str> = m.values_of("flags").unwrap_or_default().collect();
        trace!(r#"flags: "{:?}""#, flags);
        return Ok(Some(Command::Set(seq_range, flags)));
    }

    if let Some(m) = m.subcommand_matches("remove") {
        debug!("remove command matched");
        let seq_range = m.value_of("seq-range").unwrap();
        trace!(r#"seq range: "{:?}""#, seq_range);
        let flags: Vec<&str> = m.values_of("flags").unwrap_or_default().collect();
        trace!(r#"flags: "{:?}""#, flags);
        return Ok(Some(Command::Remove(seq_range, flags)));
    }

    Ok(None)
}

/// Message flag flags argument.
fn flags_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("flags")
        .help("IMAP flags")
        .long_help("IMAP flags. Flags are case-insensitive, and they do not need to be prefixed with `\\`.")
        .value_name("FLAGS…")
        .multiple(true)
        .required(true)
}

/// Message flag subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("flag")
        .aliases(&["flags", "flg"])
        .about("Handles flags")
        .subcommand(
            SubCommand::with_name("add")
                .about("Adds flags to a message")
                .arg(msg_arg::seq_range_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("set")
                .about("Replaces all message flags")
                .arg(msg_arg::seq_range_arg())
                .arg(flags_arg()),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .aliases(&["rm"])
                .about("Removes flags from a message")
                .arg(msg_arg::seq_range_arg())
                .arg(flags_arg()),
        )]
}
