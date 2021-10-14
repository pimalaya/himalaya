//! Module related to message handling.
//!
//! This module gathers all message commands.  

use anyhow::{Context, Result};
use atty::Stream;
use imap::types::Flag;
use log::{debug, trace};
use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    fs,
    io::{self, BufRead},
};
use url::Url;

use crate::{
    config::Account,
    domain::{
        imap::ImapServiceInterface,
        mbox::Mbox,
        msg::{Flags, Msg, Part, TextPlainPart, Tpl},
        smtp::SmtpServiceInterface,
    },
    output::OutputServiceInterface,
};

/// Download all message attachments to the user account downloads directory.
pub fn attachments<
    'a,
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface<'a>,
>(
    seq: &str,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let attachments = imap.find_msg(&seq)?.attachments();
    let attachments_len = attachments.len();
    debug!(
        r#"{} attachment(s) found for message "{}""#,
        attachments_len, seq
    );

    for attachment in attachments {
        let filepath = account.downloads_dir.join(&attachment.filename);
        debug!("downloading {}…", attachment.filename);
        fs::write(&filepath, &attachment.content)
            .context(format!("cannot download attachment {:?}", filepath))?;
    }

    output.print(format!(
        "{} attachment(s) successfully downloaded to {:?}",
        attachments_len, account.downloads_dir
    ))
}

/// Copy a message from a mailbox to another.
pub fn copy<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    seq: &str,
    mbox: &str,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let mbox = Mbox::new(mbox);
    let msg = imap.find_raw_msg(&seq)?;
    let flags = Flags::try_from(vec![Flag::Seen])?;
    imap.append_raw_msg_with_flags(&mbox, &msg, flags)?;
    output.print(format!(
        r#"Message {} successfully copied to folder "{}""#,
        seq, mbox
    ))
}

/// Delete messages matching the given sequence range.
pub fn delete<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    seq: &str,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let flags = Flags::try_from(vec![Flag::Seen, Flag::Deleted])?;
    imap.add_flags(seq, &flags)?;
    imap.expunge()?;
    output.print(format!(r#"Message(s) {} successfully deleted"#, seq))
}

/// Forward the given message UID from the selected mailbox.
pub fn forward<
    'a,
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface<'a>,
    SmtpService: SmtpServiceInterface,
>(
    seq: &str,
    attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    imap.find_msg(seq)?
        .into_forward(account)?
        .add_attachments(attachments_paths)?
        .edit_with_editor(account, output, imap, smtp)
}

/// List paginated messages from the selected mailbox.
pub fn list<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    page_size: Option<usize>,
    page: usize,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let page_size = page_size.unwrap_or(account.default_page_size);
    trace!("page size: {}", page_size);

    let msgs = imap.get_msgs(&page_size, &page)?;
    trace!("messages: {:#?}", msgs);
    output.print(msgs)
}

/// Parse and edit a message from a [mailto] URL string.
///
/// [mailto]: https://en.wikipedia.org/wiki/Mailto
pub fn mailto<
    'a,
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface<'a>,
    SmtpService: SmtpServiceInterface,
