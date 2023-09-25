use anyhow::{anyhow, Context, Result};
use atty::Stream;
use email::{
    account::AccountConfig,
    backend::Backend,
    email::{template::FilterParts, Flag, Flags, Message, MessageBuilder},
    sender::Sender,
};
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
    Envelopes, IdMapper,
};

pub async fn attachments<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    ids: Vec<&str>,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    let emails = backend.get_emails(&folder, ids.clone()).await?;
    let mut index = 0;

    let mut emails_count = 0;
    let mut attachments_count = 0;

    for email in emails.to_vec() {
        let id = ids.get(index).unwrap();
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

pub async fn copy<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    from_folder: &str,
    to_folder: &str,
    ids: Vec<&str>,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    backend.copy_emails(&from_folder, &to_folder, ids).await?;
    printer.print("Email(s) successfully copied!")
}

pub async fn delete<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    ids: Vec<&str>,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    backend.delete_emails(&folder, ids).await?;
    printer.print("Email(s) successfully deleted!")
}

pub async fn forward<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    sender: &mut dyn Sender,
    folder: &str,
    id: &str,
    headers: Option<Vec<(&str, &str)>>,
    body: Option<&str>,
) -> Result<()> {
    let ids = id_mapper.get_ids([id])?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();

    let tpl = backend
        .get_emails(&folder, ids)
        .await?
        .first()
        .ok_or_else(|| anyhow!("cannot find email {}", id))?
        .to_forward_tpl_builder(config)
        .with_some_headers(headers)
        .with_some_body(body)
        .build()
        .await?;
    trace!("initial template: {tpl}");
    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl).await?;
    Ok(())
}

pub async fn list<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    debug!("page size: {}", page_size);

    let envelopes = Envelopes::from_backend(
        config,
        id_mapper,
        backend.list_envelopes(&folder, page_size, page).await?,
    )?;
    trace!("envelopes: {:?}", envelopes);

    printer.print_table(
        Box::new(envelopes),
        PrintTableOpts {
            format: &config.email_reading_format,
            max_width,
        },
    )
}

/// Parses and edits a message from a [mailto] URL string.
///
/// [mailto]: https://en.wikipedia.org/wiki/Mailto
pub async fn mailto<P: Printer>(
    config: &AccountConfig,
    backend: &mut dyn Backend,
    sender: &mut dyn Sender,
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

    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl).await
}

pub async fn move_<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    from_folder: &str,
    to_folder: &str,
    ids: Vec<&str>,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    backend.move_emails(&from_folder, &to_folder, ids).await?;
    printer.print("Email(s) successfully moved!")
}

pub async fn read<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    ids: Vec<&str>,
    text_mime: &str,
    raw: bool,
    headers: Vec<&str>,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    let emails = backend.get_emails(&folder, ids).await?;

    let mut glue = "";
    let mut bodies = String::default();

    for email in emails.to_vec() {
        bodies.push_str(glue);

        if raw {
            // emails do not always have valid utf8, uses "lossy" to
            // display what can be displayed
            bodies.push_str(&String::from_utf8_lossy(email.raw()?).into_owned());
        } else {
            let tpl: String = email
                .to_read_tpl(&config, |tpl| match text_mime {
                    "html" => tpl
                        .with_hide_all_headers()
                        .with_filter_parts(FilterParts::Only("text/html".into())),
                    _ => tpl.with_show_additional_headers(&headers),
                })
                .await?
                .into();
            bodies.push_str(&tpl);
        }

        glue = "\n\n";
    }

    printer.print(bodies)
}

pub async fn reply<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    sender: &mut dyn Sender,
    folder: &str,
    id: &str,
    all: bool,
    headers: Option<Vec<(&str, &str)>>,
    body: Option<&str>,
) -> Result<()> {
    let ids = id_mapper.get_ids([id])?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();

    let tpl = backend
        .get_emails(&folder, ids)
        .await?
        .first()
        .ok_or_else(|| anyhow!("cannot find email {}", id))?
        .to_reply_tpl_builder(config)
        .with_some_headers(headers)
        .with_some_body(body)
        .with_reply_all(all)
        .build()
        .await?;
    trace!("initial template: {tpl}");
    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl).await?;
    backend
        .add_flags(&folder, vec![id], &Flags::from_iter([Flag::Answered]))
        .await?;
    Ok(())
}

pub async fn save<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    raw_email: String,
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

    let id = backend
        .add_email(&folder, raw_email.as_bytes(), &Flags::default())
        .await?;
    id_mapper.create_alias(id)?;

    Ok(())
}

pub async fn search<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    let envelopes = Envelopes::from_backend(
        config,
        id_mapper,
        backend
            .search_envelopes(&folder, &query, "", page_size, page)
            .await?,
    )?;
    let opts = PrintTableOpts {
        format: &config.email_reading_format,
        max_width,
    };

    printer.print_table(Box::new(envelopes), opts)
}

pub async fn sort<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    sort: String,
    query: String,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    let envelopes = Envelopes::from_backend(
        config,
        id_mapper,
        backend
            .search_envelopes(&folder, &query, &sort, page_size, page)
            .await?,
    )?;
    let opts = PrintTableOpts {
        format: &config.email_reading_format,
        max_width,
    };

    printer.print_table(Box::new(envelopes), opts)
}

pub async fn send<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut dyn Backend,
    sender: &mut dyn Sender,
    raw_email: String,
) -> Result<()> {
    let folder = config.sent_folder_alias()?;
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
    trace!("raw email: {:?}", raw_email);
    sender.send(raw_email.as_bytes()).await?;
    if config.email_sending_save_copy {
        backend
            .add_email(
                &folder,
                raw_email.as_bytes(),
                &Flags::from_iter([Flag::Seen]),
            )
            .await?;
    }
    Ok(())
}

pub async fn write<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut dyn Backend,
    sender: &mut dyn Sender,
    headers: Option<Vec<(&str, &str)>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl = Message::new_tpl_builder(config)
        .with_some_headers(headers)
        .with_some_body(body)
        .build()
        .await?;
    trace!("initial template: {tpl}");
    editor::edit_tpl_with_editor(config, printer, backend, sender, tpl).await?;
    Ok(())
}
