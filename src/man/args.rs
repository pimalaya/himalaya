//! Module related to man CLI.
//!
//! This module provides subcommands and a command matcher related to
//! man.

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use log::debug;

const ARG_DIR: &str = "dir";
const CMD_MAN: &str = "man";

/// Man commands.
pub enum Cmd<'a> {
    /// Generates all man pages to the specified directory.
    GenerateAll(&'a str),
}

/// Man command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    if let Some(m) = m.subcommand_matches(CMD_MAN) {
        let dir = m.get_one::<String>(ARG_DIR).map(String::as_str).unwrap();
        debug!("directory: {}", dir);
        return Ok(Some(Cmd::GenerateAll(dir)));
    };

    Ok(None)
}

/// Man subcommands.
pub fn subcmd() -> Command {
    Command::new(CMD_MAN)
        .about("Generate all man pages to the given directory")
        .arg(
            Arg::new(ARG_DIR)
                .help("Directory to generate man files in")
                .long_help(
                    "Represents the directory where all man files of
all commands and subcommands should be generated in.",
                )
                .required(true),
        )
}
