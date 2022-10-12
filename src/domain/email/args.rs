//! Module related to email CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to email.

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, SubCommand};
use himalaya_lib::email::TplOverride;
use log::{debug, trace};

use crate::{email, flag, folder, tpl, ui::table};

const ARG_ATTACHMENTS: &str = "attachment";
const ARG_CRITERIA: &str = "criterion";
const ARG_ENCRYPT: &str = "encrypt";
const ARG_HEADERS: &str = "header";
const ARG_ID: &str = "id";
const ARG_IDS: &str = "ids";
const ARG_MIME_TYPE: &str = "mime-type";
const ARG_PAGE: &str = "page";
const ARG_PAGE_SIZE: &str = "page-size";
const ARG_QUERY: &str = "query";
const ARG_RAW: &str = "raw";
const ARG_REPLY_ALL: &str = "reply-all";
const ARG_SANITIZE: &str = "sanitize";
const CMD_ATTACHMENTS: &str = "attachments";
const CMD_COPY: &str = "copy";
const CMD_DEL: &str = "delete";
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

type Criteria = String;
type Encrypt = bool;
type Folder<'a> = &'a str;
type Page = usize;
type PageSize = usize;
type Query = String;
type Sanitize = bool;
type Raw = bool;
type RawEmail<'a> = &'a str;
type TextMime<'a> = &'a str;

pub(crate) type All = bool;
pub(crate) type Attachments<'a> = Vec<&'a str>;
pub(crate) type Headers<'a> = Vec<&'a str>;
pub(crate) type Id<'a> = &'a str;
pub(crate) type Ids<'a> = &'a str;

