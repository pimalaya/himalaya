//! Email CLI module.
//!
//! This module contains the command matcher, the subcommands and the
//! arguments related to the email domain.

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::template;

const ARG_CRITERIA: &str = "criterion";
const ARG_HEADERS: &str = "headers";
const ARG_ID: &str = "id";
const ARG_IDS: &str = "ids";
const ARG_MIME_TYPE: &str = "mime-type";
const ARG_QUERY: &str = "query";
const ARG_RAW: &str = "raw";
const ARG_REPLY_ALL: &str = "reply-all";
const CMD_ATTACHMENTS: &str = "attachments";
const CMD_COPY: &str = "copy";
const CMD_DELETE: &str = "delete";
const CMD_FORWARD: &str = "forward";
const CMD_MESSAGE: &str = "message";
const CMD_MOVE: &str = "move";
const CMD_READ: &str = "read";
const CMD_REPLY: &str = "reply";
const CMD_SAVE: &str = "save";
const CMD_SEND: &str = "send";
const CMD_WRITE: &str = "write";

pub type All = bool;
pub type Criteria = String;
pub type Folder<'a> = &'a str;
pub type Headers<'a> = Vec<&'a str>;
pub type Id<'a> = &'a str;
pub type Ids<'a> = Vec<&'a str>;
pub type Query = String;
pub type Raw = bool;
pub type RawEmail = String;
pub type TextMime<'a> = &'a str;

/// Represents the email commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Attachments(Ids<'a>),
    Copy(Ids<'a>, Folder<'a>),
    Delete(Ids<'a>),
    Forward(
        Id<'a>,
        template::args::Headers<'a>,
        template::args::Body<'a>,
    ),
    Move(Ids<'a>, Folder<'a>),
    Read(Ids<'a>, TextMime<'a>, Raw, Headers<'a>),
    Reply(
        Id<'a>,
        All,
        template::args::Headers<'a>,
        template::args::Body<'a>,
    ),
    Save(RawEmail),
    Send(RawEmail),
    Write(template::args::Headers<'a>, template::args::Body<'a>),
}

/// Email command matcher.
pub fn matches(m: &ArgMatches) -> Result<Option<Cmd>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_MESSAGE) {
        if let Some(m) = m.subcommand_matches(CMD_ATTACHMENTS) {
            let ids = parse_ids_arg(m);
            Some(Cmd::Attachments(ids))
        } else if let Some(m) = m.subcommand_matches(CMD_COPY) {
            let ids = parse_ids_arg(m);
            let folder = "INBOX";
            Some(Cmd::Copy(ids, folder))
        } else if let Some(m) = m.subcommand_matches(CMD_DELETE) {
            let ids = parse_ids_arg(m);
            Some(Cmd::Delete(ids))
        } else if let Some(m) = m.subcommand_matches(CMD_FORWARD) {
            let id = parse_id_arg(m);
            let headers = template::args::parse_headers_arg(m);
            let body = template::args::parse_body_arg(m);
            Some(Cmd::Forward(id, headers, body))
        } else if let Some(m) = m.subcommand_matches(CMD_MOVE) {
            let ids = parse_ids_arg(m);
            let folder = "INBOX";
            Some(Cmd::Move(ids, folder))
        } else if let Some(m) = m.subcommand_matches(CMD_READ) {
            let ids = parse_ids_arg(m);
            let mime = parse_mime_type_arg(m);
            let raw = parse_raw_flag(m);
            let headers = parse_headers_arg(m);
            Some(Cmd::Read(ids, mime, raw, headers))
        } else if let Some(m) = m.subcommand_matches(CMD_REPLY) {
            let id = parse_id_arg(m);
            let all = parse_reply_all_flag(m);
            let headers = template::args::parse_headers_arg(m);
            let body = template::args::parse_body_arg(m);
            Some(Cmd::Reply(id, all, headers, body))
        } else if let Some(m) = m.subcommand_matches(CMD_SAVE) {
            let email = parse_raw_arg(m);
            Some(Cmd::Save(email))
        } else if let Some(m) = m.subcommand_matches(CMD_SEND) {
            let email = parse_raw_arg(m);
            Some(Cmd::Send(email))
        } else if let Some(m) = m.subcommand_matches(CMD_WRITE) {
            let headers = template::args::parse_headers_arg(m);
            let body = template::args::parse_body_arg(m);
            Some(Cmd::Write(headers, body))
        } else {
            None
        }
    } else {
        None
    };

    Ok(cmd)
}

