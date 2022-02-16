//! Module related to message CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message.

use std::convert::TryInto;

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, SubCommand};
use log::{debug, info, trace};

use crate::{
    domain::{
        mbox::mbox_arg,
        msg::{flag_arg, msg_arg, tpl_arg},
        SortCriterion,
    },
    ui::table_arg,
};

type Seq<'a> = &'a str;
type PageSize = usize;
type Page = usize;
type Mbox<'a> = &'a str;
type TextMime<'a> = &'a str;
type Raw = bool;
type All = bool;
type RawMsg<'a> = &'a str;
type Query = String;
type AttachmentPaths<'a> = Vec<&'a str>;
type MaxTableWidth = Option<usize>;
type Encrypt = bool;
type Criteria = String;

/// Message commands.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd<'a> {
    Attachments(Seq<'a>),
    Copy(Seq<'a>, Mbox<'a>),
    Delete(Seq<'a>),
    Forward(Seq<'a>, AttachmentPaths<'a>, Encrypt),
    List(MaxTableWidth, Option<PageSize>, Page),
    Move(Seq<'a>, Mbox<'a>),
    Read(Seq<'a>, TextMime<'a>, Raw),
    Reply(Seq<'a>, All, AttachmentPaths<'a>, Encrypt),
    Save(RawMsg<'a>),
    Search(Query, MaxTableWidth, Option<PageSize>, Page),
    Sort(Criteria, Query, MaxTableWidth, Option<PageSize>, Page),
    Send(RawMsg<'a>),
    Write(AttachmentPaths<'a>, Encrypt),

    Flag(Option<flag_arg::Cmd<'a>>),
    Tpl(Option<tpl_arg::Cmd<'a>>),
}

/// Message command matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Cmd<'a>>> {
    info!("entering message command matcher");

    if let Some(m) = m.subcommand_matches("attachments") {
        info!("attachments command matched");
        let seq = m.value_of("seq").unwrap();
        debug!("seq: {}", seq);
        return Ok(Some(Cmd::Attachments(seq)));
    }

    if let Some(m) = m.subcommand_matches("copy") {
        info!("copy command matched");
        let seq = m.value_of("seq").unwrap();
        debug!("seq: {}", seq);
        let mbox = m.value_of("mbox-target").unwrap();
        debug!(r#"target mailbox: "{:?}""#, mbox);
        return Ok(Some(Cmd::Copy(seq, mbox)));
    }

    if let Some(m) = m.subcommand_matches("delete") {
        info!("copy command matched");
        let seq = m.value_of("seq").unwrap();
        debug!("seq: {}", seq);
        return Ok(Some(Cmd::Delete(seq)));
    }

    if let Some(m) = m.subcommand_matches("forward") {
        info!("forward command matched");
        let seq = m.value_of("seq").unwrap();
        debug!("seq: {}", seq);
        let paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        debug!("attachments paths: {:?}", paths);
        let encrypt = m.is_present("encrypt");
        debug!("encrypt: {}", encrypt);
        return Ok(Some(Cmd::Forward(seq, paths, encrypt)));
    }

    if let Some(m) = m.subcommand_matches("list") {
        info!("list command matched");
        let max_table_width = m
            .value_of("max-table-width")
            .and_then(|width| width.parse::<usize>().ok());
        debug!("max table width: {:?}", max_table_width);
        let page_size = m.value_of("page-size").and_then(|s| s.parse().ok());
        debug!("page size: {:?}", page_size);
        let page = m
            .value_of("page")
            .unwrap_or("1")
            .parse()
            .ok()
            .map(|page| 1.max(page) - 1)
            .unwrap_or_default();
        debug!("page: {}", page);
        return Ok(Some(Cmd::List(max_table_width, page_size, page)));
    }

    if let Some(m) = m.subcommand_matches("move") {
        info!("move command matched");
        let seq = m.value_of("seq").unwrap();
        debug!("seq: {}", seq);
        let mbox = m.value_of("mbox-target").unwrap();
        debug!("target mailbox: {:?}", mbox);
        return Ok(Some(Cmd::Move(seq, mbox)));
    }

    if let Some(m) = m.subcommand_matches("read") {
        info!("read command matched");
        let seq = m.value_of("seq").unwrap();
        debug!("seq: {}", seq);
        let mime = m.value_of("mime-type").unwrap();
        debug!("text mime: {}", mime);
        let raw = m.is_present("raw");
        debug!("raw: {}", raw);
        return Ok(Some(Cmd::Read(seq, mime, raw)));
    }

    if let Some(m) = m.subcommand_matches("reply") {
        info!("reply command matched");
        let seq = m.value_of("seq").unwrap();
        debug!("seq: {}", seq);
        let all = m.is_present("reply-all");
        debug!("reply all: {}", all);
        let paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        debug!("attachments paths: {:?}", paths);
        let encrypt = m.is_present("encrypt");
        debug!("encrypt: {}", encrypt);

        return Ok(Some(Cmd::Reply(seq, all, paths, encrypt)));
    }

    if let Some(m) = m.subcommand_matches("save") {
        info!("save command matched");
        let msg = m.value_of("message").unwrap_or_default();
        trace!("message: {}", msg);
        return Ok(Some(Cmd::Save(msg)));
    }

    if let Some(m) = m.subcommand_matches("search") {
        info!("search command matched");
        let max_table_width = m
            .value_of("max-table-width")
            .and_then(|width| width.parse::<usize>().ok());
        debug!("max table width: {:?}", max_table_width);
        let page_size = m.value_of("page-size").and_then(|s| s.parse().ok());
        debug!("page size: {:?}", page_size);
        let page = m
            .value_of("page")
            .unwrap()
            .parse()
            .ok()
            .map(|page| 1.max(page) - 1)
            .unwrap_or_default();
        debug!("page: {}", page);
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
        debug!("query: {}", query);
        return Ok(Some(Cmd::Search(query, max_table_width, page_size, page)));
    }

    if let Some(m) = m.subcommand_matches("sort") {
        info!("sort command matched");
        let max_table_width = m
            .value_of("max-table-width")
            .and_then(|width| width.parse::<usize>().ok());
        debug!("max table width: {:?}", max_table_width);
        let page_size = m.value_of("page-size").and_then(|s| s.parse().ok());
        debug!("page size: {:?}", page_size);
        let page = m
            .value_of("page")
            .unwrap()
            .parse()
            .ok()
            .map(|page| 1.max(page) - 1)
            .unwrap_or_default();
        debug!("page: {:?}", page);
        let criteria = m
            .values_of("criteria")
            .unwrap_or_default()
            .collect::<Vec<_>>()
            .join(" ");
        debug!("criteria: {:?}", criteria);
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
        debug!("query: {:?}", query);
        return Ok(Some(Cmd::Sort(
            criteria,
            query,
            max_table_width,
            page_size,
            page,
        )));
    }

    if let Some(m) = m.subcommand_matches("send") {
        info!("send command matched");
        let msg = m.value_of("message").unwrap_or_default();
        trace!("message: {}", msg);
        return Ok(Some(Cmd::Send(msg)));
    }

    if let Some(m) = m.subcommand_matches("write") {
        info!("write command matched");
        let attachment_paths: Vec<&str> = m.values_of("attachments").unwrap_or_default().collect();
        debug!("attachments paths: {:?}", attachment_paths);
        let encrypt = m.is_present("encrypt");
        debug!("encrypt: {}", encrypt);
        return Ok(Some(Cmd::Write(attachment_paths, encrypt)));
    }

    if let Some(m) = m.subcommand_matches("template") {
        return Ok(Some(Cmd::Tpl(tpl_arg::matches(m)?)));
    }

    if let Some(m) = m.subcommand_matches("flag") {
        return Ok(Some(Cmd::Flag(flag_arg::matches(m)?)));
    }

    info!("default list command matched");
    Ok(Some(Cmd::List(None, None, 0)))
}

/// Message sequence number argument.
pub fn seq_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("seq")
        .help("Specifies the targetted message")
        .value_name("SEQ")
        .required(true)
}

/// Message sequence range argument.
pub fn seq_range_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("seq-range")
        .help("Specifies targetted message(s)")
        .long_help("Specifies a range of targetted messages. The range follows the [RFC3501](https://datatracker.ietf.org/doc/html/rfc3501#section-9) format: `1:5` matches messages with sequence number between 1 and 5, `1,5` matches messages with sequence number 1 or 5, * matches all messages.")
        .value_name("SEQ")
        .required(true)
}

/// Message reply all argument.
pub fn reply_all_arg<'a>() -> Arg<'a, 'a> {
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

/// Message attachment argument.
pub fn attachment_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("attachments")
        .help("Adds attachment to the message")
        .short("a")
        .long("attachment")
        .value_name("PATH")
        .multiple(true)
}

/// Message encrypt argument.
pub fn encrypt_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("encrypt")
        .help("Encrypts the message")
        .short("e")
        .long("encrypt")
}

/// Message subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![
        flag_arg::subcmds(),
        tpl_arg::subcmds(),
        vec![
            SubCommand::with_name("attachments")
                .aliases(&["attachment", "att", "a"])
                .about("Downloads all message attachments")
                .arg(msg_arg::seq_arg()),
            SubCommand::with_name("list")
                .aliases(&["lst", "l"])
                .about("Lists all messages")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table_arg::max_width()),
            SubCommand::with_name("search")
                .aliases(&["s", "query", "q"])
                .about("Lists messages matching the given IMAP query")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table_arg::max_width())
                .arg(
                    Arg::with_name("query")
                        .help("IMAP query")
                        .long_help("The IMAP query format follows the [RFC3501](https://tools.ietf.org/html/rfc3501#section-6.4.4). The query is case-insensitive.")
                        .value_name("QUERY")
                        .multiple(true)
                        .required(true),
                ),
            SubCommand::with_name("sort")
                .about("Sorts messages by the given criteria and matching the given IMAP query")
                .arg(page_size_arg())
                .arg(page_arg())
                .arg(table_arg::max_width())
		.arg(
		    Arg::with_name("criteria")
			.long("criteria")
			.short("c")
			.help("Defines the message sorting preferences")
			.value_name("CRITERION:ORDER")
			.takes_value(true)
			.multiple(true)
			.required(true)
			.possible_values(&[
			    "arrival", "arrival:asc", "arrival:desc",
			    "cc", "cc:asc", "cc:desc",
			    "date", "date:asc", "date:desc",
			    "from", "from:asc", "from:desc",
			    "size", "size:asc", "size:desc",
			    "subject", "subject:asc", "subject:desc",
			    "to", "to:asc", "to:desc",
			]),
		)
                .arg(
                    Arg::with_name("query")
                        .help("IMAP query")
                        .long_help("The IMAP query format follows the [RFC3501](https://tools.ietf.org/html/rfc3501#section-6.4.4). The query is case-insensitive.")
                        .value_name("QUERY")
			.default_value("ALL")
                        .raw(true),
                ),
            SubCommand::with_name("write")
                .about("Writes a new message")
                .arg(attachment_arg())
                .arg(encrypt_arg()),
            SubCommand::with_name("send")
                .about("Sends a raw message")
                .arg(Arg::with_name("message").raw(true)),
            SubCommand::with_name("save")
                .about("Saves a raw message")
                .arg(Arg::with_name("message").raw(true)),
            SubCommand::with_name("read")
                .about("Reads text bodies of a message")
                .arg(seq_arg())
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
                .aliases(&["rep", "r"])
                .about("Answers to a message")
                .arg(seq_arg())
                .arg(reply_all_arg())
                .arg(attachment_arg())
		.arg(encrypt_arg()),
            SubCommand::with_name("forward")
                .aliases(&["fwd", "f"])
                .about("Forwards a message")
                .arg(seq_arg())
                .arg(attachment_arg())
		.arg(encrypt_arg()),
            SubCommand::with_name("copy")
                .aliases(&["cp", "c"])
                .about("Copies a message to the targetted mailbox")
                .arg(seq_arg())
                .arg(mbox_arg::target_arg()),
            SubCommand::with_name("move")
                .aliases(&["mv"])
                .about("Moves a message to the targetted mailbox")
                .arg(seq_arg())
                .arg(mbox_arg::target_arg()),
            SubCommand::with_name("delete")
                .aliases(&["del", "d", "remove", "rm"])
                .about("Deletes a message")
                .arg(seq_arg()),
        ],
    ]
    .concat()
}
