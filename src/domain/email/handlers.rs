//! Module related to message handling.
//!
//! This module gathers all message commands.  

use anyhow::{Context, Result};
use atty::Stream;
use himalaya_lib::{
    AccountConfig, Backend, Email, Part, Parts, Sender, TextPlainPart, TplOverride,
};
use log::{debug, info, trace};
use mailparse::addrparse;
use std::{
    borrow::Cow,
    fs,
    io::{self, BufRead},
};
use url::Url;

use crate::{
    printer::{PrintTableOpts, Printer},
    ui::editor,
};

/// Downloads all message attachments to the user account downloads directory.
pub fn attachments<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let attachments = backend.email_get(mbox, seq)?.attachments();
    let attachments_len = attachments.len();

    if attachments_len == 0 {
        return printer.print_struct(format!("No attachment found for message {}", seq));
    }

    printer.print_str(format!(
        "{} attachment(s) found for message {}",
        attachments_len, seq
    ))?;

    for attachment in attachments {
        let file_path = config.get_download_file_path(&attachment.filename)?;
        printer.print_str(format!("Downloading {:?}…", file_path))?;
        fs::write(&file_path, &attachment.content)
            .context(format!("cannot download attachment {:?}", file_path))?;
    }

    printer.print_struct("Done!")
}

/// Copy a message from a folder to another.
pub fn copy<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox_src: &str,
    mbox_dst: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.email_copy(mbox_src, mbox_dst, seq)?;
    printer.print_struct(format!(
        "Message {} successfully copied to folder {}",
        seq, mbox_dst
    ))
}

/// Delete messages matching the given sequence range.
pub fn delete<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.email_delete(mbox, seq)?;
    printer.print_struct(format!("Message(s) {} successfully deleted", seq))
}

/// Forward the given message UID from the selected folder.
pub fn forward<'a, P: Printer, B: Backend<'a> + ?Sized, S: Sender + ?Sized>(
    seq: &str,
    attachments_paths: Vec<&str>,
    encrypt: bool,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
) -> Result<()> {
    let msg = backend
        .email_get(mbox, seq)?
        .into_forward(config)?
        .add_attachments(attachments_paths)?
        .encrypt(encrypt);
    editor::edit_msg_with_editor(
        msg,
        TplOverride::default(),
        config,
        printer,
        backend,
        sender,
    )?;
    Ok(())
}

/// List paginated messages from the selected folder.
pub fn list<'a, P: Printer, B: Backend<'a> + ?Sized>(
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    debug!("page size: {}", page_size);
    let msgs = backend.envelope_list(mbox, page_size, page)?;
    trace!("envelopes: {:?}", msgs);
    printer.print_table(
        Box::new(msgs),
        PrintTableOpts {
            format: &config.email_reading_format,
            max_width,
        },
    )
}

