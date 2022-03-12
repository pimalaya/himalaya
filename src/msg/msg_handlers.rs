//! Module related to message handling.
//!
//! This module gathers all message commands.  

use anyhow::{Context, Result};
use atty::Stream;
use log::{debug, info, trace};
use mailparse::addrparse;
use std::{
    borrow::Cow,
    fs,
    io::{self, BufRead},
};
use url::Url;

use crate::{
    backends::Backend,
    config::{AccountConfig, DEFAULT_SENT_FOLDER},
    msg::{Msg, Part, Parts, TextPlainPart},
    output::{PrintTableOpts, PrinterService},
    smtp::SmtpService,
};

/// Downloads all message attachments to the user account downloads directory.
pub fn attachments<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let attachments = backend.get_msg(mbox, seq)?.attachments();
    let attachments_len = attachments.len();

    if attachments_len == 0 {
        return printer.print_struct(format!("No attachment found for message {:?}", seq));
    }

    printer.print_str(format!(
        "Found {:?} attachment{} for message {:?}",
        attachments_len,
        if attachments_len > 1 { "s" } else { "" },
        seq
    ))?;

    for attachment in attachments {
        let file_path = config.get_download_file_path(&attachment.filename)?;
        printer.print_str(format!("Downloading {:?}…", file_path))?;
        fs::write(&file_path, &attachment.content)
            .context(format!("cannot download attachment {:?}", file_path))?;
    }

    printer.print_struct(format!(
        "Attachment{} successfully downloaded to {:?}",
        if attachments_len > 1 { "s" } else { "" },
        config.downloads_dir
    ))
}

/// Copy a message from a mailbox to another.
pub fn copy<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox_src: &str,
    mbox_dst: &str,
    printer: &mut P,
    backend: Box<&mut B>,
) -> Result<()> {
    backend.copy_msg(mbox_src, mbox_dst, seq)?;
    printer.print_struct(format!(
        r#"Message {} successfully copied to folder "{}""#,
        seq, mbox_dst
    ))
}

/// Delete messages matching the given sequence range.
pub fn delete<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox: &str,
    printer: &mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    backend.del_msg(mbox, seq)?;
    printer.print_struct(format!(r#"Message(s) {} successfully deleted"#, seq))
}

/// Forward the given message UID from the selected mailbox.
pub fn forward<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
    seq: &str,
    attachments_paths: Vec<&str>,
    encrypt: bool,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
    smtp: &mut S,
) -> Result<()> {
    backend
        .get_msg(mbox, seq)?
        .into_forward(config)?
        .add_attachments(attachments_paths)?
        .encrypt(encrypt)
        .edit_with_editor(config, printer, backend, smtp)?;
    Ok(())
}

/// List paginated messages from the selected mailbox.
pub fn list<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    imap: Box<&'a mut B>,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.default_page_size);
    debug!("page size: {}", page_size);
    let msgs = imap.get_envelopes(mbox, page_size, page)?;
    trace!("envelopes: {:?}", msgs);
    printer.print_table(
        msgs,
        PrintTableOpts {
            format: &config.format,
            max_width,
        },
    )
}

/// Parses and edits a message from a [mailto] URL string.
///
/// [mailto]: https://en.wikipedia.org/wiki/Mailto
pub fn mailto<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
    url: &Url,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
    smtp: &mut S,
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

    let msg = Msg {
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
        ..Msg::default()
    };
    trace!("message: {:?}", msg);

    msg.edit_with_editor(config, printer, backend, smtp)?;
    Ok(())
}

/// Move a message from a mailbox to another.
pub fn move_<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq: &str,
    mbox_src: &str,
    mbox_dst: &str,
    printer: &mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    backend.move_msg(mbox_src, mbox_dst, seq)?;
    printer.print_struct(format!(
        r#"Message {} successfully moved to folder "{}""#,
        seq, mbox_dst
    ))
}

/// Read a message by its sequence number.
pub fn read<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq: &str,
    text_mime: &str,
    raw: bool,
    headers: Vec<&str>,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let msg = backend.get_msg(mbox, seq)?;

    printer.print_struct(if raw {
        // Emails don't always have valid utf8. Using "lossy" to display what we can.
        String::from_utf8_lossy(&msg.raw).into_owned()
    } else {
        msg.to_readable_string(text_mime, headers, config)?
    })
}

/// Reply to the given message UID.
pub fn reply<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
    seq: &str,
    all: bool,
    attachments_paths: Vec<&str>,
    encrypt: bool,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
    smtp: &mut S,
) -> Result<()> {
    backend
        .get_msg(mbox, seq)?
        .into_reply(all, config)?
        .add_attachments(attachments_paths)?
        .encrypt(encrypt)
        .edit_with_editor(config, printer, backend, smtp)?
        .add_flags(mbox, seq, "replied")
}

/// Saves a raw message to the targetted mailbox.
pub fn save<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    mbox: &str,
    raw_msg: &str,
    printer: &mut P,
    backend: Box<&mut B>,
) -> Result<()> {
    info!("entering save message handler");

    debug!("mailbox: {}", mbox);

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
    backend.add_msg(mbox, raw_msg.as_bytes(), "seen")?;
    Ok(())
}

/// Paginate messages from the selected mailbox matching the specified query.
pub fn search<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.default_page_size);
    debug!("page size: {}", page_size);
    let msgs = backend.search_envelopes(mbox, &query, "", page_size, page)?;
    trace!("messages: {:#?}", msgs);
    printer.print_table(
        msgs,
        PrintTableOpts {
            format: &config.format,
            max_width,
        },
    )
}

/// Paginates messages from the selected mailbox matching the specified query, sorted by the given criteria.
pub fn sort<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    sort: String,
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
    mbox: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.default_page_size);
    debug!("page size: {}", page_size);
    let msgs = backend.search_envelopes(mbox, &query, &sort, page_size, page)?;
    trace!("envelopes: {:#?}", msgs);
    printer.print_table(
        msgs,
        PrintTableOpts {
            format: &config.format,
            max_width,
        },
    )
}

/// Send a raw message.
pub fn send<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
    raw_msg: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&mut B>,
    smtp: &mut S,
) -> Result<()> {
    info!("entering send message handler");

    let is_tty = atty::is(Stream::Stdin);
    debug!("is tty: {}", is_tty);
    let is_json = printer.is_json();
    debug!("is json: {}", is_json);

    let sent_folder = config
        .mailboxes
        .get("sent")
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_SENT_FOLDER);
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
    let msg = Msg::from_tpl(&raw_msg)?;
    smtp.send(&config, &msg)?;
    backend.add_msg(&sent_folder, raw_msg.as_bytes(), "seen")?;
    Ok(())
}

/// Compose a new message.
pub fn write<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
    attachments_paths: Vec<&str>,
    encrypt: bool,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
    smtp: &mut S,
) -> Result<()> {
    Msg::default()
        .add_attachments(attachments_paths)?
        .encrypt(encrypt)
        .edit_with_editor(config, printer, backend, smtp)?;
    Ok(())
}
