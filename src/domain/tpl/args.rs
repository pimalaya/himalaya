//! Module related to email template CLI.
//!
//! This module provides subcommands, arguments and a command matcher
//! related to email templating.

use anyhow::Result;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

use crate::email;

const ARG_BODY: &str = "body";
const ARG_HEADERS: &str = "header";
const ARG_TPL: &str = "template";
const CMD_FORWARD: &str = "forward";
const CMD_REPLY: &str = "reply";
const CMD_SAVE: &str = "save";
const CMD_SEND: &str = "send";
const CMD_WRITE: &str = "write";

pub const CMD_TPL: &str = "template";

pub type RawTpl<'a> = &'a str;
pub type Headers<'a> = Option<Vec<&'a str>>;
pub type Body<'a> = Option<&'a str>;

/// Represents the template commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Forward(email::args::Id<'a>, Headers<'a>, Body<'a>),
    Write(Headers<'a>, Body<'a>),
    Reply(email::args::Id<'a>, email::args::All, Headers<'a>, Body<'a>),
    Save(RawTpl<'a>),
    Send(RawTpl<'a>),
}

/// Represents the template command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_FORWARD) {
        let id = email::args::parse_id_arg(m);
        let headers = parse_headers_arg(m);
        let body = parse_body_arg(m);
        Some(Cmd::Forward(id, headers, body))
    } else if let Some(m) = m.subcommand_matches(CMD_REPLY) {
        let id = email::args::parse_id_arg(m);
        let all = email::args::parse_reply_all_flag(m);
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
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name(CMD_TPL)
        .aliases(&["tpl"])
        .about("Handles email templates")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name(CMD_FORWARD)
                .aliases(&["fwd", "fw", "f"])
                .about("Generates a template for forwarding an email")
                .arg(email::args::id_arg())
                .args(&args()),
        )
        .subcommand(
            SubCommand::with_name(CMD_REPLY)
                .aliases(&["rep", "re", "r"])
                .about("Generates a template for replying to an email")
                .arg(email::args::id_arg())
                .arg(email::args::reply_all_flag())
                .args(&args()),
        )
        .subcommand(
            SubCommand::with_name(CMD_SAVE)
                .about("Compiles the template into a valid email then saves it")
                .arg(Arg::with_name(ARG_TPL).raw(true)),
        )
        .subcommand(
            SubCommand::with_name(CMD_SEND)
                .about("Compiles the template into a valid email then sends it")
                .arg(Arg::with_name(ARG_TPL).raw(true)),
        )
        .subcommand(
            SubCommand::with_name(CMD_WRITE)
                .aliases(&["w", "new", "n"])
                .about("Generates a template for writing a new email")
                .args(&args()),
        )]
}

/// Represents the template arguments.
pub fn args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name(ARG_HEADERS)
            .help("Overrides a specific header")
            .short("h")
            .long("header")
            .value_name("KEY:VAL")
            .multiple(true),
        Arg::with_name(ARG_BODY)
            .help("Overrides the body")
            .short("B")
            .long("body")
            .value_name("STRING"),
    ]
}

/// Represents the template headers argument parser.
pub fn parse_headers_arg<'a>(matches: &'a ArgMatches<'a>) -> Headers<'a> {
    matches.values_of(ARG_HEADERS).map(Iterator::collect)
}

/// Represents the template body argument parser.
pub fn parse_body_arg<'a>(matches: &'a ArgMatches<'a>) -> Body<'a> {
    matches.value_of(ARG_BODY)
}

/// Represents the raw template argument parser.
pub fn parse_raw_arg<'a>(matches: &'a ArgMatches<'a>) -> RawTpl<'a> {
    matches.value_of(ARG_TPL).unwrap_or_default()
}
