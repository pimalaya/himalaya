use atty::Stream;
use clap;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::{
    fs,
    io::{self, BufRead},
};

use imap::types::Flag;

use crate::{
    ctx::Ctx,
    imap::model::ImapConnector,
    input,
    mbox::cli::mbox_target_arg,
    msg::{
        attachment::Attachment,
        envelope::Envelope,
        mail::{Mail, Mails},
        // model::{Attachments, Msg, Msgs, ReadableMsg},
        // tpl::cli::{tpl_matches, tpl_subcommand},
    },
    smtp,
};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
        Input(crate::input::Error, crate::input::ErrorKind);
        // MsgModel(crate::msg::model::Error, crate::msg::model::ErrorKind);
        MsgModel(super::mail::Error, super::mail::ErrorKind);
        // TplCli(crate::msg::tpl::cli::Error, crate::msg::tpl::cli::ErrorKind);
        Smtp(crate::smtp::Error, crate::smtp::ErrorKind);
        // MailModel(crate::msg::mail::Error, crate::msg::mail::ErrorKind);
    }
    foreign_links {
        Utf8(std::string::FromUtf8Error);
    }
}

// ===================
// Main-Functions
// ===================
pub fn msg_subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
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
        // tpl_subcommand(),
        clap::SubCommand::with_name("test")
            .about("Testing subcommand for the new mail struct")
            .arg(attachment_arg()),
    ]
}

pub fn msg_matches(ctx: &Ctx) -> Result<bool> {
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

        // ("template", Some(matches)) => Ok(tpl_matches(ctx, matches)?),

        ("list", opt_matches) => msg_matches_list(ctx, opt_matches),
        (_other, opt_matches) => msg_matches_list(ctx, opt_matches),
    }
}

// =======================
// Argument Functions
// =======================
pub fn uid_arg<'a>() -> clap::Arg<'a, 'a> {
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

