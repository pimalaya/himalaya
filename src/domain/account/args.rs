//! This module provides arguments related to the user account config.

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::info;
use pimalaya_email::folder::sync::Strategy as SyncFoldersStrategy;
use std::collections::HashSet;

use crate::{folder, ui::table};

const ARG_ACCOUNT: &str = "account";
const ARG_DRY_RUN: &str = "dry-run";
const CMD_ACCOUNTS: &str = "accounts";
const CMD_CONFIGURE: &str = "configure";
const CMD_LIST: &str = "list";
const CMD_SYNC: &str = "sync";

type DryRun = bool;

/// Represents the account commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    /// Represents the list accounts command.
    List(table::args::MaxTableWidth),
    /// Represents the sync account command.
    Sync(Option<SyncFoldersStrategy>, DryRun),
    /// Configure the current selected account.
    Configure,
}

/// Represents the account command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_ACCOUNTS) {
        if let Some(m) = m.subcommand_matches(CMD_SYNC) {
            info!("sync account subcommand matched");
            let dry_run = parse_dry_run_arg(m);
            let include = folder::args::parse_include_arg(m);
            let exclude = folder::args::parse_exclude_arg(m);
            let folders_strategy = if let Some(folder) = folder::args::parse_source_arg(m) {
                Some(SyncFoldersStrategy::Include(HashSet::from_iter([
                    folder.to_owned()
                ])))
            } else if !include.is_empty() {
                Some(SyncFoldersStrategy::Include(include.to_owned()))
            } else if !exclude.is_empty() {
                Some(SyncFoldersStrategy::Exclude(exclude))
            } else if folder::args::parse_all_arg(m) {
                Some(SyncFoldersStrategy::All)
            } else {
                None
            };
            Some(Cmd::Sync(folders_strategy, dry_run))
        } else if let Some(m) = m.subcommand_matches(CMD_LIST) {
            info!("list accounts subcommand matched");
            let max_table_width = table::args::parse_max_width(m);
            Some(Cmd::List(max_table_width))
        } else if let Some(_) = m.subcommand_matches(CMD_CONFIGURE) {
            info!("configure account subcommand matched");
            Some(Cmd::Configure)
        } else {
            info!("no account subcommand matched, falling back to subcommand list");
            Some(Cmd::List(None))
        }
    } else {
        None
    };

    Ok(cmd)
}

/// Represents the account subcommand.
pub fn subcmd() -> Command {
    Command::new(CMD_ACCOUNTS)
        .about("Manage accounts")
        .subcommands([
            Command::new(CMD_LIST)
                .about("List all accounts from the config file")
                .arg(table::args::max_width()),
            Command::new(CMD_SYNC)
                .about("Synchronize the given account locally")
                .arg(folder::args::all_arg("Synchronize all folders"))
                .arg(folder::args::include_arg(
                    "Synchronize only the given folders",
                ))
                .arg(folder::args::exclude_arg(
                    "Synchronize all folders except the given ones",
                ))
                .arg(dry_run()),
            Command::new(CMD_CONFIGURE)
                .about("Configure the current selected account")
                .aliases(["config", "conf", "cfg"]),
        ])
}

/// Represents the user account name argument. This argument allows
/// the user to select a different account than the default one.
pub fn arg() -> Arg {
    Arg::new(ARG_ACCOUNT)
        .help("Set the account")
        .long("account")
        .short('a')
        .global(true)
        .value_name("STRING")
}

/// Represents the user account name argument parser.
pub fn parse_arg(matches: &ArgMatches) -> Option<&str> {
    matches.get_one::<String>(ARG_ACCOUNT).map(String::as_str)
}

/// Represents the user account sync dry run flag. This flag allows
/// the user to see the changes of a sync without applying them.
pub fn dry_run() -> Arg {
    Arg::new(ARG_DRY_RUN)
        .help("Do not apply changes of the synchronization")
        .long_help(
            "Do not apply changes of the synchronization.
Changes can be visualized with the RUST_LOG=trace environment variable.",
        )
        .short('d')
        .long("dry-run")
        .action(ArgAction::SetTrue)
}

/// Represents the user account sync dry run flag parser.
pub fn parse_dry_run_arg(m: &ArgMatches) -> bool {
    m.get_flag(ARG_DRY_RUN)
}
