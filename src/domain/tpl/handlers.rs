use anyhow::{anyhow, Result};
use atty::Stream;
use email::{
    account::AccountConfig,
    email::{Flag, Message},
};
use mml::MmlCompilerBuilder;
use std::io::{stdin, BufRead};

use crate::{backend::Backend, printer::Printer};

pub async fn forward<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    id: &str,
    headers: Option<Vec<(&str, &str)>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl: String = backend
        .get_messages(folder, &[id])
        .await?
        .first()
        .ok_or_else(|| anyhow!("cannot find email {}", id))?
        .to_forward_tpl_builder(config)
        .with_some_headers(headers)
        .with_some_body(body)
        .build()
        .await?
        .into();

    printer.print(tpl)
}

pub async fn reply<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    id: &str,
    all: bool,
    headers: Option<Vec<(&str, &str)>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl: String = backend
        .get_messages(folder, &[id])
        .await?
        .first()
        .ok_or_else(|| anyhow!("cannot find email {}", id))?
        .to_reply_tpl_builder(config)
        .with_some_headers(headers)
        .with_some_body(body)
        .with_reply_all(all)
        .build()
        .await?
        .into();

    printer.print(tpl)
}

pub async fn save<P: Printer>(
    #[allow(unused_variables)] config: &AccountConfig,
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    tpl: String,
) -> Result<()> {
    let tpl = if atty::is(Stream::Stdin) || printer.is_json() {
        tpl.replace("\r", "")
    } else {
        stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\n")
    };

    let compiler = MmlCompilerBuilder::new();

    #[cfg(feature = "pgp")]
    let compiler = compiler.with_pgp(config.pgp.clone());

    let email = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

    backend.add_raw_message(folder, &email).await?;

    printer.print("Template successfully saved!")
}

pub async fn send<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &Backend,
    tpl: String,
) -> Result<()> {
    let folder = config.sent_folder_alias()?;

    let tpl = if atty::is(Stream::Stdin) || printer.is_json() {
        tpl.replace("\r", "")
    } else {
        stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\n")
    };

    let compiler = MmlCompilerBuilder::new();

    #[cfg(feature = "pgp")]
    let compiler = compiler.with_pgp(config.pgp.clone());

    let email = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

    backend.send_raw_message(&email).await?;

    if config.email_sending_save_copy.unwrap_or_default() {
        backend
            .add_raw_message_with_flag(&folder, &email, Flag::Seen)
            .await?;
    }

    printer.print("Template successfully sent!")?;
    Ok(())
}

pub async fn write<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    headers: Option<Vec<(&str, &str)>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl: String = Message::new_tpl_builder(config)
        .with_some_headers(headers)
        .with_some_body(body)
        .build()
        .await?
        .into();

    printer.print(tpl)
}