/// Parses and edits a message from a [mailto] URL string.
///
/// [mailto]: https://en.wikipedia.org/wiki/Mailto
pub fn mailto<'a, P: Printer, B: Backend<'a> + ?Sized, S: Sender + ?Sized>(
    url: &Url,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
) -> Result<()> {
    info!("entering mailto command handler");

    let to = addrparse(url.path())?;
    let mut cc = Vec::new();
    let mut bcc = Vec::new();
    let mut subject = Cow::default();
    let mut body = Cow::default();

    for (key, val) in url.query_pairs() {
        match key.as_bytes() {
            b"cc" => {
                cc.push(val.to_string());
            }
            b"bcc" => {
                bcc.push(val.to_string());
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

    let msg = Email {
        from: Some(vec![config.address()?].into()),
        to: if to.is_empty() { None } else { Some(to) },
        cc: if cc.is_empty() {
            None
        } else {
            Some(addrparse(&cc.join(","))?)
        },
        bcc: if bcc.is_empty() {
            None
        } else {
            Some(addrparse(&bcc.join(","))?)
        },
        subject: subject.into(),
        parts: Parts(vec![Part::TextPlain(TextPlainPart {
            content: body.into(),
        })]),
        ..Email::default()
    };
    trace!("message: {:?}", msg);

    editor::edit_msg_with_editor(
        msg,
        TplOverride::default(),
        config,
        printer,
        backend,
        sender,
    )?;
    Ok(())
}

/// Move a message from a folder to another.
pub fn move_<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox_src: &str,
    mbox_dst: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.email_move(mbox_src, mbox_dst, seq)?;
    printer.print_struct(format!(
        r#"Message {} successfully moved to folder "{}""#,
        seq, mbox_dst
    ))
}

/// Read a message by its sequence number.
pub fn read<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq: &str,
    text_mime: &str,
    raw: bool,
    headers: Vec<&str>,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let msg = backend.email_get(mbox, seq)?;

    printer.print_struct(if raw {
        // Emails don't always have valid utf8. Using "lossy" to display what we can.
        String::from_utf8_lossy(&msg.raw).into_owned()
    } else {
        msg.to_readable_string(text_mime, headers, config)?
    })
}

/// Reply to the given message UID.
pub fn reply<'a, P: Printer, B: Backend<'a> + ?Sized, S: Sender + ?Sized>(
    seq: &str,
    all: bool,
    attachments_paths: Vec<&str>,
    encrypt: bool,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
) -> Result<()> {
    let msg = backend
        .email_get(mbox, seq)?
        .into_reply(all, config)?
        .add_attachments(attachments_paths)?
        .encrypt(encrypt);
    editor::edit_msg_with_editor(
        msg,
        TplOverride::default(),
        config,
        printer,
        backend,
        sender,
    )?;
    backend.flags_add(mbox, seq, "replied")?;
    Ok(())
}

/// Saves a raw message to the targetted folder.
pub fn save<'a, P: Printer, B: Backend<'a> + ?Sized>(
    mbox: &str,
    raw_msg: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    debug!("folder: {}", mbox);

    let is_tty = atty::is(Stream::Stdin);
    debug!("is tty: {}", is_tty);
    let is_json = printer.is_json();
    debug!("is json: {}", is_json);

    let raw_msg = if is_tty || is_json {
        raw_msg.replace("\r", "").replace("\n", "\r\n")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\r\n")
    };
    backend.email_add(mbox, raw_msg.as_bytes(), "seen")?;
    Ok(())
}

/// Paginate messages from the selected folder matching the specified
/// query.
pub fn search<'a, P: Printer, B: Backend<'a> + ?Sized>(
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    debug!("page size: {}", page_size);
    let msgs = backend.envelope_search(mbox, &query, "", page_size, page)?;
    trace!("messages: {:#?}", msgs);
    printer.print_table(
        Box::new(msgs),
        PrintTableOpts {
            format: &config.email_reading_format,
            max_width,
        },
    )
}

/// Paginates messages from the selected folder matching the specified
/// query, sorted by the given criteria.
pub fn sort<'a, P: Printer, B: Backend<'a> + ?Sized>(
    sort: String,
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    debug!("page size: {}", page_size);
    let msgs = backend.envelope_search(mbox, &query, &sort, page_size, page)?;
    trace!("envelopes: {:#?}", msgs);
    printer.print_table(
        Box::new(msgs),
        PrintTableOpts {
            format: &config.email_reading_format,
            max_width,
        },
    )
}

/// Send a raw message.
pub fn send<'a, P: Printer, B: Backend<'a> + ?Sized, S: Sender + ?Sized>(
    raw_msg: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
) -> Result<()> {
    info!("entering send message handler");

    let is_tty = atty::is(Stream::Stdin);
    debug!("is tty: {}", is_tty);
    let is_json = printer.is_json();
    debug!("is json: {}", is_json);

    let sent_folder = config.folder_alias("sent")?;
    debug!("sent folder: {:?}", sent_folder);

    let raw_msg = if is_tty || is_json {
        raw_msg.replace("\r", "").replace("\n", "\r\n")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\r\n")
    };
    trace!("raw message: {:?}", raw_msg);
    let msg = Email::from_tpl(&raw_msg)?;
    sender.send(&config, &msg)?;
    backend.email_add(&sent_folder, raw_msg.as_bytes(), "seen")?;
    Ok(())
}

/// Compose a new message.
pub fn write<'a, P: Printer, B: Backend<'a> + ?Sized, S: Sender + ?Sized>(
    tpl: TplOverride,
    attachments_paths: Vec<&str>,
    encrypt: bool,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
) -> Result<()> {
    let msg = Email::default()
        .add_attachments(attachments_paths)?
        .encrypt(encrypt);
    editor::edit_msg_with_editor(msg, tpl, config, printer, backend, sender)?;
    Ok(())
}
