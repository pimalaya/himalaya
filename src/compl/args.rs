//! Module related to completion CLI.
//!
//! This module provides subcommands and a command matcher related to completion.

use anyhow::Result;
use clap::{value_parser, Arg, ArgMatches, Command};
use clap_complete::Shell;
use log::debug;

const ARG_SHELL: &str = "shell";
const CMD_COMPLETION: &str = "completion";

type SomeShell = Shell;

/// Completion commands.
pub enum Cmd {
    /// Generate completion script for the given shell.
    Generate(SomeShell),
}

/// Completion command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    if let Some(m) = m.subcommand_matches(CMD_COMPLETION) {
        let shell = m.get_one::<Shell>(ARG_SHELL).cloned().unwrap();
        debug!("shell: {:?}", shell);
        return Ok(Some(Cmd::Generate(shell)));
    };

    Ok(None)
}

/// Completion subcommands.
pub fn subcmd() -> Command {
    Command::new(CMD_COMPLETION)
        .about("Generates the completion script for the given shell")
        .args(&[Arg::new(ARG_SHELL)
            .value_parser(value_parser!(Shell))
            .required(true)])
}
