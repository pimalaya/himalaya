use atty::Stream;
use clap;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::{
    fs,
    io::{self, BufRead},
    ops::Deref,
};

use crate::{
    cli,
    ctx::Ctx,
    flag::model::Flag,
    imap::model::ImapConnector,
    input,
    msg::{
        model::{Attachments, Msg, Msgs, ReadableMsg},
        tpl::{
            // cli::{tpl_matches, tpl_subcommand},
            model::Tpl,
        },
    },
    smtp,
};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
        Input(crate::input::Error, crate::input::ErrorKind);
        MsgModel(crate::msg::model::Error, crate::msg::model::ErrorKind);
        TplCli(crate::cli::tpl::Error, crate::cli::tpl::ErrorKind);
        Smtp(crate::smtp::Error, crate::smtp::ErrorKind);
    }
    foreign_links {
        Utf8(std::string::FromUtf8Error);
    }
}

// ===================
// Main Functions
// ===================
/// Provides the following subcommands:
/// - `list`
/// - `search`
/// - `write`
/// - `send`
/// - `save`
/// - `read`
/// - `attachments`
/// - `reply`
/// - `forward`
/// - `copy`
/// - `move`
/// - `delete`
pub fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![
        clap::SubCommand::with_name("list")
            .aliases(&["lst"])
            .about("Lists all messages")
            .arg(page_size_arg())
            .arg(page_arg()),
        clap::SubCommand::with_name("search")
            .aliases(&["query", "q"])
            .about("Lists messages matching the given IMAP query")
            .arg(page_size_arg())
            .arg(page_arg())
            .arg(
                clap::Arg::with_name("query")
                    .help("IMAP query (see https://tools.ietf.org/html/rfc3501#section-6.4.4)")
                    .value_name("QUERY")
                    .multiple(true)
                    .required(true),
            ),
        clap::SubCommand::with_name("write")
            .about("Writes a new message")
            .arg(attachment_arg()),
        clap::SubCommand::with_name("send")
            .about("Sends a raw message")
            .arg(clap::Arg::with_name("message").raw(true).last(true)),
        clap::SubCommand::with_name("save")
            .about("Saves a raw message")
            .arg(clap::Arg::with_name("message").raw(true)),
        clap::SubCommand::with_name("read")
            .about("Reads text bodies of a message")
            .arg(uid_arg())
            .arg(
                clap::Arg::with_name("mime-type")
                    .help("MIME type to use")
                    .short("t")
                    .long("mime-type")
                    .value_name("MIME")
                    .possible_values(&["plain", "html"])
                    .default_value("plain"),
            )
            .arg(
                clap::Arg::with_name("raw")
                    .help("Reads raw message")
                    .long("raw")
                    .short("r"),
            ),
        clap::SubCommand::with_name("attachments")
            .about("Downloads all message attachments")
            .arg(uid_arg()),
        clap::SubCommand::with_name("reply")
            .about("Answers to a message")
            .arg(uid_arg())
            .arg(reply_all_arg())
            .arg(attachment_arg()),
        clap::SubCommand::with_name("forward")
            .aliases(&["fwd"])
            .about("Forwards a message")
            .arg(uid_arg())
            .arg(attachment_arg()),
        clap::SubCommand::with_name("copy")
            .aliases(&["cp"])
            .about("Copies a message to the targetted mailbox")
            .arg(uid_arg())
            .arg(cli::mbox::target_arg()),
        clap::SubCommand::with_name("move")
            .aliases(&["mv"])
            .about("Moves a message to the targetted mailbox")
            .arg(uid_arg())
            .arg(cli::mbox::target_arg()),
        clap::SubCommand::with_name("delete")
            .aliases(&["remove", "rm"])
            .about("Deletes a message")
            .arg(uid_arg()),
        cli::tpl::subcmds(),
    ]
}

