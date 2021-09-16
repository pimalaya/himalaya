use anyhow::{anyhow, Context, Result};
use atty::Stream;
use clap;
use imap::types::Flag;
use lettre::message::header::ContentTransferEncoding;
use log::{debug, error, trace};
use std::{
    borrow::Cow,
    collections::HashMap,
    convert::TryFrom,
    fs,
    io::{self, BufRead},
};
use url::Url;

use super::{
    body::Body,
    entity::{Msg, MsgSerialized, Msgs},
    headers::Headers,
};
use crate::{
    domain::{
        account::entity::Account,
        imap::service::ImapServiceInterface,
        mbox::{cli::mbox_target_arg, entity::Mbox},
        smtp::service::SmtpServiceInterface,
    },
    flag::model::Flags,
    input,
    output::service::{OutputService, OutputServiceInterface},
};

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

pub fn matches<ImapService: ImapServiceInterface, SmtpService: SmtpServiceInterface>(
    arg_matches: &clap::ArgMatches,
    mbox: &Mbox,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<bool> {
    match arg_matches.subcommand() {
        ("attachments", Some(matches)) => {
            msg_matches_attachments(&output, &account, &matches, imap)
        }
        ("copy", Some(matches)) => msg_matches_copy(&output, &matches, imap),
        ("delete", Some(matches)) => msg_matches_delete(&output, &matches, imap),
        ("forward", Some(matches)) => msg_matches_forward(&output, &account, &matches, imap, smtp),
        ("move", Some(matches)) => msg_matches_move(&output, &matches, imap),
        ("read", Some(matches)) => msg_matches_read(&output, &matches, imap),
        ("reply", Some(matches)) => msg_matches_reply(&output, &account, &matches, imap, smtp),
        ("save", Some(matches)) => msg_matches_save(&mbox, matches, imap),
        ("search", Some(matches)) => msg_matches_search(&output, &account, &matches, imap),
        ("send", Some(matches)) => msg_matches_send(&output, &matches, imap, smtp),
        ("write", Some(matches)) => msg_matches_write(&output, &account, &matches, imap, smtp),

        ("template", Some(matches)) => Ok(msg_matches_tpl(&output, &account, &matches, imap)?),
        ("list", opt_matches) => msg_matches_list(&output, &account, opt_matches, imap),
        (_other, opt_matches) => msg_matches_list(&output, &account, opt_matches, imap),
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

fn msg_matches_list<ImapService: ImapServiceInterface>(
    output: &OutputService,
    account: &Account,
    opt_matches: Option<&clap::ArgMatches>,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("list command matched");

    let page_size: usize = opt_matches
        .and_then(|matches| matches.value_of("page-size").and_then(|s| s.parse().ok()))
        .unwrap_or(account.default_page_size);
    debug!("page size: {:?}", page_size);
    let page: usize = opt_matches
        .and_then(|matches| matches.value_of("page").unwrap().parse().ok())
        .map(|page| 1.max(page) - 1)
        .unwrap_or_default();
    debug!("page: {}", &page);

    let msgs = imap.list_msgs(&page_size, &page)?;
    let msgs = if let Some(ref fetches) = msgs {
        Msgs::try_from(fetches)?
    } else {
        Msgs::new()
    };

    trace!("messages: {:?}", msgs);

    output.print(msgs)?;

    imap.logout()?;
    Ok(true)
}

fn msg_matches_search<ImapService: ImapServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("search command matched");

    let page_size: usize = matches
        .value_of("page-size")
        .and_then(|s| s.parse().ok())
        .unwrap_or(account.default_page_size);
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

    let msgs = imap.search_msgs(&query, &page_size, &page)?;
    let msgs = if let Some(ref fetches) = msgs {
        Msgs::try_from(fetches)?
    } else {
        Msgs::new()
    };
    trace!("messages: {:?}", msgs);
    output.print(msgs)?;

    imap.logout()?;
    Ok(true)
}

fn msg_matches_read<ImapService: ImapServiceInterface>(
    output: &OutputService,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("read command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);
    let mime = format!("text/{}", matches.value_of("mime-type").unwrap());
    debug!("mime: {}", mime);
    let raw = matches.is_present("raw");
    debug!("raw: {}", raw);

    let msg = imap.get_msg(&uid)?;
    if raw {
        output.print(msg.get_raw_as_string()?)?;
    } else {
        output.print(MsgSerialized::try_from(&msg)?)?;
    }
    imap.logout()?;
    Ok(true)
}

fn msg_matches_attachments<ImapService: ImapServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("attachments command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);

    // get the msg and than it's attachments
    let msg = imap.get_msg(&uid)?;
    let attachments = msg.attachments.clone();

    debug!(
        "{} attachment(s) found for message {}",
        &attachments.len(),
        &uid
    );

    // Iterate through all attachments and download them to the download
    // directory of the account.
    for attachment in &attachments {
        let filepath = account.downloads_dir.join(&attachment.filename);
        debug!("downloading {}…", &attachment.filename);
        fs::write(&filepath, &attachment.body_raw)
            .with_context(|| format!("cannot save attachment {:?}", filepath))?;
    }

    debug!(
        "{} attachment(s) successfully downloaded",
        &attachments.len()
    );

    output.print(format!(
        "{} attachment(s) successfully downloaded",
        &attachments.len()
    ))?;

    imap.logout()?;
    Ok(true)
}

fn msg_matches_write<ImapService: ImapServiceInterface, SmtpService: SmtpServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<bool> {
    debug!("write command matched");

    // create the new msg
    // TODO: Make the header starting customizeable like from template
    let mut msg = Msg::new_with_headers(
        &account,
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

    msg_interaction(output, &mut msg, imap, smtp)?;

    imap.logout()?;

    Ok(true)
}

fn msg_matches_reply<ImapService: ImapServiceInterface, SmtpService: SmtpServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<bool> {
    debug!("reply command matched");

    // -- Preparations --
    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);
    let mut msg = imap.get_msg(&uid)?;

    // Change the msg to a reply-msg.
    msg.change_to_reply(&account, matches.is_present("reply-all"))?;

    // Apply the given attachments to the reply-msg.
    let attachments: Vec<&str> = matches
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    attachments.iter().for_each(|path| msg.add_attachment(path));

    debug!("found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    msg_interaction(output, &mut msg, imap, smtp)?;

    imap.logout()?;
    Ok(true)
}

pub fn msg_matches_mailto<ImapService: ImapServiceInterface, SmtpService: SmtpServiceInterface>(
    output: &OutputService,
    account: &Account,
    url: &Url,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    debug!("mailto command matched");

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
        from: vec![account.address()],
        to: vec![url.path().to_string()],
        encoding: ContentTransferEncoding::Base64,
        bcc: Some(bcc),
        cc: Some(cc),
        signature: Some(account.signature.to_owned()),
        subject: Some(subject.into()),
        ..Headers::default()
    };

    let mut msg = Msg::new_with_headers(&account, headers);
    msg.body = Body::new_with_text(body);
    msg_interaction(output, &mut msg, imap, smtp)?;

    imap.logout()?;
    Ok(())
}

fn msg_matches_forward<ImapService: ImapServiceInterface, SmtpService: SmtpServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<bool> {
    debug!("forward command matched");

    // fetch the msg
    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);

    let mut msg = imap.get_msg(&uid)?;
    // prepare to forward it
    msg.change_to_forwarding(&account);

    let attachments: Vec<&str> = matches
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    attachments.iter().for_each(|path| msg.add_attachment(path));

    debug!("found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    // apply changes
    msg_interaction(output, &mut msg, imap, smtp)?;

    imap.logout()?;

    Ok(true)
}

fn msg_matches_copy<ImapService: ImapServiceInterface>(
    output: &OutputService,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("copy command matched");

    // fetch the message to be copyied
    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);
    let target = Mbox::try_from(matches.value_of("target"))?;

    let mut msg = imap.get_msg(&uid)?;
    // the message, which will be in the new mailbox doesn't need to be seen
    msg.flags.insert(Flag::Seen);

    imap.append_msg(&target, &mut msg)?;

    debug!("message {} successfully copied to folder `{}`", uid, target);

    output.print(format!(
        "Message {} successfully copied to folder `{}`",
        uid, target
    ))?;

    imap.logout()?;
    Ok(true)
}

fn msg_matches_move<ImapService: ImapServiceInterface>(
    output: &OutputService,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("move command matched");

    // fetch the msg which should be moved
    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", &uid);
    let target = Mbox::try_from(matches.value_of("target"))?;

    let mut msg = imap.get_msg(&uid)?;
    // create the msg in the target-msgbox
    msg.flags.insert(Flag::Seen);
    imap.append_msg(&target, &mut msg)?;

    debug!("message {} successfully moved to folder `{}`", uid, target);
    output.print(format!(
        "Message {} successfully moved to folder `{}`",
        uid, target
    ))?;

    // delete the msg in the old mailbox
    let flags = vec![Flag::Seen, Flag::Deleted];
    imap.add_flags(uid, Flags::from(flags))?;
    imap.expunge()?;
    imap.logout()?;
    Ok(true)
}

fn msg_matches_delete<ImapService: ImapServiceInterface>(
    output: &OutputService,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("delete command matched");

    // remove the message according to its UID
    let uid = matches.value_of("uid").unwrap();
    let flags = vec![Flag::Seen, Flag::Deleted];
    imap.add_flags(uid, Flags::from(flags))?;
    imap.expunge()?;

    debug!("message {} successfully deleted", uid);
    output.print(format!("Message {} successfully deleted", uid))?;

    imap.logout()?;
    Ok(true)
}

fn msg_matches_send<ImapService: ImapServiceInterface, SmtpService: SmtpServiceInterface>(
    output: &OutputService,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<bool> {
    debug!("send command matched");

    let msg = if atty::is(Stream::Stdin) || output.is_json() {
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
    smtp.send(&sendable)?;
    debug!("message sent!");

    // add the message/msg to the Sent-Mailbox of the user
    msg.flags.insert(Flag::Seen);
    let mbox = Mbox::from("Sent");
    imap.append_msg(&mbox, &mut msg)?;

    imap.logout()?;

    Ok(true)
}

fn msg_matches_save<ImapService: ImapServiceInterface>(
    mbox: &Mbox,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("save command matched");
    let msg: &str = matches.value_of("message").unwrap();
    let mut msg = Msg::try_from(msg)?;
    msg.flags.insert(Flag::Seen);
    imap.append_msg(&mbox, &mut msg)?;
    imap.logout()?;
    Ok(true)
}

pub fn msg_matches_tpl<ImapService: ImapServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    match matches.subcommand() {
        ("new", Some(matches)) => tpl_matches_new(&output, &account, matches),
        ("reply", Some(matches)) => tpl_matches_reply(&output, &account, matches, imap),
        ("forward", Some(matches)) => tpl_matches_forward(&output, &account, matches, imap),

        // TODO: find a way to show the help message for template subcommand
        _ => Err(anyhow!("Subcommand not found")),
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

fn tpl_matches_new(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
) -> Result<bool> {
    debug!("new command matched");
    let mut msg = Msg::new(&account);
    override_msg_with_args(&mut msg, &matches);
    trace!("Message: {:?}", msg);
    output.print(MsgSerialized::try_from(&msg)?)?;
    Ok(true)
}

fn tpl_matches_reply<ImapService: ImapServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("reply command matched");
    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);
    let mut msg = imap.get_msg(&uid)?;
    msg.change_to_reply(&account, matches.is_present("reply-all"))?;
    override_msg_with_args(&mut msg, &matches);
    trace!("Message: {:?}", msg);
    output.print(MsgSerialized::try_from(&msg)?)?;
    Ok(true)
}

fn tpl_matches_forward<ImapService: ImapServiceInterface>(
    output: &OutputService,
    account: &Account,
    matches: &clap::ArgMatches,
    imap: &mut ImapService,
) -> Result<bool> {
    debug!("forward command matched");
    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);
    let mut msg = imap.get_msg(&uid)?;
    msg.change_to_forwarding(&account);
    override_msg_with_args(&mut msg, &matches);
    trace!("Message: {:?}", msg);
    output.print(MsgSerialized::try_from(&msg)?)?;
    Ok(true)
}

/// This function opens the prompt to do some actions to the msg like sending, editing it again and
/// so on.
fn msg_interaction<ImapService: ImapServiceInterface, SmtpService: SmtpServiceInterface>(
    output: &OutputService,
    msg: &mut Msg,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<bool> {
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
                    smtp.send(&sendable)?;

                    // TODO: Gmail sent mailboxes are called `[Gmail]/Sent`
                    // which creates a conflict, fix this!

                    // let the server know, that the user sent a msg
                    msg.flags.insert(Flag::Seen);
                    let mbox = Mbox::from("Sent");
                    imap.append_msg(&mbox, msg)?;

                    // remove the draft, since we sent it
                    input::remove_draft()?;
                    output.print("Message successfully sent")?;
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

                    let mbox = Mbox::from("Drafts");
                    match imap.append_msg(&mbox, msg) {
                        Ok(_) => {
                            input::remove_draft()?;
                            output.print("Message successfully saved to Drafts")?;
                        }
                        Err(err) => {
                            output.print("Cannot save draft to the server")?;
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