/// Represents the email subcommands.
pub fn subcmd() -> Command {
    Command::new(CMD_MESSAGE)
        .about("Subcommand to manage messages")
        .long_about("Subcommand to manage messages like read, write, reply or send")
        .aliases(["msg"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands([
            Command::new(CMD_ATTACHMENTS)
                .about("Downloads all emails attachments")
                .arg(ids_arg()),
            Command::new(CMD_WRITE)
                .about("Write a new email")
                .aliases(["new", "n"])
                .args(template::args::args()),
            Command::new(CMD_SEND)
                .about("Send a raw email")
                .arg(raw_arg()),
            Command::new(CMD_SAVE)
                .about("Save a raw email")
                .arg(raw_arg()),
            Command::new(CMD_READ)
                .about("Read text bodies of emails")
                .arg(mime_type_arg())
                .arg(raw_flag())
                .arg(headers_arg())
                .arg(ids_arg()),
            Command::new(CMD_REPLY)
                .about("Answer to an email")
                .arg(reply_all_flag())
                .args(template::args::args())
                .arg(id_arg()),
            Command::new(CMD_FORWARD)
                .aliases(["fwd", "f"])
                .about("Forward an email")
                .args(template::args::args())
                .arg(id_arg()),
            Command::new(CMD_COPY)
                .alias("cp")
                .about("Copy emails to the given folder")
                // .arg(folder::args::target_arg())
                .arg(ids_arg()),
            Command::new(CMD_MOVE)
                .alias("mv")
                .about("Move emails to the given folder")
                // .arg(folder::args::target_arg())
                .arg(ids_arg()),
            Command::new(CMD_DELETE)
                .aliases(["remove", "rm"])
                .about("Delete emails")
                .arg(ids_arg()),
        ])
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

/// Represents the email ids argument.
pub fn ids_arg() -> Arg {
    Arg::new(ARG_IDS)
        .help("Email ids")
        .value_name("IDS")
        .num_args(1..)
        .required(true)
}

/// Represents the email ids argument parser.
pub fn parse_ids_arg(matches: &ArgMatches) -> Vec<&str> {
    matches
        .get_many::<String>(ARG_IDS)
        .unwrap()
        .map(String::as_str)
        .collect()
}

/// Represents the email sort criteria argument.
pub fn criteria_arg<'a>() -> Arg {
    Arg::new(ARG_CRITERIA)
        .help("Email sorting preferences")
        .long("criterion")
        .short('c')
        .value_name("CRITERION:ORDER")
        .action(ArgAction::Append)
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
        .help("Include all recipients")
        .long("all")
        .short('A')
        .action(ArgAction::SetTrue)
}

/// Represents the email reply all argument parser.
pub fn parse_reply_all_flag(matches: &ArgMatches) -> bool {
    matches.get_flag(ARG_REPLY_ALL)
}

/// Represents the email headers argument.
pub fn headers_arg() -> Arg {
    Arg::new(ARG_HEADERS)
        .help("Shows additional headers with the email")
        .long("header")
        .short('H')
        .value_name("STRING")
        .action(ArgAction::Append)
}

/// Represents the email headers argument parser.
pub fn parse_headers_arg(m: &ArgMatches) -> Vec<&str> {
    m.get_many::<String>(ARG_HEADERS)
        .unwrap_or_default()
        .map(String::as_str)
        .collect::<Vec<_>>()
}

/// Represents the raw flag.
pub fn raw_flag() -> Arg {
    Arg::new(ARG_RAW)
        .help("Returns raw version of email")
        .long("raw")
        .short('r')
        .action(ArgAction::SetTrue)
}

/// Represents the raw flag parser.
pub fn parse_raw_flag(m: &ArgMatches) -> bool {
    m.get_flag(ARG_RAW)
}

/// Represents the email raw argument.
pub fn raw_arg() -> Arg {
    Arg::new(ARG_RAW).raw(true)
}

/// Represents the email raw argument parser.
pub fn parse_raw_arg(m: &ArgMatches) -> String {
    m.get_one::<String>(ARG_RAW).cloned().unwrap_or_default()
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
