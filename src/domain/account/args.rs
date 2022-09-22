//! This module provides arguments related to the user account config.

use anyhow::Result;
use clap::{App, Arg, ArgMatches, SubCommand};
use log::{debug, info};

use crate::ui::table;

type MaxTableWidth = Option<usize>;

/// Represents the account commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    /// Represents the list accounts command.
    List(MaxTableWidth),
}

/// Represents the account command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    info!(">> account command matcher");

    let cmd = if let Some(m) = m.subcommand_matches("accounts") {
        info!("accounts command matched");

        let max_table_width = m
            .value_of("max-table-width")
            .and_then(|width| width.parse::<usize>().ok());
        debug!("max table width: {:?}", max_table_width);

        Some(Cmd::List(max_table_width))
    } else {
        None
    };

    info!("<< account command matcher");
    Ok(cmd)
}

/// Represents the account subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("accounts")
        .aliases(&["account", "acc", "a"])
        .about("Lists accounts")
        .arg(table::args::max_width())]
}

/// Represents the user account name argument.
/// This argument allows the user to select a different account than
/// the default one.
pub fn name_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("account")
        .long("account")
        .short("a")
        .help("Selects a specific account")
        .value_name("NAME")
}
