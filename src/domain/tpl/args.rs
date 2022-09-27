//! Module related to email template CLI.
//!
//! This module provides subcommands, arguments and a command matcher
//! related to email templating.

use anyhow::Result;
use clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};
use himalaya_lib::email::TplOverride;
use log::debug;

use crate::email;

const ARG_BCC: &str = "bcc";
const ARG_BODY: &str = "body";
const ARG_CC: &str = "cc";
const ARG_FROM: &str = "from";
const ARG_HEADERS: &str = "header";
const ARG_SIGNATURE: &str = "signature";
const ARG_SUBJECT: &str = "subject";
const ARG_TO: &str = "to";
const ARG_TPL: &str = "template";
const CMD_FORWARD: &str = "forward";
const CMD_NEW: &str = "new";
const CMD_REPLY: &str = "reply";
const CMD_SAVE: &str = "save";
const CMD_SEND: &str = "send";

pub(crate) const CMD_TPL: &str = "template";

type Tpl<'a> = &'a str;

/// Represents the template commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Forward(email::args::Id<'a>, TplOverride<'a>),
    New(TplOverride<'a>),
    Reply(email::args::Id<'a>, email::args::All, TplOverride<'a>),
    Save(email::args::Attachments<'a>, Tpl<'a>),
    Send(email::args::Attachments<'a>, Tpl<'a>),
}

/// Represents the template command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    let cmd = if let Some(m) = m.subcommand_matches(CMD_FORWARD) {
        debug!("forward subcommand matched");
        let id = email::args::parse_id_arg(m);
        let tpl = parse_override_arg(m);
        Some(Cmd::Forward(id, tpl))
    } else if let Some(m) = m.subcommand_matches(CMD_NEW) {
        debug!("new subcommand matched");
        let tpl = parse_override_arg(m);
        Some(Cmd::New(tpl))
    } else if let Some(m) = m.subcommand_matches(CMD_REPLY) {
        debug!("reply subcommand matched");
        let id = email::args::parse_id_arg(m);
        let all = email::args::parse_reply_all_flag(m);
        let tpl = parse_override_arg(m);
        Some(Cmd::Reply(id, all, tpl))
    } else if let Some(m) = m.subcommand_matches(CMD_SAVE) {
        debug!("save subcommand matched");
        let attachments = email::args::parse_attachments_arg(m);
        let tpl = parse_raw_arg(m);
        Some(Cmd::Save(attachments, tpl))
    } else if let Some(m) = m.subcommand_matches(CMD_SEND) {
        debug!("send subcommand matched");
        let attachments = email::args::parse_attachments_arg(m);
        let tpl = parse_raw_arg(m);
        Some(Cmd::Send(attachments, tpl))
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
            SubCommand::with_name(CMD_NEW)
                .aliases(&["n"])
                .about("Generates a template for a new email")
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
            SubCommand::with_name(CMD_FORWARD)
                .aliases(&["fwd", "fw", "f"])
                .about("Generates a template for forwarding an email")
                .arg(email::args::id_arg())
                .args(&args()),
        )
        .subcommand(
            SubCommand::with_name(CMD_SAVE)
                .about("Saves an email based on the given template")
                .arg(&email::args::attachments_arg())
                .arg(Arg::with_name(ARG_TPL).raw(true)),
        )
        .subcommand(
            SubCommand::with_name(CMD_SEND)
                .about("Sends an email based on the given template")
                .arg(&email::args::attachments_arg())
                .arg(Arg::with_name(ARG_TPL).raw(true)),
        )]
}

/// Represents the template arguments.
pub fn args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name(ARG_SUBJECT)
            .help("Overrides the Subject header")
            .short("s")
            .long("subject")
            .value_name("STRING"),
        Arg::with_name(ARG_FROM)
            .help("Overrides the From header")
            .short("f")
            .long("from")
            .value_name("ADDR")
            .multiple(true),
        Arg::with_name(ARG_TO)
            .help("Overrides the To header")
            .short("t")
            .long("to")
            .value_name("ADDR")
            .multiple(true),
        Arg::with_name(ARG_CC)
            .help("Overrides the Cc header")
            .short("c")
            .long("cc")
            .value_name("ADDR")
            .multiple(true),
        Arg::with_name(ARG_BCC)
            .help("Overrides the Bcc header")
            .short("b")
            .long("bcc")
            .value_name("ADDR")
            .multiple(true),
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
        Arg::with_name(ARG_SIGNATURE)
            .help("Overrides the signature")
            .short("S")
            .long("signature")
            .value_name("STRING"),
    ]
}

/// Represents the template override argument parser.
pub fn parse_override_arg<'a>(matches: &'a ArgMatches<'a>) -> TplOverride {
    TplOverride {
        subject: matches.value_of(ARG_SUBJECT),
        from: matches.values_of(ARG_FROM).map(Iterator::collect),
        to: matches.values_of(ARG_TO).map(Iterator::collect),
        cc: matches.values_of(ARG_CC).map(Iterator::collect),
        bcc: matches.values_of(ARG_BCC).map(Iterator::collect),
        headers: matches.values_of(ARG_HEADERS).map(Iterator::collect),
        body: matches.value_of(ARG_BODY),
        signature: matches.value_of(ARG_SIGNATURE),
    }
}

/// Represents the raw template argument parser.
pub fn parse_raw_arg<'a>(matches: &'a ArgMatches<'a>) -> &'a str {
    matches.value_of(ARG_TPL).unwrap_or_default()
}
