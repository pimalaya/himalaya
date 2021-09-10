use super::body::Body;
use super::headers::Headers;
use super::model::{Msg, MsgSerialized, Msgs};
use url::Url;

use atty::Stream;
use clap;
use error_chain::error_chain;
use lettre::message::header::ContentTransferEncoding;
use log::{debug, error, trace};

use std::{
    borrow::Cow,
    collections::HashMap,
    convert::TryFrom,
    fs,
    io::{self, BufRead},
};

use imap::types::Flag;

use crate::{
    ctx::Ctx, flag::model::Flags, imap::model::ImapConnector, input, mbox::cli::mbox_target_arg,
    smtp,
};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
        Input(crate::input::Error, crate::input::ErrorKind);
        MsgModel(super::model::Error, super::model::ErrorKind);
        Smtp(crate::smtp::Error, crate::smtp::ErrorKind);
    }
    foreign_links {
        Utf8(std::string::FromUtf8Error);
    }
}

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
            .arg(mbox_target_arg()),
        clap::SubCommand::with_name("move")
            .aliases(&["mv"])
            .about("Moves a message to the targetted mailbox")
            .arg(uid_arg())
            .arg(mbox_target_arg()),
        clap::SubCommand::with_name("delete")
            .aliases(&["remove", "rm"])
            .about("Deletes a message")
            .arg(uid_arg()),
        clap::SubCommand::with_name("template")
            .aliases(&["tpl"])
            .about("Generates a message template")
            .subcommand(
                clap::SubCommand::with_name("new")
                    .aliases(&["n"])
                    .about("Generates a new message template")
                    .args(&tpl_args()),
            )
            .subcommand(
                clap::SubCommand::with_name("reply")
                    .aliases(&["rep", "r"])
                    .about("Generates a reply message template")
                    .arg(uid_arg())
                    .arg(reply_all_arg())
                    .args(&tpl_args()),
            )
            .subcommand(
                clap::SubCommand::with_name("forward")
                    .aliases(&["fwd", "fw", "f"])
                    .about("Generates a forward message template")
                    .arg(uid_arg())
                    .args(&tpl_args()),
            ),
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

        ("template", Some(matches)) => Ok(msg_matches_tpl(ctx, matches)?),
        ("list", opt_matches) => msg_matches_list(ctx, opt_matches),
        (_other, opt_matches) => msg_matches_list(ctx, opt_matches),
    }
}

// == Argument Functions ==
/// Returns an Clap-Argument to be able to use `<UID>` in the commandline like
/// for the `himalaya read` subcommand.
pub(crate) fn uid_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("uid")
        .help("Specifies the targetted message")
        .value_name("UID")
        .required(true)
}

fn reply_all_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("reply-all")
        .help("Includes all recipients")
        .short("A")
        .long("all")
}

fn page_size_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("page-size")
        .help("Page size")
        .short("s")
        .long("size")
        .value_name("INT")
}

fn page_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("page")
        .help("Page number")
        .short("p")
        .long("page")
        .value_name("INT")
        .default_value("0")
}

fn attachment_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("attachments")
        .help("Adds attachment to the message")
        .short("a")
        .long("attachment")
        .value_name("PATH")
        .multiple(true)
}

fn tpl_args<'a>() -> Vec<clap::Arg<'a, 'a>> {
    vec![
        clap::Arg::with_name("subject")
            .help("Overrides the Subject header")
            .short("s")
            .long("subject")
            .value_name("STRING"),
        clap::Arg::with_name("from")
            .help("Overrides the From header")
            .short("f")
            .long("from")
            .value_name("ADDR")
            .multiple(true),
        clap::Arg::with_name("to")
            .help("Overrides the To header")
            .short("t")
            .long("to")
            .value_name("ADDR")
            .multiple(true),
        clap::Arg::with_name("cc")
            .help("Overrides the Cc header")
            .short("c")
            .long("cc")
            .value_name("ADDR")
            .multiple(true),
        clap::Arg::with_name("bcc")
            .help("Overrides the Bcc header")
            .short("b")
            .long("bcc")
            .value_name("ADDR")
            .multiple(true),
        clap::Arg::with_name("header")
            .help("Overrides a specific header")
            .short("h")
            .long("header")
            .value_name("KEY: VAL")
            .multiple(true),
        clap::Arg::with_name("body")
            .help("Overrides the body")
            .short("B")
            .long("body")
            .value_name("STRING"),
        clap::Arg::with_name("signature")
            .help("Overrides the signature")
            .short("S")
            .long("signature")
            .value_name("STRING"),
    ]
}

