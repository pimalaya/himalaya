//! Module related to message handling.
//!
//! This module gathers all message commands.  

use anyhow::{Context, Result};
use imap::types::Flag;
use lettre::message::header::ContentTransferEncoding;
use log::{debug, trace};
use std::{borrow::Cow, convert::TryFrom, fs};
use url::Url;

use crate::{
    config::entity::Account,
    domain::{
        imap::service::ImapServiceInterface,
        mbox::entity::Mbox,
        msg::{header::entity::Headers, Flags, Msg},
        smtp::service::SmtpServiceInterface,
    },
    output::service::OutputServiceInterface,
};

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

/// Copy a message from a mailbox to another.
pub fn copy<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    // The sequence number of the message to copy
    seq: &str,
    // The mailbox to copy the message in
    target: Option<&str>,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let target = Mbox::try_from(target)?;
    let mut msg = imap.find_msg(&seq)?;

    // Append message to targetted mailbox
    msg.flags.insert(Flag::Seen);
    imap.append_msg(&target, msg)?;

    output.print(format!(
        r#"Message {} successfully copied to folder "{}""#,
        seq, target
    ))?;
    Ok(())
}

/// Delete messages matching the given sequence range.
pub fn delete<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    seq: &str,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    imap.add_flags(seq, &Flags::try_from(vec![Flag::Seen, Flag::Deleted])?)?;
    imap.expunge()?;
    output.print(format!("Message(s) {} successfully deleted", seq))?;
    Ok(())
}

/// Forward the given message UID from the selected mailbox.
pub fn forward<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    seq: &str,
    attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    debug!("entering forward handler");
    imap.find_msg(seq)?
        .into_forward(account)?
        .edit(account, output, imap, smtp)?;
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

/// Move a message from a mailbox to another.
pub fn move_<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    // The sequence number of the message to move
    seq: &str,
    // The mailbox to move the message in
    target: Option<&str>,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let target = Mbox::try_from(target)?;
    let mut msg = imap.find_msg(&seq)?;

    // Append message to targetted mailbox
    msg.flags.insert(Flag::Seen);
    imap.append_msg(&target, msg)?;

    // Flag as deleted the original message
    let flags = Flags::try_from(vec![Flag::Seen, Flag::Deleted])?;
    imap.add_flags(seq, &flags)?;
    imap.expunge()?;

    output.print(format!(
        r#"Message {} successfully moved to folder "{}""#,
        seq, target
    ))?;
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
    seq: &str,
    all: bool,
    attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    debug!("entering reply handler");
    imap.find_msg(seq)?
        .into_reply(all, account)?
        .edit(account, output, imap, smtp)?;
    Ok(())
}

/// Save a raw message to the targetted mailbox.
pub fn save<ImapService: ImapServiceInterface>(
    mbox: Option<&str>,
    msg: &str,
    imap: &mut ImapService,
) -> Result<()> {
    // let mbox = Mbox::try_from(mbox)?;
    // let mut msg = Msg::try_from(msg)?;
    // msg.flags.insert(Flag::Seen);
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
    // let msg = if atty::is(Stream::Stdin) || output.is_json() {
    //     msg.replace("\r", "").replace("\n", "\r\n")
    // } else {
    //     io::stdin()
    //         .lock()
    //         .lines()
    //         .filter_map(|ln| ln.ok())
    //         .map(|ln| ln.to_string())
    //         .collect::<Vec<String>>()
    //         .join("\r\n")
    // };
    // let mut msg = Msg::try_from(msg.as_str())?;
    // send the message/msg
    // let sendable = msg.to_sendable_msg()?;
    // smtp.send(&sendable)?;
    // debug!("message sent!");
    // add the message/msg to the Sent-Mailbox of the user
    // msg.flags.insert(Flag::Seen);
    // let mbox = Mbox::from("Sent");
    // imap.append_msg(&mbox, &mut msg)?;
    Ok(())
}

/// Compose a new message.
pub fn write<
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface,
    SmtpService: SmtpServiceInterface,
>(
    // FIXME
    _attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    debug!("entering write handler");
    Msg::default().edit(account, output, imap, smtp)?;
    Ok(())
}