// ====================
// Match functions
// ====================
fn msg_matches_list(ctx: &Ctx, opt_matches: Option<&clap::ArgMatches>) -> Result<bool> {
    debug!("List command matched");

    let page_size: usize = opt_matches
        .and_then(|matches| matches.value_of("page-size").and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| ctx.config.default_page_size(&ctx.account));
    debug!("Page size: {:?}", page_size);
    let page: usize = opt_matches
        .and_then(|matches| matches.value_of("page").unwrap().parse().ok())
        .unwrap_or_default();
    debug!("Page: {}", &page);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msgs = imap_conn.list_msgs(&ctx.mbox, &page_size, &page)?;
    let msgs = if let Some(ref fetches) = msgs {
        Mails::from(fetches)
    } else {
        Mails::new()
    };

    trace!("messages: {:?}", msgs);
    ctx.output.print(msgs);

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_search(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Search command matched");

    let page_size: usize = matches
        .value_of("page-size")
        .and_then(|s| s.parse().ok())
        .unwrap_or(ctx.config.default_page_size(&ctx.account));
    debug!("Page size: {}", &page_size);
    let page: usize = matches
        .value_of("page")
        .unwrap()
        .parse()
        .unwrap_or_default();
    debug!("Page: {}", &page);

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
    debug!("Query: {}", &page);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msgs = imap_conn.search_msgs(&ctx.mbox, &query, &page_size, &page)?;
    let msgs = if let Some(ref fetches) = msgs {
        Mails::from(fetches)
    } else {
        Mails::new()
    };
    trace!("messages: {:?}", msgs);
    ctx.output.print(msgs);

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_read(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Read command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("Uid: {}", uid);
    let mime = format!("text/{}", matches.value_of("mime-type").unwrap());
    debug!("Mime: {}", mime);
    let raw = matches.is_present("raw");
    debug!("Raw: {}", raw);

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = imap_conn.read_msg(&ctx.mbox, &uid)?;

    let msg = msg.get_body().unwrap();

    if raw {
        ctx.output.print(msg.trim_end_matches("\n"));
    } else {
        ctx.output.print(msg);
    }

    imap_conn.logout();
    Ok(true)
}

fn msg_matches_attachments(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Attachments command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("Uid: {}", &uid);

    // get the mail and than it's attachments
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = imap_conn.read_msg(&ctx.mbox, &uid)?;
    let attachments: Vec<&Attachment> = msg.get_attachments().collect();

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

        debug!("Downloading {}…", &attachment.filename);

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

// fn msg_matches_write_test(ctx: &Ctx, matches: &clap::ArgMatches)->
// Result<bool> {     debug!("[Testing] Write matches");
//
//     //
//     let attachments: Vec<Attachment> = vec![
//         // this adds the body of the mail
//         Attachment::new(
//             "",
//             "text/plain",
//             input::open_editor_with_tpl(&[])?.into_bytes()),
//     ];
//
//     Ok(true)
//
// }

fn msg_matches_write(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Write command matched");

    // prepare the imap server to update the status (for example if the user
    // wants to send the mail)
    let mut imap_conn = ImapConnector::new(&ctx.account)?;

    // -----------------------------
    // Prepare the general mail
    // -----------------------------
    // TODO: Make the header starting customizeable like from template
    let mut mail = Mail::new_with_envelope(
        &ctx.account,
        Envelope {
            subject: Some(String::new()),
            to: Vec::new(),
            ..Envelope::default()
        },
    );

    // ----------------
    // Attachments
    // ----------------
    // Parse the paths from the commandline first
    let attachment_paths: Vec<&str> = matches
        // get the provided arguments after the `--attachments` arg
        // for example if the user called it like that:
        //
        //  himalaya --attachments path1 path2 path3
        //
        // than we will put them all in a vector
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    // now iterate over each path and add the attachments
    attachment_paths
        .iter()
        .for_each(|path| mail.add_attachment(path));

    // ---------------------
    // User Interaction
    // ---------------------
    mail_interaction(&ctx, &mail, &imap_conn);

    // ------------
    // Cleanup
    // ------------
    // let's be nice to the server and say "bye" to the server
    imap_conn.logout();

    Ok(true)
}

fn msg_matches_reply(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Reply command matched");

    // -----------------
    // Preparations
    // -----------------
    // Fetch the mail first, which should be replied to
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let mut msg = imap_conn.read_msg(&ctx.mbox, &uid)?;

    debug!("Uid: {}", uid);

    // ---------------------------
    // Adjust content of mail
    // ---------------------------
    // Change the mail to a reply-mail.
    if matches.is_present("reply-all") {
        msg.change_to_reply(&ctx.account, true);
    } else {
        msg.change_to_reply(&ctx.account, false);
    }

    // ----------------
    // Attachments
    // ----------------
    // Apply the given attachments to the reply-mail.
    let mut attachments: Vec<&str> = matches
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    attachments.iter().for_each(|path| msg.add_attachment(path));

    debug!("Found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    // ---------------------
    // User interaction
    // ---------------------
    mail_interaction(&ctx, &msg, &imap_conn);

    // ------------
    // Cleanup
    // ------------
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_forward(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Forward command matched");

    // ----------------
    // Preparation
    // ----------------
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let msg = imap_conn.read_msg(&ctx.mbox, &uid)?;

    debug!("Uid: {}", uid);

    // ---------------------------
    // Adjust content of mail
    // ---------------------------
    msg.change_to_forwarding();

    // ----------------
    // Attachments
    // ----------------
    let mut attachments: Vec<&str> = matches
        .values_of("attachments")
        .unwrap_or_default()
        .collect();

    attachments.iter().for_each(|path| msg.add_attachment(path));

    debug!("Found {} attachments", attachments.len());
    trace!("attachments: {:?}", attachments);

    // ---------------------
    // User interaction
    // ---------------------
    mail_interaction(&ctx, &msg, &imap_conn);

    // ------------
    // Cleanup
    // ------------
    imap_conn.logout();

    Ok(true)
}

fn msg_matches_copy(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Copy command matched");

    // -----------------
    // Preparations
    // -----------------
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let target = matches.value_of("target").unwrap();
    let msg = imap_conn.read_msg(&ctx.mbox, &uid)?;

    debug!("Uid: {}", &uid);
    debug!("Target: {}", &target);

    // ------------
    // Changes
    // ------------
    // before sending it, mark the new message as seen
    let mut flags = msg.get_flags();
    flags.push(Flag::Seen);

    imap_conn.append_msg(target, &msg.into_bytes().unwrap(), flags)?;

    debug!("Message {} successfully copied to folder `{}`", uid, target);

    ctx.output.print(format!(
        "Message {} successfully copied to folder `{}`",
        uid, target
    ));

    // ------------
    // Cleanup
    // ------------
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_move(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Move command matched");

    // -----------------
    // Preparations
    // -----------------
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();
    let target = matches.value_of("target").unwrap();
    let msg = imap_conn.read_msg(&ctx.mbox, &uid)?;

    debug!("Uid: {}", &uid);
    debug!("Target: {}", &target);

    // -----------
    // Action
    // -----------
    // create the mail in the target-mailbox
    let mut flags = msg.get_flags();
    flags.push(Flag::Seen);
    imap_conn.append_msg(target, &msg.into_bytes().unwrap(), flags)?;

    debug!("Message {} successfully moved to folder `{}`", uid, target);
    ctx.output.print(format!(
        "Message {} successfully moved to folder `{}`",
        uid, target
    ));

    // mark the current mail in the current mailbox as deleted
    imap_conn.add_flags(&ctx.mbox, uid, "\\Seen \\Deleted")?;

    // remove the current mail from the current mailbox
    imap_conn.expunge(&ctx.mbox)?;

    // be nice to the server and say "bye" ;)
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_delete(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Delete command matched");

    // -----------------
    // Preparations
    // -----------------
    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let uid = matches.value_of("uid").unwrap();

    debug!("Uid: {}", &uid);

    // -----------
    // Delete
    // -----------
    imap_conn.add_flags(&ctx.mbox, uid, "\\Seen \\Deleted")?;
    imap_conn.expunge(&ctx.mbox)?;

    debug!("Message {} successfully deleted", uid);
    ctx.output
        .print(format!("Message {} successfully deleted", uid));

    // ------------
    // Cleanup
    // ------------
    imap_conn.logout();
    Ok(true)
}

fn msg_matches_send(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Send command matched");

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
            .collect::<Vec<String>>()
            .join("\r\n")
    };

    let msg = match Mail::new_with_pre_body(&ctx.account, msg.into_bytes()) {
        Ok(msg) => msg,
        Err(_) => return Ok(false),
    };

    let msg = match msg.to_sendable_msg() {
        Ok(sendable) => sendable,
        Err(_) => return Ok(false),
    };

    smtp::send(&ctx.account, &msg)?;
    imap_conn.append_msg("Sent", &msg.formatted(), vec![imap::types::Flag::Seen])?;
    imap_conn.logout();

    Ok(true)
}

fn msg_matches_save(ctx: &Ctx, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("Save command matched");

    let mut imap_conn = ImapConnector::new(&ctx.account)?;
    let msg = matches.value_of("message").unwrap();
    let msg = match Mail::new_with_pre_body(&ctx.account, msg.to_string().into_bytes()) {
        Ok(mail) => mail,
        Err(_) => return Ok(false),
    };

    imap_conn.append_msg(&ctx.mbox, &msg.into_bytes().unwrap(), vec![imap::types::Flag::Seen])?;
    imap_conn.logout();

    Ok(true)
}

// =====================
// Helper functions
// =====================
fn mail_interaction(ctx: &Ctx, mail: &Mail, imap_conn: &ImapConnector) -> Result<bool> {
    loop {
        match input::post_edit_choice() {
            Ok(choice) => match choice {
                input::PostEditChoice::Send => {
                    debug!("Sending message…");

                    // prepare the mail to be send
                    let sendable = match mail.to_sendable_msg() {
                        Ok(sendable) => sendable,
                        // In general if an error occured, then this is normally
                        // due to a missing value of a header. So let's give the
                        // user another try and give him/her the chance to fix
                        // that :)
                        Err(_) => continue,
                    };
                    smtp::send(&ctx.account, &sendable)?;

                    // TODO: Gmail sent mailboxes are called `[Gmail]/Sent`
                    // which creates a conflict, fix this!

                    // let the server know, that the user sent a mail
                    imap_conn.append_msg("Sent", &sendable.formatted(), vec![imap::types::Flag::Seen])?;

                    // remove the draft, since we sent it
                    input::remove_draft()?;
                    ctx.output.print("Message successfully sent");
                    break;
                }
                // edit the body of the mail
                input::PostEditChoice::Edit => {
                    // Did something goes wrong when the user changed the
                    // content?
                    if let Err(err) = mail.edit_body() {
                        println!("[ERROR] {}", err);
                        println!(concat!(
                            "Please try to fix the problem by editing",
                            "the mail again."
                        ));
                    }
                }
                input::PostEditChoice::LocalDraft => break,
                input::PostEditChoice::RemoteDraft => {
                    debug!("Saving to draft…");
                    match mail.into_bytes() {
                        Ok(parsed) => {
                            imap_conn.append_msg("Drafts", &parsed, vec![imap::types::Flag::Seen])?;
                            input::remove_draft()?;
                            ctx.output.print("Message successfully saved to Drafts");
                        }
                        Err(_) => ctx.output.print("Couldn't save it to the server..."),
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
