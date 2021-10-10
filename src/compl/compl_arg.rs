//! Module related to completion CLI.
//!
//! This module provides subcommands and a command matcher related to completion.

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, Shell, SubCommand};
use log::debug;

type OptionShell<'a> = Option<&'a str>;

/// Completion commands.
pub enum Command<'a> {
    /// Generate completion script for the given shell slice.
    Generate(OptionShell<'a>),
}

/// Completion command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Command<'a>>> {
    if let Some(m) = m.subcommand_matches("completion") {
        debug!("completion command matched");
        let shell = m.value_of("shell");
        debug!("shell: `{:?}`", shell);
        return Ok(Some(Command::Generate(shell)));
    };

    Ok(None)
}

/// Completion subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("completion")
        .aliases(&["completions", "compl", "compe", "comp"])
        .about("Generates the completion script for the given shell")
        .args(&[Arg::with_name("shell")
            .possible_values(&Shell::variants()[..])
            .required(true)])]
}
