//! Folder CLI module.
//!
//! This module provides subcommands, arguments and a command matcher
//! related to the folder domain.

use std::collections::HashSet;

use anyhow::Result;
use clap::{self, Arg, ArgAction, ArgMatches, Command};
use log::{debug, info};

use crate::ui::table;

const ARG_ALL: &str = "all";
const ARG_EXCLUDE: &str = "exclude";
const ARG_INCLUDE: &str = "include";
const ARG_SOURCE: &str = "source";
const ARG_TARGET: &str = "target";
const CMD_CREATE: &str = "create";
const CMD_DELETE: &str = "delete";
const CMD_EXPUNGE: &str = "expunge";
const CMD_FOLDERS: &str = "folders";
const CMD_LIST: &str = "list";

/// Represents the folder commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    Create,
    List(table::args::MaxTableWidth),
    Expunge,
    Delete,
}

/// Represents the folder command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_FOLDERS) {
        if let Some(_) = m.subcommand_matches(CMD_EXPUNGE) {
            info!("expunge folder subcommand matched");
            Some(Cmd::Expunge)
        } else if let Some(_) = m.subcommand_matches(CMD_CREATE) {
            debug!("create folder command matched");
            Some(Cmd::Create)
        } else if let Some(m) = m.subcommand_matches(CMD_LIST) {
            debug!("list folders command matched");
            let max_table_width = table::args::parse_max_width(m);
            Some(Cmd::List(max_table_width))
        } else if let Some(_) = m.subcommand_matches(CMD_DELETE) {
            debug!("delete folder command matched");
            Some(Cmd::Delete)
        } else {
            info!("no folder subcommand matched, falling back to subcommand list");
            Some(Cmd::List(None))
        }
    } else {
        None
    };

    Ok(cmd)
}

/// Represents the folder subcommand.
pub fn subcmd() -> Command {
    Command::new(CMD_FOLDERS)
        .about("Manage folders")
        .subcommands([
            Command::new(CMD_EXPUNGE).about("Delete emails marked for deletion"),
            Command::new(CMD_CREATE)
                .aliases(["add", "new"])
                .about("Create a new folder"),
            Command::new(CMD_LIST)
                .about("List folders")
                .arg(table::args::max_width()),
            Command::new(CMD_DELETE)
                .aliases(["remove", "rm"])
                .about("Delete a folder with all its emails"),
        ])
}

/// Represents the source folder argument.
pub fn source_arg() -> Arg {
    Arg::new(ARG_SOURCE)
        .help("Set the source folder")
        .long("folder")
        .short('f')
        .global(true)
        .value_name("SOURCE")
}

/// Represents the source folder argument parser.
pub fn parse_source_arg(matches: &ArgMatches) -> Option<&str> {
    matches.get_one::<String>(ARG_SOURCE).map(String::as_str)
}

/// Represents the all folders argument.
pub fn all_arg(help: &'static str) -> Arg {
    Arg::new(ARG_ALL)
        .help(help)
        .long("all-folders")
        .alias("all")
        .short('A')
        .action(ArgAction::SetTrue)
        .conflicts_with(ARG_SOURCE)
        .conflicts_with(ARG_INCLUDE)
        .conflicts_with(ARG_EXCLUDE)
}

/// Represents the all folders argument parser.
pub fn parse_all_arg(m: &ArgMatches) -> bool {
    m.get_flag(ARG_ALL)
}

/// Represents the folders to include argument.
pub fn include_arg(help: &'static str) -> Arg {
    Arg::new(ARG_INCLUDE)
        .help(help)
        .long("include-folder")
        .alias("only")
        .short('F')
        .value_name("FOLDER")
        .num_args(1..)
        .action(ArgAction::Append)
        .conflicts_with(ARG_SOURCE)
        .conflicts_with(ARG_ALL)
        .conflicts_with(ARG_EXCLUDE)
}

/// Represents the folders to include argument parser.
pub fn parse_include_arg(m: &ArgMatches) -> HashSet<String> {
    m.get_many::<String>(ARG_INCLUDE)
        .unwrap_or_default()
        .map(ToOwned::to_owned)
        .collect()
}

/// Represents the folders to exclude argument.
pub fn exclude_arg(help: &'static str) -> Arg {
    Arg::new(ARG_EXCLUDE)
        .help(help)
        .long("exclude-folder")
        .alias("except")
        .short('x')
        .value_name("FOLDER")
        .num_args(1..)
        .action(ArgAction::Append)
        .conflicts_with(ARG_SOURCE)
        .conflicts_with(ARG_ALL)
        .conflicts_with(ARG_INCLUDE)
}

/// Represents the folders to exclude argument parser.
pub fn parse_exclude_arg(m: &ArgMatches) -> HashSet<String> {
    m.get_many::<String>(ARG_EXCLUDE)
        .unwrap_or_default()
        .map(ToOwned::to_owned)
        .collect()
}

/// Represents the target folder argument.
pub fn target_arg() -> Arg {
    Arg::new(ARG_TARGET)
        .help("Specifies the target folder")
        .value_name("TARGET")
        .required(true)
}

/// Represents the target folder argument parser.
pub fn parse_target_arg(matches: &ArgMatches) -> &str {
    matches.get_one::<String>(ARG_TARGET).unwrap().as_str()
}

#[cfg(test)]
mod tests {
    use clap::{error::ErrorKind, Command};

    use super::*;

    #[test]
    fn it_should_match_cmds() {
        let arg = Command::new("himalaya")
            .subcommand(subcmd())
            .get_matches_from(&["himalaya", "folders"]);
        assert_eq!(Some(Cmd::List(None)), matches(&arg).unwrap());

        let arg = Command::new("himalaya")
            .subcommand(subcmd())
            .get_matches_from(&["himalaya", "folders", "list", "--max-width", "20"]);
        assert_eq!(Some(Cmd::List(Some(20))), matches(&arg).unwrap());
    }

    #[test]
    fn it_should_match_source_arg() {
        macro_rules! get_matches_from {
            ($($arg:expr),*) => {
                Command::new("himalaya")
                    .arg(source_arg())
                    .get_matches_from(&["himalaya", $($arg,)*])
            };
        }

        let app = get_matches_from![];
        assert_eq!(None, app.get_one::<String>(ARG_SOURCE).map(String::as_str));

        let app = get_matches_from!["-f", "SOURCE"];
        assert_eq!(
            Some("SOURCE"),
            app.get_one::<String>(ARG_SOURCE).map(String::as_str)
        );

        let app = get_matches_from!["--folder", "SOURCE"];
        assert_eq!(
            Some("SOURCE"),
            app.get_one::<String>(ARG_SOURCE).map(String::as_str)
        );
    }

    #[test]
    fn it_should_match_target_arg() {
        macro_rules! get_matches_from {
            ($($arg:expr),*) => {
                Command::new("himalaya")
                    .arg(target_arg())
                    .try_get_matches_from_mut(&["himalaya", $($arg,)*])
            };
        }

        let app = get_matches_from![];
        assert_eq!(ErrorKind::MissingRequiredArgument, app.unwrap_err().kind());

        let app = get_matches_from!["TARGET"];
        assert_eq!(
            Some("TARGET"),
            app.unwrap()
                .get_one::<String>(ARG_TARGET)
                .map(String::as_str)
        );
    }
}
