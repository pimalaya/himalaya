//! Email CLI module.
//!
//! This module contains the command matcher, the subcommands and the
//! arguments related to the email domain.

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::{email, flag, folder, tpl, ui::table};

const ARG_CRITERIA: &str = "criterion";
const ARG_HEADERS: &str = "header";
const ARG_ID: &str = "id";
const ARG_MIME_TYPE: &str = "mime-type";
const ARG_PAGE: &str = "page";
const ARG_PAGE_SIZE: &str = "page-size";
const ARG_QUERY: &str = "query";
const ARG_RAW: &str = "raw";
const ARG_REPLY_ALL: &str = "reply-all";
const ARG_SANITIZE: &str = "sanitize";
const CMD_ATTACHMENTS: &str = "attachments";
const CMD_COPY: &str = "copy";
const CMD_DELETE: &str = "delete";
const CMD_FORWARD: &str = "forward";
const CMD_LIST: &str = "list";
const CMD_MOVE: &str = "move";
const CMD_READ: &str = "read";
const CMD_REPLY: &str = "reply";
const CMD_SAVE: &str = "save";
const CMD_SEARCH: &str = "search";
const CMD_SEND: &str = "send";
const CMD_SORT: &str = "sort";
const CMD_WRITE: &str = "write";

pub type All = bool;
pub type Criteria = String;
pub type Folder<'a> = &'a str;
pub type Headers<'a> = Vec<&'a str>;
pub type Id<'a> = &'a str;
pub type Page = usize;
pub type PageSize = usize;
pub type Query = String;
pub type Raw = bool;
pub type RawEmail = String;
pub type Sanitize = bool;
pub type TextMime<'a> = &'a str;