/// Represents the email commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Attachments(Id<'a>),
    Copy(Id<'a>, Folder<'a>),
    Delete(Ids<'a>),
    Forward(Id<'a>, Attachments<'a>, Encrypt),
    List(table::args::MaxTableWidth, Option<PageSize>, Page),
    Move(Id<'a>, Folder<'a>),
    Read(Id<'a>, TextMime<'a>, Sanitize, Raw, Headers<'a>),
    Reply(Id<'a>, All, Attachments<'a>, Encrypt),
    Save(RawEmail<'a>),
    Search(Query, table::args::MaxTableWidth, Option<PageSize>, Page),
    Send(RawEmail<'a>),
    Sort(
        Criteria,
        Query,
        table::args::MaxTableWidth,
        Option<PageSize>,
        Page,
    ),
    Write(TplOverride<'a>, Attachments<'a>, Encrypt),

    Flag(Option<flag::args::Cmd<'a>>),
    Tpl(Option<tpl::args::Cmd<'a>>),
}

/// Email command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    trace!("matches: {:?}", m);

    let cmd = if let Some(m) = m.subcommand_matches(CMD_ATTACHMENTS) {
        debug!("attachments command matched");
        let id = parse_id_arg(m);
        Cmd::Attachments(id)
    } else if let Some(m) = m.subcommand_matches(CMD_COPY) {
        debug!("copy command matched");
        let id = parse_id_arg(m);
        let folder = folder::args::parse_target_arg(m);
        Cmd::Copy(id, folder)
    } else if let Some(m) = m.subcommand_matches(CMD_DEL) {
        debug!("delete command matched");
        let ids = parse_ids_arg(m);
        Cmd::Delete(ids)
    } else if let Some(m) = m.subcommand_matches(CMD_FORWARD) {
        debug!("forward command matched");
        let id = parse_id_arg(m);
        let attachments = parse_attachments_arg(m);
        let encrypt = parse_encrypt_flag(m);
        Cmd::Forward(id, attachments, encrypt)
    } else if let Some(m) = m.subcommand_matches(CMD_LIST) {
        debug!("list command matched");
        let max_table_width = table::args::parse_max_width(m);
        let page_size = parse_page_size_arg(m);
        let page = parse_page_arg(m);
        Cmd::List(max_table_width, page_size, page)
    } else if let Some(m) = m.subcommand_matches(CMD_MOVE) {
        debug!("move command matched");
        let id = parse_id_arg(m);
        let folder = folder::args::parse_target_arg(m);
        Cmd::Move(id, folder)
    } else if let Some(m) = m.subcommand_matches(CMD_READ) {
        debug!("read command matched");
        let id = parse_id_arg(m);
        let mime = parse_mime_type_arg(m);
        let sanitize = parse_sanitize_flag(m);
        let raw = parse_raw_flag(m);
        let headers = parse_headers_arg(m);
        Cmd::Read(id, mime, sanitize, raw, headers)
    } else if let Some(m) = m.subcommand_matches(CMD_REPLY) {
        debug!("reply command matched");
        let id = parse_id_arg(m);
        let all = parse_reply_all_flag(m);
        let attachments = parse_attachments_arg(m);
        let encrypt = parse_encrypt_flag(m);
        Cmd::Reply(id, all, attachments, encrypt)
    } else if let Some(m) = m.subcommand_matches(CMD_SAVE) {
        debug!("save command matched");
        let email = parse_raw_arg(m);
        Cmd::Save(email)
    } else if let Some(m) = m.subcommand_matches(CMD_SEARCH) {
        debug!("search command matched");
        let max_table_width = table::args::parse_max_width(m);
        let page_size = parse_page_size_arg(m);
        let page = parse_page_arg(m);
        let query = parse_query_arg(m);
        Cmd::Search(query, max_table_width, page_size, page)
    } else if let Some(m) = m.subcommand_matches(CMD_SORT) {
        debug!("sort command matched");
        let max_table_width = table::args::parse_max_width(m);
        let page_size = parse_page_size_arg(m);
        let page = parse_page_arg(m);
        let criteria = parse_criteria_arg(m);
        let query = parse_query_arg(m);
        Cmd::Sort(criteria, query, max_table_width, page_size, page)
    } else if let Some(m) = m.subcommand_matches(CMD_SEND) {
        debug!("send command matched");
        let email = parse_raw_arg(m);
        Cmd::Send(email)
    } else if let Some(m) = m.subcommand_matches(CMD_WRITE) {
        debug!("write command matched");
        let attachments = parse_attachments_arg(m);
        let encrypt = parse_encrypt_flag(m);
        let tpl = tpl::args::parse_override_arg(m);
        Cmd::Write(tpl, attachments, encrypt)
    } else if let Some(m) = m.subcommand_matches(tpl::args::CMD_TPL) {
        Cmd::Tpl(tpl::args::matches(m)?)
    } else if let Some(m) = m.subcommand_matches(flag::args::CMD_FLAG) {
        Cmd::Flag(flag::args::matches(m)?)
    } else {
        debug!("default list command matched");
        Cmd::List(None, None, 0)
    };

    Ok(Some(cmd))
}

/// Represents the email subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![
        flag::args::subcmds(),
        tpl::args::subcmds(),
        vec![
            SubCommand::with_name(CMD_ATTACHMENTS)
                .aliases(&["attachment", "attach", "att", "at", "a"])
                .about("Downloads all attachments of the targeted email")
                .arg(email::args::id_arg()),
            SubCommand::with_name(CMD_LIST)
                .aliases(&["lst", "l"])
                .about("Lists all emails")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table::args::max_width()),
            SubCommand::with_name(CMD_SEARCH)
                .aliases(&["s", "query", "q"])
                .about("Lists emails matching the given query")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table::args::max_width())
                .arg(query_arg()),
            SubCommand::with_name(CMD_SORT)
                .about("Sorts emails by the given criteria and matching the given query")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table::args::max_width())
                .arg(criteria_arg())
                .arg(query_arg()),
            SubCommand::with_name(CMD_WRITE)
                .about("Writes a new email")
                .aliases(&["w", "new", "n"])
                .args(&tpl::args::args())
                .arg(attachments_arg())
                .arg(encrypt_flag()),
            SubCommand::with_name(CMD_SEND)
                .about("Sends a raw email")
                .arg(raw_arg()),
            SubCommand::with_name(CMD_SAVE)
                .about("Saves a raw email")
                .arg(raw_arg()),
            SubCommand::with_name(CMD_READ)
                .about("Reads text bodies of an email")
                .arg(id_arg())
                .arg(mime_type_arg())
                .arg(sanitize_flag())
                .arg(raw_flag())
                .arg(headers_arg()),
            SubCommand::with_name(CMD_REPLY)
                .aliases(&["rep", "r"])
                .about("Answers to an email")
                .arg(id_arg())
                .arg(reply_all_flag())
                .arg(attachments_arg())
                .arg(encrypt_flag()),
            SubCommand::with_name(CMD_FORWARD)
                .aliases(&["fwd", "f"])
                .about("Forwards an email")
                .arg(id_arg())
                .arg(attachments_arg())
                .arg(encrypt_flag()),
            SubCommand::with_name(CMD_COPY)
                .aliases(&["cp", "c"])
                .about("Copies an email to the targeted folder")
                .arg(id_arg())
                .arg(folder::args::target_arg()),
            SubCommand::with_name(CMD_MOVE)
                .aliases(&["mv"])
                .about("Moves an email to the targeted folder")
                .arg(id_arg())
                .arg(folder::args::target_arg()),
            SubCommand::with_name(CMD_DEL)
                .aliases(&["del", "d", "remove", "rm"])
                .about("Deletes an email")
                .arg(ids_arg()),
        ],
    ]
    .concat()
}

/// Represents the email id argument.
pub fn id_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_ID)
        .help("Specifies the target email")
        .value_name("ID")
        .required(true)
}

/// Represents the email id argument parser.
pub fn parse_id_arg<'a>(matches: &'a ArgMatches<'a>) -> &'a str {
    matches.value_of(ARG_ID).unwrap()
}

/// Represents the email sort criteria argument.
pub fn criteria_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_CRITERIA)
        .long("criterion")
        .short("c")
        .help("Email sorting preferences")
        .value_name("CRITERION:ORDER")
        .takes_value(true)
        .multiple(true)
        .required(true)
        .possible_values(&[
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
pub fn parse_criteria_arg<'a>(matches: &'a ArgMatches<'a>) -> String {
    matches
        .values_of(ARG_CRITERIA)
        .unwrap_or_default()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Represents the email ids argument.
pub fn ids_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_IDS)
        .help("Specifies the target email(s)")
        .long_help("Specifies a range of emails. The range follows the RFC3501 format.")
        .value_name("IDS")
        .required(true)
}

