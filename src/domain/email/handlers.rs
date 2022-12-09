use anyhow::{Context, Result};
use atty::Stream;
use himalaya_lib::{AccountConfig, Backend, Email, Sender, ShowTextPartsStrategy, Tpl, TplBuilder};
use log::{debug, trace};
use std::{
    fs,
    io::{self, BufRead},
};
use url::Url;
use uuid::Uuid;

use crate::{
    printer::{PrintTableOpts, Printer},
    ui::editor,
};

pub fn attachments<P: Printer, B: Backend + ?Sized>(
    id: &str,
    folder: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let attachments = backend.get_email(folder, id)?.attachments()?;

    if attachments.is_empty() {
        return printer.print(format!("No attachment found for email {}", id));
    }

    printer.print_log(format!(
        "{} attachment(s) found for email {}",
        attachments.len(),
        id
    ))?;

    for attachment in attachments {
        let filename = attachment
            .filename
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let filepath = config.get_download_file_path(&filename)?;
        printer.print_log(format!("Downloading {:?}…", filepath))?;
        fs::write(&filepath, &attachment.body)
            .context(format!("cannot download attachment {:?}", filepath))?;
    }

    printer.print("Done!")
}

pub fn copy<P: Printer, B: Backend + ?Sized>(
    id: &str,
    folder: &str,
    folder_target: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.copy_email(folder, folder_target, id)?;
    printer.print(format!(
        "Email {} successfully copied to folder {}",
        id, folder_target
    ))
}

pub fn delete<P: Printer, B: Backend + ?Sized>(
    id: &str,
    folder: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.delete_email(folder, id)?;
    printer.print("Email successfully deleted!")
}

pub fn forward<P: Printer, B: Backend + ?Sized, S: Sender + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
    folder: &str,
    id: &str,
    headers: Option<Vec<&str>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl = backend
        .get_email(folder, id)?
        .to_forward_tpl_builder(config)?
        .set_some_raw_headers(headers)
        .some_text_plain_part(body)
        .build();
    trace!("initial template: {}", *tpl);
    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl)?;
    Ok(())
}

pub fn list<P: Printer, B: Backend + ?Sized>(
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
    folder: &str,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    debug!("page size: {}", page_size);
    let msgs = backend.list_envelope(folder, page_size, page)?;
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
pub fn mailto<P: Printer, B: Backend + ?Sized, S: Sender + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
    url: &Url,
) -> Result<()> {
    let mut tpl = TplBuilder::default().to(url.path());

    for (key, val) in url.query_pairs() {
        match key.to_lowercase().as_bytes() {
            b"cc" => tpl = tpl.cc(val),
            b"bcc" => tpl = tpl.bcc(val),
            b"subject" => tpl = tpl.subject(val),
            b"body" => tpl = tpl.text_plain_part(val.as_bytes()),
            _ => (),
        }
    }

    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl.build())
}

pub fn move_<P: Printer, B: Backend + ?Sized>(
    id: &str,
    folder: &str,
    folder_target: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.move_email(folder, folder_target, id)?;
    printer.print(format!(
        "Email {} successfully moved to folder {}",
        id, folder_target
    ))
}

pub fn read<P: Printer, B: Backend + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    id: &str,
    text_mime: &str,
    sanitize: bool,
    raw: bool,
    headers: Vec<&str>,
) -> Result<()> {
    let mut email = backend.get_email(folder, id)?;

    if raw {
        // emails do not always have valid utf8, uses "lossy" to
        // display what can be displayed
        let raw_email = String::from_utf8_lossy(email.as_raw()?).into_owned();
        return printer.print(raw_email);
    }

    let tpl = email
        .to_read_tpl_builder()?
        .show_headers(config.email_reading_headers())
        .show_headers(headers)
        .show_text_parts_only(true)
        .use_show_text_parts_strategy(if text_mime == "plain" {
            ShowTextPartsStrategy::PlainOtherwiseHtml
        } else {
            ShowTextPartsStrategy::HtmlOtherwisePlain
        })
        .sanitize_text_parts(sanitize)
        .build();

    printer.print(<Tpl as Into<String>>::into(tpl))
}

pub fn reply<P: Printer, B: Backend + ?Sized, S: Sender + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
    folder: &str,
    id: &str,
    all: bool,
    headers: Option<Vec<&str>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl = backend
        .get_email(folder, id)?
        .to_reply_tpl_builder(config, all)?
        .set_some_raw_headers(headers)
        .some_text_plain_part(body)
        .build();
    trace!("initial template: {}", *tpl);
    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl)?;
    backend.add_flags(folder, id, "replied")?;
    Ok(())
}

pub fn save<P: Printer, B: Backend + ?Sized>(
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    raw_email: &str,
) -> Result<()> {
    let is_tty = atty::is(Stream::Stdin);
    let is_json = printer.is_json();
    let raw_email = if is_tty || is_json {
        raw_email.replace("\r", "").replace("\n", "\r\n")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\r\n")
    };
    backend.add_email(folder, raw_email.as_bytes(), "seen")?;
    Ok(())
}

pub fn search<P: Printer, B: Backend + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    let envelopes = backend.search_envelope(folder, &query, "", page_size, page)?;
    let opts = PrintTableOpts {
        format: &config.email_reading_format,
        max_width,
    };

    printer.print_table(Box::new(envelopes), opts)
}

pub fn sort<P: Printer, B: Backend + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    sort: String,
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    let envelopes = backend.search_envelope(folder, &query, &sort, page_size, page)?;
    let opts = PrintTableOpts {
        format: &config.email_reading_format,
        max_width,
    };

    printer.print_table(Box::new(envelopes), opts)
}

pub fn send<P: Printer, B: Backend + ?Sized, S: Sender + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
    raw_email: &str,
) -> Result<()> {
    let is_tty = atty::is(Stream::Stdin);
    let is_json = printer.is_json();
    let sent_folder = config.folder_alias("sent")?;
    let raw_email = if is_tty || is_json {
        raw_email.replace("\r", "").replace("\n", "\r\n")
    } else {
        io::stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\r\n")
    };
    trace!("raw email: {:?}", raw_email);
    sender.send(raw_email.as_bytes())?;
    backend.add_email(&sent_folder, raw_email.as_bytes(), "seen")?;
    Ok(())
}

pub fn write<P: Printer, B: Backend + ?Sized, S: Sender + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
    headers: Option<Vec<&str>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl = Email::new_tpl_builder(config)?
        .set_some_raw_headers(headers)
        .some_text_plain_part(body)
        .build();
    trace!("initial template: {}", *tpl);
    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl)?;
    Ok(())
}