/// Represents the email commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Attachments(Id<'a>),
    Copy(Id<'a>, Folder<'a>),
    Delete(Id<'a>),
    Flag(Option<flag::args::Cmd<'a>>),
    Forward(Id<'a>, tpl::args::Headers<'a>, tpl::args::Body<'a>),
    List(table::args::MaxTableWidth, Option<PageSize>, Page),
    Move(Id<'a>, Folder<'a>),
    Read(Id<'a>, TextMime<'a>, Sanitize, Raw, Headers<'a>),
    Reply(Id<'a>, All, tpl::args::Headers<'a>, tpl::args::Body<'a>),
    Save(RawEmail),
    Search(Query, table::args::MaxTableWidth, Option<PageSize>, Page),
    Send(RawEmail),
    Sort(
        Criteria,
        Query,
        table::args::MaxTableWidth,
        Option<PageSize>,
        Page,
    ),
    Tpl(Option<tpl::args::Cmd<'a>>),
    Write(tpl::args::Headers<'a>, tpl::args::Body<'a>),
}

/// Email command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_ATTACHMENTS) {
        let id = parse_id_arg(m);
        Cmd::Attachments(id)
    } else if let Some(m) = m.subcommand_matches(CMD_COPY) {
        let id = parse_id_arg(m);
        let folder = folder::args::parse_target_arg(m);
        Cmd::Copy(id, folder)
    } else if let Some(m) = m.subcommand_matches(CMD_DELETE) {
        let id = parse_id_arg(m);
        Cmd::Delete(id)
    } else if let Some(m) = m.subcommand_matches(flag::args::CMD_FLAG) {
        Cmd::Flag(flag::args::matches(m)?)
    } else if let Some(m) = m.subcommand_matches(CMD_FORWARD) {
        let id = parse_id_arg(m);
        let headers = tpl::args::parse_headers_arg(m);
        let body = tpl::args::parse_body_arg(m);
        Cmd::Forward(id, headers, body)
    } else if let Some(m) = m.subcommand_matches(CMD_LIST) {
        let max_table_width = table::args::parse_max_width(m);
        let page_size = parse_page_size_arg(m);
        let page = parse_page_arg(m);
        Cmd::List(max_table_width, page_size, page)
    } else if let Some(m) = m.subcommand_matches(CMD_MOVE) {
        let id = parse_id_arg(m);
        let folder = folder::args::parse_target_arg(m);
        Cmd::Move(id, folder)
    } else if let Some(m) = m.subcommand_matches(CMD_READ) {
        let id = parse_id_arg(m);
        let mime = parse_mime_type_arg(m);
        let sanitize = parse_sanitize_flag(m);
        let raw = parse_raw_flag(m);
        let headers = parse_headers_arg(m);
        Cmd::Read(id, mime, sanitize, raw, headers)
    } else if let Some(m) = m.subcommand_matches(CMD_REPLY) {
        let id = parse_id_arg(m);
        let all = parse_reply_all_flag(m);
        let headers = tpl::args::parse_headers_arg(m);
        let body = tpl::args::parse_body_arg(m);
        Cmd::Reply(id, all, headers, body)
    } else if let Some(m) = m.subcommand_matches(CMD_SAVE) {
        let email = parse_raw_arg(m);
        Cmd::Save(email)
    } else if let Some(m) = m.subcommand_matches(CMD_SEARCH) {
        let max_table_width = table::args::parse_max_width(m);
        let page_size = parse_page_size_arg(m);
        let page = parse_page_arg(m);
        let query = parse_query_arg(m);
        Cmd::Search(query, max_table_width, page_size, page)
    } else if let Some(m) = m.subcommand_matches(CMD_SORT) {
        let max_table_width = table::args::parse_max_width(m);
        let page_size = parse_page_size_arg(m);
        let page = parse_page_arg(m);
        let criteria = parse_criteria_arg(m);
        let query = parse_query_arg(m);
        Cmd::Sort(criteria, query, max_table_width, page_size, page)
    } else if let Some(m) = m.subcommand_matches(CMD_SEND) {
        let email = parse_raw_arg(m);
        Cmd::Send(email)
    } else if let Some(m) = m.subcommand_matches(tpl::args::CMD_TPL) {
        Cmd::Tpl(tpl::args::matches(m)?)
    } else if let Some(m) = m.subcommand_matches(CMD_WRITE) {
        let headers = tpl::args::parse_headers_arg(m);
        let body = tpl::args::parse_body_arg(m);
        Cmd::Write(headers, body)
    } else {
        Cmd::List(None, None, 0)
    };

    Ok(Some(cmd))
}

/// Represents the email subcommands.
pub fn subcmds() -> Vec<Command> {
    vec![
        flag::args::subcmds(),
        tpl::args::subcmds(),
        vec![
            Command::new(CMD_ATTACHMENTS)
                .aliases(&["attachment", "attach", "att", "at"])
                .about("Downloads all attachments of the targeted email")
                .arg(email::args::id_arg()),
            Command::new(CMD_LIST)
                .aliases(&["lst", "l"])
                .about("Lists all emails")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table::args::max_width()),
            Command::new(CMD_SEARCH)
                .aliases(&["s", "query", "q"])
                .about("Lists emails matching the given query")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table::args::max_width())
                .arg(query_arg()),
            Command::new(CMD_SORT)
                .about("Sorts emails by the given criteria and matching the given query")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table::args::max_width())
                .arg(criteria_arg())
                .arg(query_arg()),
            Command::new(CMD_WRITE)
                .about("Writes a new email")
                .aliases(&["w", "new", "n"])
                .args(&tpl::args::args()),
            Command::new(CMD_SEND)
                .about("Sends a raw email")
                .arg(raw_arg()),
            Command::new(CMD_SAVE)
                .about("Saves a raw email")
                .arg(raw_arg()),
            Command::new(CMD_READ)
                .about("Reads text bodies of an email")
                .arg(id_arg())
                .arg(mime_type_arg())
                .arg(sanitize_flag())
                .arg(raw_flag())
                .arg(headers_arg()),
            Command::new(CMD_REPLY)
                .aliases(&["rep", "r"])
                .about("Answers to an email")
                .arg(id_arg())
                .arg(reply_all_flag()),
            Command::new(CMD_FORWARD)
                .aliases(&["fwd", "f"])
                .about("Forwards an email")
                .arg(id_arg()),
            Command::new(CMD_COPY)
                .aliases(&["cp", "c"])
                .about("Copies an email to the targeted folder")
                .arg(id_arg())
                .arg(folder::args::target_arg()),
            Command::new(CMD_MOVE)
                .aliases(&["mv"])
                .about("Moves an email to the targeted folder")
                .arg(id_arg())
                .arg(folder::args::target_arg()),
            Command::new(CMD_DELETE)
                .aliases(&["del", "d", "remove", "rm"])
                .about("Deletes an email")
                .arg(id_arg()),
        ],
    ]
    .concat()
}

/// Represents the email id argument.
pub fn id_arg() -> Arg {
    Arg::new(ARG_ID)
        .help("Specifies the target email")
        .value_name("ID")
        .required(true)
}

/// Represents the email id argument parser.
pub fn parse_id_arg(matches: &ArgMatches) -> &str {
    matches.get_one::<String>(ARG_ID).unwrap()
}