/// Represents the email ids argument parser.
pub fn parse_ids_arg<'a>(matches: &'a ArgMatches<'a>) -> &'a str {
    matches.value_of(email::args::ARG_IDS).unwrap()
}

/// Represents the email reply all argument.
pub fn reply_all_flag<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_REPLY_ALL)
        .help("Includes all recipients")
        .short("A")
        .long("all")
}

/// Represents the email reply all argument parser.
pub fn parse_reply_all_flag<'a>(matches: &'a ArgMatches<'a>) -> bool {
    matches.is_present(ARG_REPLY_ALL)
}

/// Represents the page size argument.
fn page_size_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_PAGE_SIZE)
        .help("Page size")
        .short("s")
        .long("size")
        .value_name("INT")
}

/// Represents the page size argument parser.
fn parse_page_size_arg<'a>(matches: &'a ArgMatches<'a>) -> Option<usize> {
    matches.value_of(ARG_PAGE_SIZE).and_then(|s| s.parse().ok())
}

/// Represents the page argument.
fn page_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_PAGE)
        .help("Page number")
        .short("p")
        .long("page")
        .value_name("INT")
        .default_value("1")
}

/// Represents the page argument parser.
fn parse_page_arg<'a>(matches: &'a ArgMatches<'a>) -> usize {
    matches
        .value_of(ARG_PAGE)
        .unwrap()
        .parse()
        .ok()
        .map(|page| 1.max(page) - 1)
        .unwrap_or_default()
}

/// Represents the email attachments argument.
pub fn attachments_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_ATTACHMENTS)
        .help("Adds attachment to the email")
        .short("a")
        .long("attachment")
        .value_name("PATH")
        .multiple(true)
}

/// Represents the email attachments argument parser.
pub fn parse_attachments_arg<'a>(matches: &'a ArgMatches<'a>) -> Vec<&'a str> {
    matches
        .values_of(ARG_ATTACHMENTS)
        .unwrap_or_default()
        .collect()
}

/// Represents the email headers argument.
pub fn headers_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_HEADERS)
        .help("Shows additional headers with the email")
        .short("h")
        .long("header")
        .value_name("STRING")
        .multiple(true)
}

/// Represents the email headers argument parser.
pub fn parse_headers_arg<'a>(matches: &'a ArgMatches<'a>) -> Vec<&'a str> {
    matches.values_of(ARG_HEADERS).unwrap_or_default().collect()
}

/// Represents the sanitize flag.
pub fn sanitize_flag<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_SANITIZE)
        .help("Sanitizes text bodies")
        .long("sanitize")
        .short("s")
}

/// Represents the raw flag.
pub fn raw_flag<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_RAW)
        .help("Returns raw version of email")
        .long("raw")
        .short("r")
}

/// Represents the sanitize flag parser.
pub fn parse_sanitize_flag<'a>(matches: &'a ArgMatches<'a>) -> bool {
    matches.is_present(ARG_SANITIZE)
}

/// Represents the raw flag parser.
pub fn parse_raw_flag<'a>(matches: &'a ArgMatches<'a>) -> bool {
    matches.is_present(ARG_RAW)
}

/// Represents the email raw argument.
pub fn raw_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_RAW).raw(true)
}

/// Represents the email raw argument parser.
pub fn parse_raw_arg<'a>(matches: &'a ArgMatches<'a>) -> &'a str {
    matches.value_of(ARG_RAW).unwrap_or_default()
}

/// Represents the email encrypt flag.
pub fn encrypt_flag<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_ENCRYPT)
        .help("Encrypts the email")
        .short("e")
        .long("encrypt")
}

/// Represents the email encrypt flag parser.
pub fn parse_encrypt_flag<'a>(matches: &'a ArgMatches<'a>) -> bool {
    matches.is_present(ARG_ENCRYPT)
}

/// Represents the email MIME type argument.
pub fn mime_type_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_MIME_TYPE)
        .help("MIME type to use")
        .short("t")
        .long("mime-type")
        .value_name("MIME")
        .possible_values(&["plain", "html"])
        .default_value("plain")
}

/// Represents the email MIME type argument parser.
pub fn parse_mime_type_arg<'a>(matches: &'a ArgMatches<'a>) -> &'a str {
    matches.value_of(ARG_MIME_TYPE).unwrap()
}

/// Represents the email query argument.
pub fn query_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_QUERY)
        .long_help("The query system depends on the backend, see the wiki for more details")
        .value_name("QUERY")
        .multiple(true)
        .required(true)
}

/// Represents the email query argument parser.
pub fn parse_query_arg<'a>(matches: &'a ArgMatches<'a>) -> String {
    matches
        .values_of(ARG_QUERY)
        .unwrap_or_default()
        .fold((false, vec![]), |(escape, mut cmds), cmd| {
            match (cmd, escape) {
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
