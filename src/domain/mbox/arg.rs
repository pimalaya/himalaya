//! Module related to mailboxes arguments.
//!
//! This module provides subcommands and an argument matcher related to mailboxes.

use anyhow::Result;
use clap::{App, Arg, ArgMatches, SubCommand};
use log::debug;

/// Enumeration of all possible matches.
pub enum Match {
    /// List all available mailboxes.
    List,
}

/// Mailboxes arg matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Match>> {
    if let Some(_) = m.subcommand_matches("mailboxes") {
        debug!("mailboxes command matched");
        return Ok(Some(Match::List));
    }

    Ok(None)
}

/// Mailboxes subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "m"])
        .about("Lists all mailboxes")]
}

/// Source mailbox arg.
pub fn source_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Selects a specific mailbox")
        .value_name("MAILBOX")
        .default_value("INBOX")
}

/// Target mailbox arg.
pub fn target_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("target")
        .help("Specifies the targetted mailbox")
        .value_name("TARGET")
}