/// Represents the email sort criteria argument.
pub fn criteria_arg<'a>() -> Arg {
    Arg::new(ARG_CRITERIA)
        .long("criterion")
        .short('c')
        .help("Email sorting preferences")
        .value_name("CRITERION:ORDER")
        .num_args(1..)
        .required(true)
        .value_parser([
            "arrival",
            "arrival:asc",
            "arrival:desc",
            "cc",
            "cc:asc",
            "cc:desc",
            "date",
            "date:asc",
            "date:desc",
            "from",
            "from:asc",
            "from:desc",
            "size",
            "size:asc",
            "size:desc",
            "subject",
            "subject:asc",
            "subject:desc",
            "to",
            "to:asc",
            "to:desc",
        ])
}

/// Represents the email sort criteria argument parser.
pub fn parse_criteria_arg(matches: &ArgMatches) -> String {
    matches
        .get_many::<String>(ARG_CRITERIA)
        .unwrap_or_default()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Represents the email reply all argument.
pub fn reply_all_flag() -> Arg {
    Arg::new(ARG_REPLY_ALL)
        .help("Includes all recipients")
        .long("all")
        .short('A')
}

/// Represents the email reply all argument parser.
pub fn parse_reply_all_flag(matches: &ArgMatches) -> bool {
    matches.contains_id(ARG_REPLY_ALL)
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

/// Represents the email headers argument.
pub fn headers_arg() -> Arg {
    Arg::new(ARG_HEADERS)
        .help("Shows additional headers with the email")
        .long("header")
        .short('H')
        .value_name("STRING")
        .num_args(..)
}

/// Represents the email headers argument parser.
pub fn parse_headers_arg(matches: &ArgMatches) -> Vec<&str> {
    matches
        .get_many::<String>(ARG_HEADERS)
        .unwrap_or_default()
        .map(String::as_str)
        .collect::<Vec<_>>()
}

/// Represents the sanitize flag.
pub fn sanitize_flag() -> Arg {
    Arg::new(ARG_SANITIZE)
        .help("Sanitizes text bodies")
        .long("sanitize")
        .short('s')
}

/// Represents the raw flag.
pub fn raw_flag() -> Arg {
    Arg::new(ARG_RAW)
        .help("Returns raw version of email")
        .long("raw")
        .short('r')
}

/// Represents the sanitize flag parser.
pub fn parse_sanitize_flag(matches: &ArgMatches) -> bool {
    matches.contains_id(ARG_SANITIZE)
}

/// Represents the raw flag parser.
pub fn parse_raw_flag(matches: &ArgMatches) -> bool {
    matches.contains_id(ARG_RAW)
}

/// Represents the email raw argument.
pub fn raw_arg() -> Arg {
    Arg::new(ARG_RAW).raw(true)
}

/// Represents the email raw argument parser.
pub fn parse_raw_arg(matches: &ArgMatches) -> String {
    matches
        .get_one::<String>(ARG_RAW)
        .cloned()
        .unwrap_or_default()
}

/// Represents the email MIME type argument.
pub fn mime_type_arg() -> Arg {
    Arg::new(ARG_MIME_TYPE)
        .help("MIME type to use")
        .short('t')
        .long("mime-type")
        .value_name("MIME")
        .value_parser(["plain", "html"])
        .default_value("plain")
}

/// Represents the email MIME type argument parser.
pub fn parse_mime_type_arg(matches: &ArgMatches) -> &str {
    matches.get_one::<String>(ARG_MIME_TYPE).unwrap()
}

/// Represents the email query argument.
pub fn query_arg() -> Arg {
    Arg::new(ARG_QUERY)
        .long_help("The query system depends on the backend, see the wiki for more details")
        .value_name("QUERY")
        .num_args(1..)
        .required(true)
}

/// Represents the email query argument parser.
pub fn parse_query_arg(matches: &ArgMatches) -> String {
    matches
        .get_many::<String>(ARG_QUERY)
        .unwrap_or_default()
        .fold((false, vec![]), |(escape, mut cmds), cmd| {
            match (cmd.as_str(), escape) {
                // Next command is an arg and needs to be escaped
                ("subject", _) | ("body", _) | ("text", _) => {
                    cmds.push(cmd.to_string());
                    (true, cmds)
                }
                // Escaped arg commands
                (_, true) => {
                    cmds.push(format!("\"{}\"", cmd));
                    (false, cmds)
                }
                // Regular commands
                (_, false) => {
                    cmds.push(cmd.to_string());
                    (false, cmds)
                }
            }
        })
        .1
        .join(" ")
}