// == Match functions ==
fn msg_matches_list(ctx: &Ctx, opt_matches: Option<&clap::ArgMatches>) -> Result<bool> {
    debug!("list command matched");

    let page_size: usize = opt_matches
        .and_then(|matches| matches.value_of("page-size").and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| ctx.config.default_page_size(&ctx.account));
    debug!("page size: {:?}", page_size);
    let page: usize = opt_matches
        .and_then(|matches| matches.value_of("page").unwrap().parse().ok())
        .map(|page| 1.max(page) - 1)
        .unwrap_or_default();
    debug!("page: {}", &page);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msgs = imap_conn.list_msgs(&ctx.mbox, &page_size, &page)?;
    let msgs = if let Some(ref fetches) = msgs {
        Msgs::try_from(fetches)?
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
        .map(|page| 1.max(page) - 1)
        .unwrap_or(1);
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
        Msgs::try_from(fetches)?
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
    let msg = imap_conn.get_msg(&ctx.mbox, &uid)?;

    if raw {
        ctx.output.print(msg.get_raw_as_string()?);
    } else {
        ctx.output.print(MsgSerialized::try_from(&msg)?);
    }
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_attachments(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("attachments command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);

    // get the msg and than it's attachments
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = imap_conn.get_msg(&ctx.mbox, &uid)?;
    let attachments = msg.attachments.clone();

    debug!(
        "{} attachment(s) found for message {}",
        &attachments.len(),
        &uid
    );

    // Iterate through all attachments and download them to the download
    // directory of the account.
    for attachment in &attachments {
        let filepath = ctx
            .config
            .downloads_filepath(&ctx.account, &attachment.filename);

        debug!("downloading {}…", &attachment.filename);

        fs::write(&filepath, &attachment.body_raw)
            .chain_err(|| format!("Could not save attachment {:?}", filepath))?;
    }

    debug!(
        "{} attachment(s) successfully downloaded",
        &attachments.len()
    );

    ctx.output.print(format!(
        "{} attachment(s) successfully downloaded",
        &attachments.len()
    ));

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_write(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("write command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;

    // create the new msg
    // TODO: Make the header starting customizeable like from template
    let mut msg = Msg::new_with_headers(
        &ctx,
        Headers {
            subject: Some(String::new()),
            to: Vec::new(),
            ..Headers::default()
        },
    );

    // take care of the attachments
    let attachment_paths: Vec<&str> = matches
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    attachment_paths
        .iter()
        .for_each(|path| msg.add_attachment(path));

    msg_interaction(&ctx, &mut msg, &mut imap_conn)?;

    // let's be nice to the server and say "bye" to the server
    imap_conn.logout();

    Ok(true)
}

fn msg_matches_reply(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("reply command matched");

    // -- Preparations --
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let mut msg = imap_conn.get_msg(&ctx.mbox, &uid)?;

    debug!("uid: {}", uid);

    // Change the msg to a reply-msg.
    msg.change_to_reply(&ctx, matches.is_present("reply-all"))?;

    // Apply the given attachments to the reply-msg.
    let attachments: Vec<&str> = matches
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    attachments.iter().for_each(|path| msg.add_attachment(path));

    debug!("found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    msg_interaction(&ctx, &mut msg, &mut imap_conn)?;

    imap_conn.logout();
    Ok(true)
}

pub fn msg_matches_mailto(ctx: &Ctx, url: &Url) -> Result<()> {
    debug!("mailto command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;

    let mut cc = Vec::new();
    let mut bcc = Vec::new();
    let mut subject = Cow::default();
    let mut body = Cow::default();

    for (key, val) in url.query_pairs() {
        match key.as_bytes() {
            b"cc" => {
                cc.push(val.into());
            }
            b"bcc" => {
                bcc.push(val.into());
            }
            b"subject" => {
                subject = val;
            }
            b"body" => {
                body = val;
            }
            _ => (),
        }
    }

    let headers = Headers {
        from: vec![ctx.config.address(&ctx.account)],
        to: vec![url.path().to_string()],
        encoding: ContentTransferEncoding::Base64,
        bcc: Some(bcc),
        cc: Some(cc),
        signature: ctx.config.signature(&ctx.account),
        subject: Some(subject.into()),
        ..Headers::default()
    };

    let mut msg = Msg::new_with_headers(&ctx, headers);
    msg.body = Body::new_with_text(body);
    msg_interaction(&ctx, &mut msg, &mut imap_conn)?;

    imap_conn.logout();
    Ok(())
}

fn msg_matches_forward(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("forward command matched");

    // fetch the msg
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let mut msg = imap_conn.get_msg(&ctx.mbox, &uid)?;

    debug!("uid: {}", uid);

    // prepare to forward it
    msg.change_to_forwarding(&ctx);

    let attachments: Vec<&str> = matches
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    attachments.iter().for_each(|path| msg.add_attachment(path));

    debug!("found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    // apply changes
    msg_interaction(&ctx, &mut msg, &mut imap_conn)?;

    imap_conn.logout();

    Ok(true)
}

fn msg_matches_copy(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("copy command matched");

    // fetch the message to be copyied
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let target = matches.value_of("target").unwrap();
    let mut msg = imap_conn.get_msg(&ctx.mbox, &uid)?;

    debug!("uid: {}", &uid);
    debug!("target: {}", &target);

    // the message, which will be in the new mailbox doesn't need to be seen
    msg.flags.insert(Flag::Seen);

    imap_conn.append_msg(target, &mut msg)?;

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

    // fetch the msg which should be moved
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let target = matches.value_of("target").unwrap();
    let mut msg = imap_conn.get_msg(&ctx.mbox, &uid)?;

    debug!("uid: {}", &uid);
    debug!("target: {}", &target);

    // create the msg in the target-msgbox
    msg.flags.insert(Flag::Seen);
    imap_conn.append_msg(target, &mut msg)?;

    debug!("message {} successfully moved to folder `{}`", uid, target);
    ctx.output.print(format!(
        "Message {} successfully moved to folder `{}`",
        uid, target
    ));

    // delete the msg in the old mailbox
    let flags = vec![Flag::Seen, Flag::Deleted];
    imap_conn.add_flags(&ctx.mbox, uid, Flags::from(flags))?;
    imap_conn.expunge(&ctx.mbox)?;

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_delete(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("delete command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;

    // remove the message according to its UID
    let uid = matches.value_of("uid").unwrap();
    let flags = vec![Flag::Seen, Flag::Deleted];
    imap_conn.add_flags(&ctx.mbox, uid, Flags::from(flags))?;
    imap_conn.expunge(&ctx.mbox)?;

    debug!("message {} successfully deleted", uid);
    ctx.output
        .print(format!("Message {} successfully deleted", uid));

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_send(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("send command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;

    let msg = if atty::is(Stream::Stdin) || ctx.output.is_json() {
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
            .collect::<Vec<String>>()
            .join("\r\n")
    };

    let mut msg = Msg::try_from(msg.as_str())?;

    // send the message/msg
    let sendable = msg.to_sendable_msg()?;
    smtp::send(&ctx.account, &sendable)?;
    debug!("message sent!");

    // add the message/msg to the Sent-Mailbox of the user
    msg.flags.insert(Flag::Seen);
    imap_conn.append_msg("Sent", &mut msg)?;

    imap_conn.logout();

    Ok(true)
}

fn msg_matches_save(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("save command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg: &str = matches.value_of("message").unwrap();

    let mut msg = Msg::try_from(msg)?;

    msg.flags.insert(Flag::Seen);
    imap_conn.append_msg(&ctx.mbox, &mut msg)?;

    imap_conn.logout();

    Ok(true)
}

pub fn msg_matches_tpl(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    match matches.subcommand() {
        ("new", Some(matches)) => tpl_matches_new(ctx, matches),
        ("reply", Some(matches)) => tpl_matches_reply(ctx, matches),
        ("forward", Some(matches)) => tpl_matches_forward(ctx, matches),

        // TODO: find a way to show the help message for template subcommand
        _ => Err("Subcommand not found".into()),
    }
}

// == Helper functions ==
// -- Template Subcommands --
// These functions are more used for the "template" subcommand
fn override_msg_with_args(msg: &mut Msg, matches: &clap::ArgMatches) {
    // -- Collecting credentials --
    let from: Vec<String> = match matches.values_of("from") {
        Some(from) => from.map(|arg| arg.to_string()).collect(),
        None => msg.headers.from.clone(),
    };

    let to: Vec<String> = match matches.values_of("to") {
        Some(to) => to.map(|arg| arg.to_string()).collect(),
        None => Vec::new(),
    };

    let subject = matches
        .value_of("subject")
        .and_then(|subject| Some(subject.to_string()));

    let cc: Option<Vec<String>> = matches
        .values_of("cc")
        .and_then(|cc| Some(cc.map(|arg| arg.to_string()).collect()));

    let bcc: Option<Vec<String>> = matches
        .values_of("bcc")
        .and_then(|bcc| Some(bcc.map(|arg| arg.to_string()).collect()));

    let signature = matches
        .value_of("signature")
        .and_then(|signature| Some(signature.to_string()))
        .or(msg.headers.signature.clone());

    let custom_headers: Option<HashMap<String, Vec<String>>> = {
        if let Some(matched_headers) = matches.values_of("header") {
            let mut custom_headers: HashMap<String, Vec<String>> = HashMap::new();

            // collect the custom headers
            for header in matched_headers {
                let mut header = header.split(":");
                let key = header.next().unwrap_or_default();
                let val = header.next().unwrap_or_default().trim_start();

                debug!("overriden header: {}={}", key, val);

                custom_headers.insert(key.to_string(), vec![val.to_string()]);
            }

            Some(custom_headers)
        } else {
            None
        }
    };

    let body = {
        if atty::isnt(Stream::Stdin) {
            let body = io::stdin()
                .lock()
                .lines()
                .filter_map(|line| line.ok())
                .map(|line| line.to_string())
                .collect::<Vec<String>>()
                .join("\n");
            debug!("overriden body from stdin: {:?}", body);
            body
        } else if let Some(body) = matches.value_of("body") {
            debug!("overriden body: {:?}", body);
            body.to_string()
        } else {
            String::new()
        }
    };

    let body = Body::new_with_text(body);

    // -- Creating and printing --
    let headers = Headers {
        from,
        subject,
        to,
        cc,
        bcc,
        signature,
        custom_headers,
        ..msg.headers.clone()
    };

    msg.headers = headers;
    msg.body = body;
}

fn tpl_matches_new(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("new command matched");

    let mut msg = Msg::new(&ctx);

    override_msg_with_args(&mut msg, &matches);

    trace!("Message: {:?}", msg);
    ctx.output.print(MsgSerialized::try_from(&msg)?);

    Ok(true)
}

fn tpl_matches_reply(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("reply command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let mut msg = imap_conn.get_msg(&ctx.mbox, &uid)?;

    msg.change_to_reply(&ctx, matches.is_present("reply-all"))?;

    override_msg_with_args(&mut msg, &matches);
    trace!("Message: {:?}", msg);
    ctx.output.print(MsgSerialized::try_from(&msg)?);

    Ok(true)
}

fn tpl_matches_forward(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("forward command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let mut msg = imap_conn.get_msg(&ctx.mbox, &uid)?;
    msg.change_to_forwarding(&ctx);

    override_msg_with_args(&mut msg, &matches);

    trace!("Message: {:?}", msg);
    ctx.output.print(MsgSerialized::try_from(&msg)?);

    Ok(true)
}

/// This function opens the prompt to do some actions to the msg like sending, editing it again and
/// so on.
fn msg_interaction(ctx: &Ctx, msg: &mut Msg, imap_conn: &mut ImapConnector) -> Result<bool> {
    // let the user change the body a little bit first, before opening the prompt
    msg.edit_body()?;

    loop {
        match input::post_edit_choice() {
            Ok(choice) => match choice {
                input::PostEditChoice::Send => {
                    debug!("sending message…");

                    // prepare the msg to be send
                    let sendable = match msg.to_sendable_msg() {
                        Ok(sendable) => sendable,
                        // In general if an error occured, then this is normally
                        // due to a missing value of a header. So let's give the
                        // user another try and give him/her the chance to fix
                        // that :)
                        Err(err) => {
                            println!("{}", err);
                            println!("Please reedit your msg to make it to a sendable message!");
                            continue;
                        }
                    };
                    smtp::send(&ctx.account, &sendable)?;

                    // TODO: Gmail sent mailboxes are called `[Gmail]/Sent`
                    // which creates a conflict, fix this!

                    // let the server know, that the user sent a msg
                    msg.flags.insert(Flag::Seen);
                    imap_conn.append_msg("Sent", msg)?;

                    // remove the draft, since we sent it
                    input::remove_draft()?;
                    ctx.output.print("Message successfully sent");
                    break;
                }
                // edit the body of the msg
                input::PostEditChoice::Edit => {
                    // Did something goes wrong when the user changed the
                    // content?
                    if let Err(err) = msg.edit_body() {
                        println!("[ERROR] {}", err);
                        println!(concat!(
                            "Please try to fix the problem by editing",
                            "the msg again."
                        ));
                    }
                }
                input::PostEditChoice::LocalDraft => break,
                input::PostEditChoice::RemoteDraft => {
                    debug!("saving to draft…");

                    msg.flags.insert(Flag::Seen);

                    match imap_conn.append_msg("Drafts", msg) {
                        Ok(_) => {
                            input::remove_draft()?;
                            ctx.output.print("Message successfully saved to Drafts");
                        }
                        Err(err) => {
                            ctx.output.print("Couldn't save it to the server...");
                            return Err(err.into());
                        }
                    };
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

    Ok(true)
}
