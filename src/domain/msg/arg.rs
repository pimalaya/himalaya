//! Module related to message CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message.

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, SubCommand};
use log::debug;

use crate::domain::{mbox, msg};

type Uid<'a> = &'a str;
type PageSize = usize;
type Page = usize;
type TargetMbox<'a> = Option<&'a str>;
type Mime = String;
type Raw = bool;
type All = bool;
type RawMsg<'a> = &'a str;
type Query = String;
type AttachmentsPaths<'a> = Vec<&'a str>;

/// Message commands.
pub enum Command<'a> {
    Attachments(Uid<'a>),
    Copy(Uid<'a>, TargetMbox<'a>),
    Delete(Uid<'a>),
    Forward(Uid<'a>, AttachmentsPaths<'a>),
    List(Option<PageSize>, Page),
    Move(Uid<'a>, TargetMbox<'a>),
    Read(Uid<'a>, Mime, Raw),
    Reply(Uid<'a>, All, AttachmentsPaths<'a>),
    Save(TargetMbox<'a>, RawMsg<'a>),
    Search(Query, Option<PageSize>, Page),
    Send(RawMsg<'a>),
    Write(AttachmentsPaths<'a>),

    Flag(Option<msg::flag::arg::Command<'a>>),
    Tpl(Option<msg::tpl::arg::Command<'a>>),
}

/// Message command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Command<'a>>> {
    if let Some(m) = m.subcommand_matches("attachments") {
        debug!("attachments command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", &uid);
        return Ok(Some(Command::Attachments(uid)));
    }

    if let Some(m) = m.subcommand_matches("copy") {
        debug!("copy command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", &uid);
        let target = m.value_of("target");
        debug!("target mailbox: `{:?}`", target);
        return Ok(Some(Command::Copy(uid, target)));
    }

    if let Some(m) = m.subcommand_matches("delete") {
        debug!("copy command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", &uid);
        return Ok(Some(Command::Delete(uid)));
    }

    if let Some(m) = m.subcommand_matches("forward") {
        debug!("forward command matched");
        let uid = m.value_of("uid").unwrap();
        let paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        debug!("attachments paths: {:?}", paths);
        debug!("uid: {}", &uid);
        return Ok(Some(Command::Forward(uid, paths)));
    }

    if let Some(m) = m.subcommand_matches("list") {
        debug!("list command matched");
        let page_size = m.value_of("page-size").and_then(|s| s.parse().ok());
        debug!("page size: `{:?}`", page_size);
        let page = m
            .value_of("page")
            .unwrap_or("1")
            .parse()
            .ok()
            .map(|page| 1.max(page) - 1)
            .unwrap_or_default();
        debug!("page: `{:?}`", page);
        return Ok(Some(Command::List(page_size, page)));
    }

    if let Some(m) = m.subcommand_matches("move") {
        debug!("move command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", &uid);
        let target = m.value_of("target");
        debug!("target mailbox: `{:?}`", target);
        return Ok(Some(Command::Move(uid, target)));
    }

    if let Some(m) = m.subcommand_matches("read") {
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", uid);
        let mime = format!("text/{}", m.value_of("mime-type").unwrap());
        debug!("mime: {}", mime);
        let raw = m.is_present("raw");
        debug!("raw: {}", raw);
        return Ok(Some(Command::Read(uid, mime, raw)));
    }

    if let Some(m) = m.subcommand_matches("reply") {
        debug!("reply command matched");
        let uid = m.value_of("uid").unwrap();
        debug!("uid: {}", uid);
        let all = m.is_present("reply-all");
        debug!("reply all: {}", all);
        let paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        debug!("attachments paths: {:?}", paths);
        return Ok(Some(Command::Reply(uid, all, paths)));
    }

    if let Some(m) = m.subcommand_matches("save") {
        debug!("save command matched");
        let msg = m.value_of("message").unwrap();
        debug!("message: {}", &msg);
        let target = m.value_of("target");
        debug!("target mailbox: `{:?}`", target);
        return Ok(Some(Command::Save(target, msg)));
    }

    if let Some(m) = m.subcommand_matches("search") {
        debug!("search command matched");
        let page_size = m.value_of("page-size").and_then(|s| s.parse().ok());
        debug!("page size: `{:?}`", page_size);
        let page = m
            .value_of("page")
            .unwrap()
            .parse()
            .ok()
            .map(|page| 1.max(page) - 1)
            .unwrap_or_default();
        debug!("page: `{:?}`", page);
        let query = m
            .values_of("query")
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
            .join(" ");
        return Ok(Some(Command::Search(query, page_size, page)));
    }

    if let Some(m) = m.subcommand_matches("send") {
        debug!("send command matched");
        let msg = m.value_of("message").unwrap_or_default();
        debug!("message: {}", msg);
        return Ok(Some(Command::Send(msg)));
    }

    if let Some(m) = m.subcommand_matches("write") {
        debug!("write command matched");
        let attachment_paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        debug!("attachments paths: {:?}", attachment_paths);
        return Ok(Some(Command::Write(attachment_paths)));
    }

    if let Some(m) = m.subcommand_matches("template") {
        return Ok(Some(Command::Tpl(msg::tpl::arg::matches(&m)?)));
    }

    if let Some(m) = m.subcommand_matches("flags") {
        return Ok(Some(Command::Flag(msg::flag::arg::matches(&m)?)));
    }

    debug!("default list command matched");
    Ok(Some(Command::List(None, 0)))
}

/// Message UID argument.
pub(crate) fn uid_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("uid")
        .help("Specifies the targetted message")
        .value_name("UID")
        .required(true)
}

/// Message reply all argument.
pub(crate) fn reply_all_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("reply-all")
        .help("Includes all recipients")
        .short("A")
        .long("all")
}

/// Message page size argument.
fn page_size_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("page-size")
        .help("Page size")
        .short("s")
        .long("size")
        .value_name("INT")
}

/// Message page argument.
fn page_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("page")
        .help("Page number")
        .short("p")
        .long("page")
        .value_name("INT")
        .default_value("0")
}

/// Message subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![
        msg::flag::arg::subcmds(),
        msg::tpl::arg::subcmds(),
        msg::attachment::arg::subcmds(),
        vec![
            SubCommand::with_name("list")
                .aliases(&["lst", "l"])
                .about("Lists all messages")
                .arg(page_size_arg())
                .arg(page_arg()),
            SubCommand::with_name("search")
                .aliases(&["s", "query", "q"])
                .about("Lists messages matching the given IMAP query")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(
                    Arg::with_name("query")
                        .help("IMAP query (see https://tools.ietf.org/html/rfc3501#section-6.4.4)")
                        .value_name("QUERY")
                        .multiple(true)
                        .required(true),
                ),
            SubCommand::with_name("write")
                .about("Writes a new message")
                .arg(msg::attachment::arg::path_arg()),
            SubCommand::with_name("send")
                .about("Sends a raw message")
                .arg(Arg::with_name("message").raw(true).last(true)),
            SubCommand::with_name("save")
                .about("Saves a raw message")
                .arg(Arg::with_name("message").raw(true)),
            SubCommand::with_name("read")
                .about("Reads text bodies of a message")
                .arg(uid_arg())
                .arg(
                    Arg::with_name("mime-type")
                        .help("MIME type to use")
                        .short("t")
                        .long("mime-type")
                        .value_name("MIME")
                        .possible_values(&["plain", "html"])
                        .default_value("plain"),
                )
                .arg(
                    Arg::with_name("raw")
                        .help("Reads raw message")
                        .long("raw")
                        .short("r"),
                ),
            SubCommand::with_name("reply")
                .about("Answers to a message")
                .arg(uid_arg())
                .arg(reply_all_arg())
                .arg(msg::attachment::arg::path_arg()),
            SubCommand::with_name("forward")
                .aliases(&["fwd"])
                .about("Forwards a message")
                .arg(uid_arg())
                .arg(msg::attachment::arg::path_arg()),
            SubCommand::with_name("copy")
                .aliases(&["cp", "c"])
                .about("Copies a message to the targetted mailbox")
                .arg(uid_arg())
                .arg(mbox::arg::target_arg()),
            SubCommand::with_name("move")
                .aliases(&["mv"])
                .about("Moves a message to the targetted mailbox")
                .arg(uid_arg())
                .arg(mbox::arg::target_arg()),
            SubCommand::with_name("delete")
                .aliases(&["remove", "rm"])
                .about("Deletes a message")
                .arg(uid_arg()),
        ],
    ]
    .concat()
}
