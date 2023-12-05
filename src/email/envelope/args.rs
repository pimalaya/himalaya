//! Email CLI module.
//!
//! This module contains the command matcher, the subcommands and the
//! arguments related to the email domain.

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::ui::table;

const ARG_PAGE: &str = "page";
const ARG_PAGE_SIZE: &str = "page-size";
const CMD_LIST: &str = "list";
const CMD_ENVELOPE: &str = "envelope";

pub type Page = usize;
pub type PageSize = usize;

/// Represents the email commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    List(table::args::MaxTableWidth, Option<PageSize>, Page),
}

/// Email command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_ENVELOPE) {
        if let Some(m) = m.subcommand_matches(CMD_LIST) {
            let max_table_width = table::args::parse_max_width(m);
            let page_size = parse_page_size_arg(m);
            let page = parse_page_arg(m);
            Some(Cmd::List(max_table_width, page_size, page))
        } else {
            Some(Cmd::List(None, None, 0))
        }
    } else {
        None
    };

    Ok(cmd)
}

/// Represents the envelope subcommand.
pub fn subcmd() -> Command {
    Command::new(CMD_ENVELOPE)
        .about("Subcommand to manage envelopes")
        .long_about("Subcommand to manage envelopes like list")
        .subcommands([Command::new(CMD_LIST)
            .alias("lst")
            .about("List envelopes")
            .arg(page_size_arg())
            .arg(page_arg())
            .arg(table::args::max_width())])
}

/// Represents the page size argument.
fn page_size_arg() -> Arg {
    Arg::new(ARG_PAGE_SIZE)
        .help("Page size")
        .long("page-size")
        .short('s')
        .value_name("INT")
}

/// Represents the page size argument parser.
fn parse_page_size_arg(matches: &ArgMatches) -> Option<usize> {
    matches
        .get_one::<String>(ARG_PAGE_SIZE)
        .and_then(|s| s.parse().ok())
}

/// Represents the page argument.
fn page_arg() -> Arg {
    Arg::new(ARG_PAGE)
        .help("Page number")
        .short('p')
        .long("page")
        .value_name("INT")
        .default_value("1")
}

/// Represents the page argument parser.
fn parse_page_arg(matches: &ArgMatches) -> usize {
    matches
        .get_one::<String>(ARG_PAGE)
        .unwrap()
        .parse()
        .ok()
        .map(|page| 1.max(page) - 1)
        .unwrap_or_default()
}
