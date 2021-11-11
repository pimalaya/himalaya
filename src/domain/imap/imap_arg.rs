//! Module related to IMAP CLI.
//!
//! This module provides subcommands and a command matcher related to IMAP.

use anyhow::Result;
use clap::{App, ArgMatches};
use log::debug;

type Keepalive = u64;

/// IMAP commands.
pub enum Command {
    /// Start the IMAP notify mode with the give keepalive duration.
    Notify(Keepalive),

    /// Start the IMAP watch mode with the give keepalive duration.
    Watch(Keepalive),
}

/// IMAP command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Command>> {
    if let Some(m) = m.subcommand_matches("notify") {
        debug!("notify command matched");
        let keepalive = clap::value_t_or_exit!(m.value_of("keepalive"), u64);
        debug!("keepalive: {}", keepalive);
        return Ok(Some(Command::Notify(keepalive)));
    }

    if let Some(m) = m.subcommand_matches("watch") {
        debug!("watch command matched");
        let keepalive = clap::value_t_or_exit!(m.value_of("keepalive"), u64);
        debug!("keepalive: {}", keepalive);
        return Ok(Some(Command::Watch(keepalive)));
    }

    Ok(None)
}

/// IMAP subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![
        clap::SubCommand::with_name("notify")
            .about("Notifies when new messages arrive in the given mailbox")
            .aliases(&["idle"])
            .arg(
                clap::Arg::with_name("keepalive")
                    .help("Specifies the keepalive duration")
                    .short("k")
                    .long("keepalive")
                    .value_name("SECS")
                    .default_value("500"),
            ),
        clap::SubCommand::with_name("watch")
            .about("Watches IMAP server changes")
            .arg(
                clap::Arg::with_name("keepalive")
                    .help("Specifies the keepalive duration")
                    .short("k")
                    .long("keepalive")
                    .value_name("SECS")
                    .default_value("500"),
            ),
    ]
}
