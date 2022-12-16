//! Module related to man CLI.
//!
//! This module provides subcommands and a command matcher related to
//! man.

use anyhow::Result;
use clap::{self, ArgMatches, Command};

const CMD_MAN: &str = "man";

/// Man commands.
pub enum Cmd {
    /// Generates man page.
    Generate,
}

/// Man command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    if let Some(_) = m.subcommand_matches(CMD_MAN) {
        return Ok(Some(Cmd::Generate));
    };

    Ok(None)
}

/// Man subcommands.
pub fn subcmds<'a>() -> Vec<clap::Command> {
    vec![Command::new(CMD_MAN)
        .alias("manual")
        .about("Generates the man page.")]
}
