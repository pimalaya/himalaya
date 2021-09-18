//! Module related to message template CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message template.

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, SubCommand};
use log::debug;

use crate::domain::msg::{self, arg::uid_arg};

type Uid<'a> = &'a str;
type All = bool;

/// Message template commands.
pub enum Command<'a> {
    New,
    Reply(Uid<'a>, All),
    Forward(Uid<'a>),
}

/// Message template command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Command<'a>>> {
    if let Some(_) = m.subcommand_matches("new") {
        debug!("new command matched");
        return Ok(Some(Command::New));
    }

    if let Some(m) = m.subcommand_matches("reply") {
        debug!("reply command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", uid);
        let all = m.is_present("reply-all");
        debug!("reply all: {}", all);
        return Ok(Some(Command::Reply(uid, all)));
    }

    if let Some(m) = m.subcommand_matches("forward") {
        debug!("forward command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", uid);
        return Ok(Some(Command::Forward(uid)));
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
                .arg(uid_arg())
                .arg(msg::arg::reply_all_arg())
                .args(&tpl_args()),
        )
        .subcommand(
            SubCommand::with_name("forward")
                .aliases(&["fwd", "fw", "f"])
                .about("Generates a forward message template")
                .arg(uid_arg())
                .args(&tpl_args()),
        )]
}
