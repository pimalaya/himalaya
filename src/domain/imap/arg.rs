//! Module related to IMAP arguments.
//!
//! This module provides subcommands and an argument matcher related to IMAP.

use anyhow::Result;
use clap::{App, ArgMatches};
use log::debug;

/// Enumeration of all possible matches.
pub enum Match {
    /// Start the IMAP notify mode with the give keepalive duration.
    Notify(u64),

    /// Start the IMAP watch mode with the give keepalive duration.
    Watch(u64),
}

/// IMAP arg matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Match>> {
    if let Some(m) = m.subcommand_matches("notify") {
        debug!("notify command matched");
        let keepalive = clap::value_t_or_exit!(m.value_of("keepalive"), u64);
        debug!("keepalive: {}", keepalive);
        return Ok(Some(Match::Notify(keepalive)));
    }

    if let Some(m) = m.subcommand_matches("watch") {
        debug!("watch command matched");
        let keepalive = clap::value_t_or_exit!(m.value_of("keepalive"), u64);
        debug!("keepalive: {}", keepalive);
        return Ok(Some(Match::Watch(keepalive)));
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
