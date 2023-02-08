//! Module related to IMAP CLI.
//!
//! This module provides subcommands and a command matcher related to IMAP.

use anyhow::Result;
use clap::{value_parser, Arg, ArgMatches, Command};
use log::debug;

const ARG_KEEPALIVE: &str = "keepalive";
const CMD_NOTIFY: &str = "notify";
const CMD_WATCH: &str = "watch";

type Keepalive = u64;

/// IMAP commands.
pub enum Cmd {
    /// Start the IMAP notify mode with the give keepalive duration.
    Notify(Keepalive),
    /// Start the IMAP watch mode with the give keepalive duration.
    Watch(Keepalive),
}

/// IMAP command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    if let Some(m) = m.subcommand_matches(CMD_NOTIFY) {
        let keepalive = m.get_one::<u64>(ARG_KEEPALIVE).unwrap();
        debug!("keepalive: {}", keepalive);
        return Ok(Some(Cmd::Notify(*keepalive)));
    }

    if let Some(m) = m.subcommand_matches(CMD_WATCH) {
        let keepalive = m.get_one::<u64>(ARG_KEEPALIVE).unwrap();
        debug!("keepalive: {}", keepalive);
        return Ok(Some(Cmd::Watch(*keepalive)));
    }

    Ok(None)
}

/// IMAP subcommands.
pub fn subcmds<'a>() -> Vec<Command> {
    vec![
        Command::new(CMD_NOTIFY)
            .about("Notifies when new messages arrive in the given folder")
            .alias("idle")
            .arg(keepalive_arg()),
        Command::new(CMD_WATCH)
            .about("Watches IMAP server changes")
            .arg(keepalive_arg()),
    ]
}

/// Represents the keepalive argument.
pub fn keepalive_arg() -> Arg {
    Arg::new(ARG_KEEPALIVE)
        .help("Specifies the keepalive duration.")
        .long("keepalive")
        .short('k')
        .value_name("SECS")
        .default_value("500")
        .value_parser(value_parser!(u64))
}
