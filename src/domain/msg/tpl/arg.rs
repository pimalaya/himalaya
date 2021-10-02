//! Module related to message template CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message template.

use anyhow::Result;
use clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};
use log::{debug, trace};

use crate::domain::msg;

type Seq<'a> = &'a str;
type All = bool;

#[derive(Debug, Default)]
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

/// Message template commands.
pub enum Command<'a> {
    New(TplOverride<'a>),
    Reply(Seq<'a>, All, TplOverride<'a>),
    Forward(Seq<'a>, TplOverride<'a>),
}

/// Message template command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Command<'a>>> {
    if let Some(m) = m.subcommand_matches("new") {
        debug!("new command matched");
        let tpl = TplOverride {
            subject: m.value_of("subject"),
            from: m.values_of("from").map(|v| v.collect()),
            to: m.values_of("to").map(|v| v.collect()),
            cc: m.values_of("cc").map(|v| v.collect()),
            bcc: m.values_of("bcc").map(|v| v.collect()),
            headers: m.values_of("headers").map(|v| v.collect()),
            body: m.value_of("body"),
            sig: m.value_of("signature"),
        };
        trace!(r#"template args: "{:?}""#, tpl);
        return Ok(Some(Command::New(tpl)));
    }

    if let Some(m) = m.subcommand_matches("reply") {
        debug!("reply command matched");
        let seq = m.value_of("seq").unwrap();
        trace!(r#"seq: "{}""#, seq);
        let all = m.is_present("reply-all");
        trace!("reply all: {}", all);
        let tpl = TplOverride {
            subject: m.value_of("subject"),
            from: m.values_of("from").map(|v| v.collect()),
            to: m.values_of("to").map(|v| v.collect()),
            cc: m.values_of("cc").map(|v| v.collect()),
            bcc: m.values_of("bcc").map(|v| v.collect()),
            headers: m.values_of("headers").map(|v| v.collect()),
            body: m.value_of("body"),
            sig: m.value_of("signature"),
        };
        trace!(r#"template args: "{:?}""#, tpl);
        return Ok(Some(Command::Reply(seq, all, tpl)));
    }

    if let Some(m) = m.subcommand_matches("forward") {
        debug!("forward command matched");
        let seq = m.value_of("seq").unwrap();
        trace!(r#"seq: "{}""#, seq);
        let tpl = TplOverride {
            subject: m.value_of("subject"),
            from: m.values_of("from").map(|v| v.collect()),
            to: m.values_of("to").map(|v| v.collect()),
            cc: m.values_of("cc").map(|v| v.collect()),
            bcc: m.values_of("bcc").map(|v| v.collect()),
            headers: m.values_of("headers").map(|v| v.collect()),
            body: m.value_of("body"),
            sig: m.value_of("signature"),
        };
        trace!(r#"template args: "{:?}""#, tpl);
        return Ok(Some(Command::Forward(seq, tpl)));
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
                .aliases(&["rep", "r"])
                .about("Generates a reply message template")
                .arg(msg::arg::seq_arg())
                .arg(msg::arg::reply_all_arg())
                .args(&tpl_args()),
        )
        .subcommand(
            SubCommand::with_name("forward")
                .aliases(&["fwd", "fw", "f"])
                .about("Generates a forward message template")
                .arg(msg::arg::seq_arg())
                .args(&tpl_args()),
        )]
}
