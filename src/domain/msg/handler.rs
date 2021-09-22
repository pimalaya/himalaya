//! Module related to message handling.
//!
//! This module gathers all message commands.  

use anyhow::{Context, Result};
use atty::Stream;
use imap::types::Flag;
use lettre::message::header::ContentTransferEncoding;
use log::{debug, trace};
use std::{
    borrow::Cow,
    convert::TryFrom,
    fs,
    io::{self, BufRead},
};
use url::Url;

use crate::{
    config::entity::Account,
    domain::{
        imap::service::ImapServiceInterface,
        mbox::entity::Mbox,
        msg::{self, entity::Msg, header::entity::Headers, Flags},
        smtp::service::SmtpServiceInterface,
    },
    output::service::OutputServiceInterface,
    ui::{
        choice::{self, PostEditChoice},
        editor,
    },
};

// TODO: move this function to the right folder
fn msg_interaction<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    output: &OutputService,
    msg: &mut Msg,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<bool> {
    // let the user change the body a little bit first, before opening the prompt
    msg.edit_body()?;

    loop {
        match choice::post_edit()? {
            PostEditChoice::Send => {
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
                // imap.append_msg(&mbox, msg)?;

                // remove the draft, since we sent it
                msg::utils::remove_draft()?;
                output.print("Message successfully sent")?;
                break;
            }
            // edit the body of the msg
            PostEditChoice::Edit => {
                Msg::parse_from_str(msg, &editor::open_editor_with_draft()?)?;
                continue;
            }
            PostEditChoice::LocalDraft => break,
            PostEditChoice::RemoteDraft => {
                debug!("saving to draft…");

                msg.flags.insert(Flag::Seen);

                let mbox = Mbox::from("Drafts");
                // match imap.append_msg(&mbox, msg) {
                //     Ok(_) => {
                //         msg::utils::remove_draft()?;
                //         output.print("Message successfully saved to Drafts")?;
                //     }
                //     Err(err) => {
                //         output.print("Cannot save draft to the server")?;
                //         return Err(err.into());
                //     }
                // };
                break;
            }
            PostEditChoice::Discard => {
                msg::utils::remove_draft()?;
                break;
            }
        }
    }

    Ok(true)
}

/// Download all attachments from the given message UID to the user account downloads directory.
pub fn attachments<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let msg = imap.get_msg(&uid)?;
    let attachments = msg.attachments.clone();

    debug!(
        "{} attachment(s) found for message {}",
        attachments.len(),
        uid
    );

    for attachment in &attachments {
        let filepath = account.downloads_dir.join(&attachment.filename);
        debug!("downloading {}…", attachment.filename);
        fs::write(&filepath, &attachment.body_raw)
            .context(format!("cannot download attachment {:?}", filepath))?;
    }

    output.print(format!(
        "{} attachment(s) successfully downloaded to {:?}",
        attachments.len(),
        account.downloads_dir
    ))?;

    Ok(())
}

/// Copy the given message UID from the selected mailbox to the targetted mailbox.
pub fn copy<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    mbox: Option<&str>,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let target = Mbox::try_from(mbox)?;
    let mut msg = imap.get_msg(&uid)?;
    msg.flags.insert(Flag::Seen);
    // imap.append_msg(&target, &mut msg)?;
    output.print(format!(
        r#"Message {} successfully copied to folder "{}""#,
        uid, target
    ))?;
    Ok(())
}

/// Delete the given message UID from the selected mailbox.
pub fn delete<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let flags = Flags::try_from(vec![Flag::Seen, Flag::Deleted])?;
    imap.add_flags(uid, &flags)?;
    imap.expunge()?;
    output.print(format!("Message(s) {} successfully deleted", uid))?;
    Ok(())
}

/// Forward the given message UID from the selected mailbox.
pub fn forward<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    uid: &str,
    attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    let mut msg = imap.get_msg(&uid)?.into_forward(&account)?;
    attachments_paths
        .iter()
        .for_each(|path| msg.add_attachment(path));
    debug!("found {} attachments", attachments_paths.len());
    trace!("attachments: {:?}", attachments_paths);
    msg_interaction(output, &mut msg, imap, smtp)?;
    Ok(())
}