pub fn matches(ctx: &Ctx) -> Result<bool> {
    match ctx.arg_matches.subcommand() {
        ("attachments", Some(matches)) => msg_matches_attachments(ctx, matches),
        ("copy", Some(matches)) => msg_matches_copy(ctx, matches),
        ("delete", Some(matches)) => msg_matches_delete(ctx, matches),
        ("forward", Some(matches)) => msg_matches_forward(ctx, matches),
        ("move", Some(matches)) => msg_matches_move(ctx, matches),
        ("read", Some(matches)) => msg_matches_read(ctx, matches),
        ("reply", Some(matches)) => msg_matches_reply(ctx, matches),
        ("save", Some(matches)) => msg_matches_save(ctx, matches),
        ("search", Some(matches)) => msg_matches_search(ctx, matches),
        ("send", Some(matches)) => msg_matches_send(ctx, matches),
        ("write", Some(matches)) => msg_matches_write(ctx, matches),

        ("template", Some(matches)) => Ok(cli::tpl::matches(ctx, matches)?),

        ("list", opt_matches) => msg_matches_list(ctx, opt_matches),
        (_other, opt_matches) => msg_matches_list(ctx, opt_matches),
    }
}


// ==================
// Arg Functions
// ==================
/// Returns the `<UID>` argument
pub fn uid_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("uid")
        .help("Specifies the targetted message")
        .value_name("UID")
        .required(true)
}

/// Returns the `-A, --all` flag
fn reply_all_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("reply-all")
        .help("Includes all recipients")
        .short("A")
        .long("all")
}

/// Returns the `-s, --size` option
fn page_size_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("page-size")
        .help("Page size")
        .short("s")
        .long("size")
        .value_name("INT")
}

/// Returns the `-p, --page` option
fn page_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("page")
        .help("Page number")
        .short("p")
        .long("page")
        .value_name("INT")
        .default_value("0")
}

/// returns the `-a, --attachment` option
fn attachment_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("attachments")
        .help("Adds attachment to the message")
        .short("a")
        .long("attachment")
        .value_name("PATH")
        .multiple(true)
}

