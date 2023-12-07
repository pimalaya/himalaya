use anyhow::{anyhow, Context, Result};
use atty::Stream;
use email::{
    account::config::AccountConfig, envelope::Id, flag::Flag, message::Message,
    template::FilterParts,
};
use log::trace;
use mail_builder::MessageBuilder;
use std::{
    fs,
    io::{self, BufRead},
};
use url::Url;
use uuid::Uuid;

use crate::{backend::Backend, printer::Printer, ui::editor};

pub async fn attachments<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    ids: Vec<&str>,
) -> Result<()> {
    let emails = backend.get_messages(&folder, &ids).await?;
    let mut index = 0;

    let mut emails_count = 0;
    let mut attachments_count = 0;

    let mut ids = ids.iter();
    for email in emails.to_vec() {
        let id = ids.next().unwrap();
        let attachments = email.attachments()?;

        index = index + 1;

        if attachments.is_empty() {
            printer.print_log(format!("No attachment found for email #{}", id))?;
            continue;
        } else {
            emails_count = emails_count + 1;
        }

        printer.print_log(format!(
            "{} attachment(s) found for email #{}…",
            attachments.len(),
            id
        ))?;

        for attachment in attachments {
            let filename = attachment
                .filename
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            let filepath = config.download_fpath(&filename)?;
            printer.print_log(format!("Downloading {:?}…", filepath))?;
            fs::write(&filepath, &attachment.body).context("cannot download attachment")?;
            attachments_count = attachments_count + 1;
        }
    }

    match attachments_count {
        0 => printer.print("No attachment found!"),
        1 => printer.print("Downloaded 1 attachment!"),
        n => printer.print(format!(
            "Downloaded {} attachment(s) from {} email(s)!",
            n, emails_count,
        )),
    }
}

/// Parses and edits a message from a [mailto] URL string.
///
/// [mailto]: https://en.wikipedia.org/wiki/Mailto
pub async fn mailto<P: Printer>(
    config: &AccountConfig,
    backend: &Backend,
    printer: &mut P,
    url: &Url,
) -> Result<()> {
    let mut builder = MessageBuilder::new().to(url.path());

    for (key, val) in url.query_pairs() {
        match key.to_lowercase().as_bytes() {
            b"cc" => builder = builder.cc(val.to_string()),
            b"bcc" => builder = builder.bcc(val.to_string()),
            b"subject" => builder = builder.subject(val),
            b"body" => builder = builder.text_body(val),
            _ => (),
        }
    }

    let tpl = config
        .generate_tpl_interpreter()
        .with_show_only_headers(config.email_writing_headers())
        .build()
        .from_msg_builder(builder)
        .await?;

    editor::edit_tpl_with_editor(config, printer, backend, tpl).await
}