/// List paginated messages from the selected mailbox.
pub fn list<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    page_size: Option<usize>,
    page: usize,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    debug!("entering list handler");

    let page_size = page_size.unwrap_or(account.default_page_size);
    trace!("page size: {}", page_size);

    let msgs = imap.get_msgs(&page_size, &page)?;
    trace!("messages: {:#?}", msgs);
    output.print(msgs)?;

    Ok(())
}

/// Parse and edit a message from a [mailto] URL string.
///
/// [mailto]: https://en.wikipedia.org/wiki/Mailto
pub fn mailto<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    url: &Url,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
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
        cc: if cc.is_empty() { None } else { Some(cc) },
        bcc: if bcc.is_empty() { None } else { Some(bcc) },
        subject: Some(subject.into()),
        ..Headers::default()
    };

    // let mut msg = Msg::new_with_headers(&account, headers);
    // msg.body = Body::new_with_text(body);
    // msg_interaction(output, &mut msg, imap, smtp)?;
    Ok(())
}

/// Move the given message UID from the selected mailbox to the targetted mailbox.
pub fn move_<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    mbox: Option<&str>,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let target = Mbox::try_from(mbox)?;
    let mut msg = imap.get_msg(&uid)?;
    // create the msg in the target-msgbox
    msg.flags.insert(Flag::Seen);
    // imap.append_msg(&target, &mut msg)?;
    output.print(format!(
        r#"Message {} successfully moved to folder "{}""#,
        uid, target
    ))?;
    // delete the msg in the old mailbox
    let flags = Flags::try_from(vec![Flag::Seen, Flag::Deleted])?;
    imap.add_flags(uid, &flags)?;
    imap.expunge()?;
    Ok(())
}

/// Read a message from the given UID.
pub fn read<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    // TODO: use the mime to select the right body
    _mime: String,
    raw: bool,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let msg = imap.get_msg(&uid)?;
    if raw {
        output.print(msg.get_raw_as_string()?)?;
    } else {
        // output.print(MsgSerialized::try_from(&msg)?)?;
    }
    Ok(())
}

/// Reply to the given message UID.
pub fn reply<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    uid: &str,
    all: bool,
    attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    let mut msg = imap.get_msg(&uid)?.into_reply(all, &account)?;
    // Apply the given attachments to the reply-msg.
    attachments_paths
        .iter()
        .for_each(|path| msg.add_attachment(path));
    debug!("found {} attachments", attachments_paths.len());
    trace!("attachments: {:#?}", attachments_paths);
    msg_interaction(output, &mut msg, imap, smtp)?;
    Ok(())
}

/// Save a raw message to the targetted mailbox.
pub fn save<ImapService: ImapServiceInterface>(
    mbox: Option<&str>,
    msg: &str,
    imap: &mut ImapService,
) -> Result<()> {
    let mbox = Mbox::try_from(mbox)?;
    let mut msg = Msg::try_from(msg)?;
    msg.flags.insert(Flag::Seen);
    // imap.append_msg(&mbox, &mut msg)?;
    Ok(())
}

/// Paginate messages from the selected mailbox matching the specified query.
pub fn search<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    query: String,
    page_size: Option<usize>,
    page: usize,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    debug!("entering search handler");

    let page_size = page_size.unwrap_or(account.default_page_size);
    trace!("page size: {}", page_size);

    let msgs = imap.find_msgs(&query, &page_size, &page)?;
    trace!("messages: {:#?}", msgs);
    output.print(msgs)?;

    Ok(())
}

/// Send a raw message.
pub fn send<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    msg: &str,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    let msg = if atty::is(Stream::Stdin) || output.is_json() {
        msg.replace("\r", "").replace("\n", "\r\n")
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
    // imap.append_msg(&mbox, &mut msg)?;
    Ok(())
}

/// Compose a new message.
pub fn write<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    // let mut msg = Msg::new_with_headers(
    //     &account,
    //     Headers {
    //         subject: Some(String::new()),
    //         to: Vec::new(),
    //         ..Headers::default()
    //     },
    // );
    // attachments_paths
    //     .iter()
    //     .for_each(|path| msg.add_attachment(path));
    // msg_interaction(output, &mut msg, imap, smtp)?;
    Ok(())
}
