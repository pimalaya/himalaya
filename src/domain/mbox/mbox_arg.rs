//! Mailbox CLI module.
//!
//! This module provides subcommands, arguments and a command matcher related to mailbox.

use anyhow::Result;
use clap::{App, Arg, ArgMatches, SubCommand};
use log::debug;

/// Represents the Mailbox commands.
pub enum Command {
    /// List all available mailboxes.
    List,
}

/// Defines the mailbox command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Command>> {
    if let Some(_) = m.subcommand_matches("mailboxes") {
        debug!("mailboxes command matched");
        return Ok(Some(Command::List));
    } else {
        Ok(None)
    }
}

/// Define the source mailbox argument.
pub fn source_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Selects a specific mailbox")
        .value_name("SOURCE")
        .default_value("INBOX")
}

/// Defines the target mailbox argument.
pub fn target_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("target")
        .help("Specifies the targetted mailbox")
        .value_name("TARGET")
        .required(true)
}

/// Contains the root mailbox subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "mb", "m"])
        .about("Lists all mailboxes")]
}