// ==================
// Match Actions
// ==================
fn msg_matches_list(ctx: &Ctx, opt_matches: Option<&clap::ArgMatches>) -> Result<bool> {
    debug!("list command matched");

    let page_size: usize = opt_matches
        .and_then(|matches| matches.value_of("page-size").and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| ctx.config.default_page_size(&ctx.account));
    debug!("page size: {:?}", page_size);
    let page: usize = opt_matches
        .and_then(|matches| matches.value_of("page").unwrap().parse().ok())
        .unwrap_or_default();
    debug!("page: {}", &page);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msgs = imap_conn.list_msgs(&ctx.mbox, &page_size, &page)?;
    let msgs = if let Some(ref fetches) = msgs {
        Msgs::from(fetches)
    } else {
        Msgs::new()
    };

    trace!("messages: {:?}", msgs);
    ctx.output.print(msgs);

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_search(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("search command matched");

    let page_size: usize = matches
        .value_of("page-size")
        .and_then(|s| s.parse().ok())
        .unwrap_or(ctx.config.default_page_size(&ctx.account));
    debug!("page size: {}", &page_size);
    let page: usize = matches
        .value_of("page")
        .unwrap()
        .parse()
        .unwrap_or_default();
    debug!("page: {}", &page);

    let query = matches
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
    debug!("query: {}", &page);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msgs = imap_conn.search_msgs(&ctx.mbox, &query, &page_size, &page)?;
    let msgs = if let Some(ref fetches) = msgs {
        Msgs::from(fetches)
    } else {
        Msgs::new()
    };
    trace!("messages: {:?}", msgs);
    ctx.output.print(msgs);

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_read(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("read command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);
    let mime = format!("text/{}", matches.value_of("mime-type").unwrap());
    debug!("mime: {}", mime);
    let raw = matches.is_present("raw");
    debug!("raw: {}", raw);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = imap_conn.read_msg(&ctx.mbox, &uid)?;
    if raw {
        let msg =
            String::from_utf8(msg).chain_err(|| "Could not decode raw message as utf8 string")?;
        let msg = msg.trim_end_matches("\n");
        ctx.output.print(msg);
    } else {
        let msg = ReadableMsg::from_bytes(&mime, &msg)?;
        ctx.output.print(msg);
    }

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_attachments(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("attachments command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = imap_conn.read_msg(&ctx.mbox, &uid)?;
    let attachments = Attachments::from_bytes(&msg)?;
    debug!(
        "{} attachment(s) found for message {}",
        &attachments.0.len(),
        &uid
    );
    for attachment in attachments.0.iter() {
        let filepath = ctx
            .config
            .downloads_filepath(&ctx.account, &attachment.filename);
        debug!("downloading {}…", &attachment.filename);
        fs::write(&filepath, &attachment.raw)
            .chain_err(|| format!("Could not save attachment {:?}", filepath))?;
    }

    debug!(
        "{} attachment(s) successfully downloaded",
        &attachments.0.len()
    );
    ctx.output.print(format!(
        "{} attachment(s) successfully downloaded",
        &attachments.0.len()
    ));

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_write(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("write command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let attachments = matches
        .values_of("attachments")
        .unwrap_or_default()
        .map(String::from)
        .collect::<Vec<_>>();
    let tpl = Tpl::new(&ctx);
    let content = input::open_editor_with_tpl(tpl.to_string().as_bytes())?;
    let mut msg = Msg::from(content);
    msg.attachments = attachments;

    loop {
        match input::post_edit_choice() {
            Ok(choice) => match choice {
                input::PostEditChoice::Send => {
                    debug!("sending message…");
                    let msg = msg.to_sendable_msg()?;
                    smtp::send(&ctx.account, &msg)?;
                    imap_conn.append_msg("Sent", &msg.formatted(), vec![Flag::Seen])?;
                    input::remove_draft()?;
                    ctx.output.print("Message successfully sent");
                    break;
                }
                input::PostEditChoice::Edit => {
                    let content = input::open_editor_with_draft()?;
                    msg = Msg::from(content);
                }
                input::PostEditChoice::LocalDraft => break,
                input::PostEditChoice::RemoteDraft => {
                    debug!("saving to draft…");
                    imap_conn.append_msg("Drafts", &msg.to_vec()?, vec![Flag::Seen])?;
                    input::remove_draft()?;
                    ctx.output.print("Message successfully saved to Drafts");
                    break;
                }
                input::PostEditChoice::Discard => {
                    input::remove_draft()?;
                    break;
                }
            },
            Err(err) => error!("{}", err),
        }
    }
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_reply(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("reply command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);
    let attachments = matches
        .values_of("attachments")
        .unwrap_or_default()
        .map(String::from)
        .collect::<Vec<_>>();
    debug!("found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = Msg::from(imap_conn.read_msg(&ctx.mbox, &uid)?);
    let tpl = if matches.is_present("reply-all") {
        msg.build_reply_all_tpl(&ctx.config, &ctx.account)?
    } else {
        msg.build_reply_tpl(&ctx.config, &ctx.account)?
    };

    let content = input::open_editor_with_tpl(&tpl.to_string().as_bytes())?;
    let mut msg = Msg::from(content);
    msg.attachments = attachments;

    loop {
        match input::post_edit_choice() {
            Ok(choice) => match choice {
                input::PostEditChoice::Send => {
                    debug!("sending message…");
                    let msg = msg.to_sendable_msg()?;
                    smtp::send(&ctx.account, &msg)?;
                    imap_conn.append_msg("Sent", &msg.formatted(), vec![Flag::Seen])?;
                    imap_conn.add_flags(&ctx.mbox, uid, "\\Answered")?;
                    input::remove_draft()?;
                    ctx.output.print("Message successfully sent");
                    break;
                }
                input::PostEditChoice::Edit => {
                    let content = input::open_editor_with_draft()?;
                    msg = Msg::from(content);
                }
                input::PostEditChoice::LocalDraft => break,
                input::PostEditChoice::RemoteDraft => {
                    debug!("saving to draft…");
                    imap_conn.append_msg("Drafts", &msg.to_vec()?, vec![Flag::Seen])?;
                    input::remove_draft()?;
                    ctx.output.print("Message successfully saved to Drafts");
                    break;
                }
                input::PostEditChoice::Discard => {
                    input::remove_draft()?;
                    break;
                }
            },
            Err(err) => error!("{}", err),
        }
    }

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_forward(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("forward command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);
    let attachments = matches
        .values_of("attachments")
        .unwrap_or_default()
        .map(String::from)
        .collect::<Vec<_>>();
    debug!("found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = Msg::from(imap_conn.read_msg(&ctx.mbox, &uid)?);
    let tpl = msg.build_forward_tpl(&ctx.config, &ctx.account)?;
    let content = input::open_editor_with_tpl(&tpl.to_string().as_bytes())?;
    let mut msg = Msg::from(content);
    msg.attachments = attachments;

    loop {
        match input::post_edit_choice() {
            Ok(choice) => match choice {
                input::PostEditChoice::Send => {
                    debug!("sending message…");
                    let msg = msg.to_sendable_msg()?;
                    smtp::send(&ctx.account, &msg)?;
                    imap_conn.append_msg("Sent", &msg.formatted(), vec![Flag::Seen])?;
                    input::remove_draft()?;
                    ctx.output.print("Message successfully sent");
                    break;
                }
                input::PostEditChoice::Edit => {
                    let content = input::open_editor_with_draft()?;
                    msg = Msg::from(content);
                }
                input::PostEditChoice::LocalDraft => break,
                input::PostEditChoice::RemoteDraft => {
                    debug!("saving to draft…");
                    imap_conn.append_msg("Drafts", &msg.to_vec()?, vec![Flag::Seen])?;
                    input::remove_draft()?;
                    ctx.output.print("Message successfully saved to Drafts");
                    break;
                }
                input::PostEditChoice::Discard => {
                    input::remove_draft()?;
                    break;
                }
            },
            Err(err) => error!("{}", err),
        }
    }

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_copy(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("copy command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);
    let target = matches.value_of("target").unwrap();
    debug!("target: {}", &target);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = Msg::from(imap_conn.read_msg(&ctx.mbox, &uid)?);
    let mut flags = msg.flags.deref().to_vec();
    flags.push(Flag::Seen);
    imap_conn.append_msg(target, &msg.raw, flags)?;
    debug!("message {} successfully copied to folder `{}`", uid, target);
    ctx.output.print(format!(
        "Message {} successfully copied to folder `{}`",
        uid, target
    ));

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_move(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("move command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);
    let target = matches.value_of("target").unwrap();
    debug!("target: {}", &target);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = Msg::from(imap_conn.read_msg(&ctx.mbox, &uid)?);
    let mut flags = msg.flags.to_vec();
    flags.push(Flag::Seen);
    imap_conn.append_msg(target, &msg.raw, flags)?;
    imap_conn.add_flags(&ctx.mbox, uid, "\\Seen \\Deleted")?;
    debug!("message {} successfully moved to folder `{}`", uid, target);
    ctx.output.print(format!(
        "Message {} successfully moved to folder `{}`",
        uid, target
    ));

    imap_conn.expunge(&ctx.mbox)?;
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_delete(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("delete command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    imap_conn.add_flags(&ctx.mbox, uid, "\\Seen \\Deleted")?;
    debug!("message {} successfully deleted", uid);
    ctx.output
        .print(format!("Message {} successfully deleted", uid));

    imap_conn.expunge(&ctx.mbox)?;
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_send(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("send command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;

    let msg = if atty::is(Stream::Stdin) {
        matches
            .value_of("message")
            .unwrap_or_default()
            .replace("\r", "")
            .replace("\n", "\r\n")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(|ln| ln.ok())
            .map(|ln| ln.to_string())
            .collect::<Vec<_>>()
            .join("\r\n")
    };
    let msg = Msg::from(msg.to_string());
    let msg = msg.to_sendable_msg()?;
    smtp::send(&ctx.account, &msg)?;
    imap_conn.append_msg("Sent", &msg.formatted(), vec![Flag::Seen])?;

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_save(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("save command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = matches.value_of("message").unwrap();
    let msg = Msg::from(msg.to_string());
    imap_conn.append_msg(&ctx.mbox, &msg.to_vec()?, vec![Flag::Seen])?;

    imap_conn.logout();
    Ok(true)
}
