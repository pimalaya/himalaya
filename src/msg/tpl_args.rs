//! Module related to message template CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message template.

use anyhow::Result;
use clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};
use log::{debug, info, trace};

use crate::msg::msg_args;

type Seq<'a> = &'a str;
type ReplyAll = bool;
type AttachmentPaths<'a> = Vec<&'a str>;
type Tpl<'a> = &'a str;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TplOverride<'a> {
    pub subject: Option<&'a str>,
    pub from: Option<Vec<&'a str>>,
    pub to: Option<Vec<&'a str>>,
    pub cc: Option<Vec<&'a str>>,
    pub bcc: Option<Vec<&'a str>>,
    pub headers: Option<Vec<&'a str>>,
    pub body: Option<&'a str>,
    pub sig: Option<&'a str>,
}

impl<'a> From<&'a ArgMatches<'a>> for TplOverride<'a> {
    fn from(matches: &'a ArgMatches<'a>) -> Self {
        Self {
            subject: matches.value_of("subject"),
            from: matches.values_of("from").map(|v| v.collect()),
            to: matches.values_of("to").map(|v| v.collect()),
            cc: matches.values_of("cc").map(|v| v.collect()),
            bcc: matches.values_of("bcc").map(|v| v.collect()),
            headers: matches.values_of("headers").map(|v| v.collect()),
            body: matches.value_of("body"),
            sig: matches.value_of("signature"),
        }
    }
}

/// Message template commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    New(TplOverride<'a>),
    Reply(Seq<'a>, ReplyAll, TplOverride<'a>),
    Forward(Seq<'a>, TplOverride<'a>),
    Save(AttachmentPaths<'a>, Tpl<'a>),
    Send(AttachmentPaths<'a>, Tpl<'a>),
}

/// Message template command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    info!("entering message template command matcher");

    if let Some(m) = m.subcommand_matches("new") {
        info!("new subcommand matched");
        let tpl = TplOverride::from(m);
        trace!("template override: {:?}", tpl);
        return Ok(Some(Cmd::New(tpl)));
    }

    if let Some(m) = m.subcommand_matches("reply") {
        info!("reply subcommand matched");
        let seq = m.value_of("seq").unwrap();
        debug!("sequence: {}", seq);
        let all = m.is_present("reply-all");
        debug!("reply all: {}", all);
        let tpl = TplOverride::from(m);
        trace!("template override: {:?}", tpl);
        return Ok(Some(Cmd::Reply(seq, all, tpl)));
    }

    if let Some(m) = m.subcommand_matches("forward") {
        info!("forward subcommand matched");
        let seq = m.value_of("seq").unwrap();
        debug!("sequence: {}", seq);
        let tpl = TplOverride::from(m);
        trace!("template args: {:?}", tpl);
        return Ok(Some(Cmd::Forward(seq, tpl)));
    }

    if let Some(m) = m.subcommand_matches("save") {
        info!("save subcommand matched");
        let attachment_paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        trace!("attachments paths: {:?}", attachment_paths);
        let tpl = m.value_of("template").unwrap_or_default();
        trace!("template: {}", tpl);
        return Ok(Some(Cmd::Save(attachment_paths, tpl)));
    }

    if let Some(m) = m.subcommand_matches("send") {
        info!("send subcommand matched");
        let attachment_paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        trace!("attachments paths: {:?}", attachment_paths);
        let tpl = m.value_of("template").unwrap_or_default();
        trace!("template: {}", tpl);
        return Ok(Some(Cmd::Send(attachment_paths, tpl)));
    }

    Ok(None)
}

/// Message template args.
pub fn tpl_args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name("subject")
            .help("Overrides the Subject header")
            .short("s")
            .long("subject")
            .value_name("STRING"),
        Arg::with_name("from")
            .help("Overrides the From header")
            .short("f")
            .long("from")
            .value_name("ADDR")
            .multiple(true),
        Arg::with_name("to")
            .help("Overrides the To header")
            .short("t")
            .long("to")
            .value_name("ADDR")
            .multiple(true),
        Arg::with_name("cc")
            .help("Overrides the Cc header")
            .short("c")
            .long("cc")
            .value_name("ADDR")
            .multiple(true),
        Arg::with_name("bcc")
            .help("Overrides the Bcc header")
            .short("b")
            .long("bcc")
            .value_name("ADDR")
            .multiple(true),
        Arg::with_name("header")
            .help("Overrides a specific header")
            .short("h")
            .long("header")
            .value_name("KEY: VAL")
            .multiple(true),
        Arg::with_name("body")
            .help("Overrides the body")
            .short("B")
            .long("body")
            .value_name("STRING"),
        Arg::with_name("signature")
            .help("Overrides the signature")
            .short("S")
            .long("signature")
            .value_name("STRING"),
    ]
}

/// Message template subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("template")
        .aliases(&["tpl"])
        .about("Generates a message template")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("new")
                .aliases(&["n"])
                .about("Generates a new message template")
                .args(&tpl_args()),
        )
        .subcommand(
            SubCommand::with_name("reply")
                .aliases(&["rep", "re", "r"])
                .about("Generates a reply message template")
                .arg(msg_args::seq_arg())
                .arg(msg_args::reply_all_arg())
                .args(&tpl_args()),
        )
        .subcommand(
            SubCommand::with_name("forward")
                .aliases(&["fwd", "fw", "f"])
                .about("Generates a forward message template")
                .arg(msg_args::seq_arg())
                .args(&tpl_args()),
        )
        .subcommand(
            SubCommand::with_name("save")
                .about("Saves a message based on the given template")
                .arg(&msg_args::attachment_arg())
                .arg(Arg::with_name("template").raw(true)),
        )
        .subcommand(
            SubCommand::with_name("send")
                .about("Sends a message based on the given template")
                .arg(&msg_args::attachment_arg())
                .arg(Arg::with_name("template").raw(true)),
        )]
}
