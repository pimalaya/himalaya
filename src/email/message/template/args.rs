//! Module related to email template CLI.
//!
//! This module provides subcommands, arguments and a command matcher
//! related to email templating.

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::warn;

use crate::message;

const ARG_BODY: &str = "body";
const ARG_HEADERS: &str = "headers";
const ARG_TPL: &str = "template";
const CMD_FORWARD: &str = "forward";
const CMD_REPLY: &str = "reply";
const CMD_SAVE: &str = "save";
const CMD_SEND: &str = "send";
const CMD_WRITE: &str = "write";

pub const CMD_TPL: &str = "template";

pub type RawTpl = String;
pub type Headers<'a> = Option<Vec<(&'a str, &'a str)>>;
pub type Body<'a> = Option<&'a str>;

/// Represents the template commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Forward(message::args::Id<'a>, Headers<'a>, Body<'a>),
    Write(Headers<'a>, Body<'a>),
    Reply(
        message::args::Id<'a>,
        message::args::All,
        Headers<'a>,
        Body<'a>,
    ),
    Save(RawTpl),
    Send(RawTpl),
}

/// Represents the template command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_FORWARD) {
        let id = message::args::parse_id_arg(m);
        let headers = parse_headers_arg(m);
        let body = parse_body_arg(m);
        Some(Cmd::Forward(id, headers, body))
    } else if let Some(m) = m.subcommand_matches(CMD_REPLY) {
        let id = message::args::parse_id_arg(m);
        let all = message::args::parse_reply_all_flag(m);
        let headers = parse_headers_arg(m);
        let body = parse_body_arg(m);
        Some(Cmd::Reply(id, all, headers, body))
    } else if let Some(m) = m.subcommand_matches(CMD_SAVE) {
        let raw_tpl = parse_raw_arg(m);
        Some(Cmd::Save(raw_tpl))
    } else if let Some(m) = m.subcommand_matches(CMD_SEND) {
        let raw_tpl = parse_raw_arg(m);
        Some(Cmd::Send(raw_tpl))
    } else if let Some(m) = m.subcommand_matches(CMD_WRITE) {
        let headers = parse_headers_arg(m);
        let body = parse_body_arg(m);
        Some(Cmd::Write(headers, body))
    } else {
        None
    };

    Ok(cmd)
}

/// Represents the template subcommands.
pub fn subcmd() -> Command {
    Command::new(CMD_TPL)
        .alias("tpl")
        .about("Subcommand to manage templates")
        .long_about("Subcommand to manage templates like write, reply, send or save")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new(CMD_FORWARD)
                .alias("fwd")
                .about("Generate a template for forwarding an email")
                .arg(message::args::id_arg())
                .args(&args()),
        )
        .subcommand(
            Command::new(CMD_REPLY)
                .about("Generate a template for replying to an email")
                .arg(message::args::id_arg())
                .arg(message::args::reply_all_flag())
                .args(&args()),
        )
        .subcommand(
            Command::new(CMD_SAVE)
                .about("Compile the template into a valid email then saves it")
                .arg(Arg::new(ARG_TPL).raw(true)),
        )
        .subcommand(
            Command::new(CMD_SEND)
                .about("Compile the template into a valid email then sends it")
                .arg(Arg::new(ARG_TPL).raw(true)),
        )
        .subcommand(
            Command::new(CMD_WRITE)
                .aliases(["new", "n"])
                .about("Generate a template for writing a new email")
                .args(&args()),
        )
}

/// Represents the template arguments.
pub fn args() -> Vec<Arg> {
    vec![
        Arg::new(ARG_HEADERS)
            .help("Override a specific header")
            .short('H')
            .long("header")
            .value_name("KEY:VAL")
            .action(ArgAction::Append),
        Arg::new(ARG_BODY)
            .help("Override the body")
            .short('B')
            .long("body")
            .value_name("STRING"),
    ]
}

/// Represents the template headers argument parser.
pub fn parse_headers_arg(m: &ArgMatches) -> Headers<'_> {
    m.get_many::<String>(ARG_HEADERS).map(|h| {
        h.filter_map(|h| match h.split_once(':') {
            Some((key, val)) => Some((key, val.trim())),
            None => {
                warn!("invalid raw header {h:?}, skipping it");
                None
            }
        })
        .collect()
    })
}

/// Represents the template body argument parser.
pub fn parse_body_arg(matches: &ArgMatches) -> Body<'_> {
    matches.get_one::<String>(ARG_BODY).map(String::as_str)
}

/// Represents the raw template argument parser.
pub fn parse_raw_arg(matches: &ArgMatches) -> RawTpl {
    matches
        .get_one::<String>(ARG_TPL)
        .cloned()
        .unwrap_or_default()
}
