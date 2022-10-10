//! Folder CLI module.
//!
//! This module provides subcommands, arguments and a command matcher
//! related to the folder domain.

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, SubCommand};
use log::debug;

use crate::ui::table;

const ARG_SOURCE: &str = "source";
const ARG_TARGET: &str = "target";
const CMD_FOLDERS: &str = "folders";

/// Represents the folder commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    List(table::args::MaxTableWidth),
}

/// Represents the folder command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_FOLDERS) {
        debug!("folders command matched");
        let max_table_width = table::args::parse_max_width(m);
        Some(Cmd::List(max_table_width))
    } else {
        None
    };

    Ok(cmd)
}

/// Represents folder subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name(CMD_FOLDERS)
        .aliases(&[
            "folder",
            "fold",
            "fo",
            "mailboxes",
            "mailbox",
            "mboxes",
            "mbox",
            "mb",
            "m",
        ])
        .about("Lists folders")
        .arg(table::args::max_width())]
}

/// Represents the source folder argument.
pub fn source_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_SOURCE)
        .short("f")
        .long("folder")
        .help("Specifies the source folder")
        .value_name("SOURCE")
        .default_value("inbox")
}

/// Represents the source folder argument parser.
pub fn parse_source_arg<'a>(matches: &'a ArgMatches<'a>) -> &'a str {
    matches.value_of(ARG_SOURCE).unwrap()
}

/// Represents the target folder argument.
pub fn target_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_TARGET)
        .help("Specifies the target folder")
        .value_name("TARGET")
        .required(true)
}

/// Represents the target folder argument parser.
pub fn parse_target_arg<'a>(matches: &'a ArgMatches<'a>) -> &'a str {
    matches.value_of(ARG_TARGET).unwrap()
}

#[cfg(test)]
mod tests {
    use clap::{App, ErrorKind};

    use super::*;

    #[test]
    fn it_should_match_cmds() {
        let arg = App::new("himalaya")
            .subcommands(subcmds())
            .get_matches_from(&["himalaya", "folders"]);
        assert_eq!(Some(Cmd::List(None)), matches(&arg).unwrap());

        let arg = App::new("himalaya")
            .subcommands(subcmds())
            .get_matches_from(&["himalaya", "folders", "--max-width", "20"]);
        assert_eq!(Some(Cmd::List(Some(20))), matches(&arg).unwrap());
    }

    #[test]
    fn it_should_match_aliases() {
        macro_rules! get_matches_from {
            ($alias:expr) => {
                App::new("himalaya")
                    .subcommands(subcmds())
                    .get_matches_from(&["himalaya", $alias])
                    .subcommand_name()
            };
        }

        assert_eq!(Some("folders"), get_matches_from!["folders"]);
        assert_eq!(Some("folders"), get_matches_from!["folder"]);
        assert_eq!(Some("folders"), get_matches_from!["fold"]);
        assert_eq!(Some("folders"), get_matches_from!["fo"]);
    }

    #[test]
    fn it_should_match_source_arg() {
        macro_rules! get_matches_from {
            ($($arg:expr),*) => {
                App::new("himalaya")
                    .arg(source_arg())
                    .get_matches_from(&["himalaya", $($arg,)*])
            };
        }

        let app = get_matches_from![];
        assert_eq!(Some("inbox"), app.value_of("source"));

        let app = get_matches_from!["-f", "SOURCE"];
        assert_eq!(Some("SOURCE"), app.value_of("source"));

        let app = get_matches_from!["--folder", "SOURCE"];
        assert_eq!(Some("SOURCE"), app.value_of("source"));
    }

    #[test]
    fn it_should_match_target_arg() {
        macro_rules! get_matches_from {
            ($($arg:expr),*) => {
                App::new("himalaya")
                    .arg(target_arg())
                    .get_matches_from_safe(&["himalaya", $($arg,)*])
            };
        }

        let app = get_matches_from![];
        assert_eq!(ErrorKind::MissingRequiredArgument, app.unwrap_err().kind);

        let app = get_matches_from!["TARGET"];
        assert_eq!(Some("TARGET"), app.unwrap().value_of("target"));
    }
}