>(
    url: &Url,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    let to: Vec<lettre::message::Mailbox> = url
        .path()
        .split(";")
        .filter_map(|s| s.parse().ok())
        .collect();
    let mut cc = Vec::new();
    let mut bcc = Vec::new();
    let mut subject = Cow::default();
    let mut body = Cow::default();

    for (key, val) in url.query_pairs() {
        match key.as_bytes() {
            b"cc" => {
                cc.push(val.parse()?);
            }
            b"bcc" => {
                bcc.push(val.parse()?);
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

    let mut msg = Msg::default();

    msg.from = Some(vec![account.address().parse()?]);
    msg.to = if to.is_empty() { None } else { Some(to) };
    msg.cc = if cc.is_empty() { None } else { Some(cc) };
    msg.bcc = if bcc.is_empty() { None } else { Some(bcc) };
    msg.subject = subject.into();
    msg.parts.push(Part::TextPlain(TextPlainPart {
        content: body.into(),
    }));
    msg.edit_with_editor(account, output, imap, smtp)
}

/// Move a message from a mailbox to another.
pub fn move_<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    // The sequence number of the message to move
    seq: &str,
    // The mailbox to move the message in
    mbox: &str,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    // Copy the message to targetted mailbox
    let mbox = Mbox::new(mbox);
    let msg = imap.find_raw_msg(&seq)?;
    let flags = Flags::try_from(vec![Flag::Seen])?;
    imap.append_raw_msg_with_flags(&mbox, &msg, flags)?;

    // Delete the original message
    let flags = Flags::try_from(vec![Flag::Seen, Flag::Deleted])?;
    imap.add_flags(seq, &flags)?;
    imap.expunge()?;

    output.print(format!(
        r#"Message {} successfully moved to folder "{}""#,
        seq, mbox
    ))
}

/// Read a message by its sequence number.
pub fn read<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    seq: &str,
    text_mime: &str,
    raw: bool,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let msg = if raw {
        String::from_utf8(imap.find_raw_msg(&seq)?)?
    } else {
        imap.find_msg(&seq)?.fold_text_parts(text_mime)
    };

    output.print(msg)
}

/// Reply to the given message UID.
pub fn reply<
    'a,
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface<'a>,
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
    imap.find_msg(seq)?
        .into_reply(all, account)?
        .add_attachments(attachments_paths)?
        .edit_with_editor(account, output, imap, smtp)?;
    let flags = Flags::try_from(vec![Flag::Answered])?;
    imap.add_flags(seq, &flags)
}

/// Save a raw message to the targetted mailbox.
pub fn save<'a, ImapService: ImapServiceInterface<'a>>(
    mbox: &str,
    msg: &str,
    imap: &mut ImapService,
) -> Result<()> {
    let mbox = Mbox::new(mbox);
    let flags = Flags::try_from(vec![Flag::Seen])?;
    imap.append_raw_msg_with_flags(&mbox, msg.as_bytes(), flags)
}

/// Paginate messages from the selected mailbox matching the specified query.
pub fn search<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    query: String,
    page_size: Option<usize>,
    page: usize,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let page_size = page_size.unwrap_or(account.default_page_size);
    trace!("page size: {}", page_size);

    let msgs = imap.find_msgs(&query, &page_size, &page)?;
    trace!("messages: {:#?}", msgs);
    output.print(msgs)
}

/// Send a raw message.
pub fn send<
    'a,
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface<'a>,
    SmtpService: SmtpServiceInterface,
>(
    raw_msg: &str,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    let raw_msg = if atty::is(Stream::Stdin) || output.is_json() {
        raw_msg.replace("\r", "").replace("\n", "\r\n")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(|ln| ln.ok())
            .map(|ln| ln.to_string())
            .collect::<Vec<String>>()
            .join("\r\n")
    };

    let tpl = Tpl(raw_msg.to_string());
    let msg = Msg::try_from(&tpl)?;
    let envelope: lettre::address::Envelope = msg.try_into()?;
    smtp.send_raw_msg(&envelope, raw_msg.as_bytes())?;
    debug!("message sent!");

    // Save message to sent folder
    let mbox = Mbox::new("Sent");
    let flags = Flags::try_from(vec![Flag::Seen])?;
    imap.append_raw_msg_with_flags(&mbox, raw_msg.as_bytes(), flags)
}

/// Compose a new message.
pub fn write<
    'a,
    OutputService: OutputServiceInterface,
    ImapService: ImapServiceInterface<'a>,
    SmtpService: SmtpServiceInterface,
>(
    attachments_paths: Vec<&str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
    smtp: &mut SmtpService,
) -> Result<()> {
    Msg::default()
        .add_attachments(attachments_paths)?
        .edit_with_editor(account, output, imap, smtp)
}
