use anyhow::{anyhow, Result};
use atty::Stream;
use pimalaya_email::{AccountConfig, Backend, CompilerBuilder, Email, Flags, Sender, Tpl};
use std::io::{stdin, BufRead};

use crate::printer::Printer;

pub fn forward<P: Printer, B: Backend + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    id: &str,
    headers: Option<Vec<&str>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl = backend
        .get_emails(folder, vec![id])?
        .first()
        .ok_or_else(|| anyhow!("cannot find email {}", id))?
        .to_forward_tpl_builder(config)?
        .set_some_raw_headers(headers)
        .some_text_plain_part(body)
        .build();

    printer.print(<Tpl as Into<String>>::into(tpl))
}

pub fn reply<P: Printer, B: Backend + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    id: &str,
    all: bool,
    headers: Option<Vec<&str>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl = backend
        .get_emails(folder, vec![id])?
        .first()
        .ok_or_else(|| anyhow!("cannot find email {}", id))?
        .to_reply_tpl_builder(config, all)?
        .set_some_raw_headers(headers)
        .some_text_plain_part(body)
        .build();

    printer.print(<Tpl as Into<String>>::into(tpl))
}

pub fn save<P: Printer, B: Backend + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    tpl: String,
) -> Result<()> {
    let email = Tpl::from(if atty::is(Stream::Stdin) || printer.is_json() {
        tpl.replace("\r", "")
    } else {
        stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\n")
    })
    .compile(
        CompilerBuilder::default()
            .some_pgp_sign_cmd(config.email_writing_sign_cmd.as_ref())
            .some_pgp_encrypt_cmd(config.email_writing_encrypt_cmd.as_ref()),
    )?;

    backend.add_email(folder, &email, &Flags::default())?;
    printer.print("Template successfully saved!")
}

pub fn send<P: Printer, B: Backend + ?Sized, S: Sender + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
    folder: &str,
    tpl: String,
) -> Result<()> {
    let email = Tpl::from(if atty::is(Stream::Stdin) || printer.is_json() {
        tpl.replace("\r", "")
    } else {
        stdin()
            .lock()
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\n")
    })
    .compile(
        CompilerBuilder::default()
            .some_pgp_sign_cmd(config.email_writing_sign_cmd.as_ref())
            .some_pgp_encrypt_cmd(config.email_writing_encrypt_cmd.as_ref()),
    )?;
    sender.send(&email)?;
    backend.add_email(folder, &email, &Flags::default())?;
    printer.print("Template successfully sent!")?;
    Ok(())
}

pub fn write<'a, P: Printer>(
    config: &'a AccountConfig,
    printer: &'a mut P,
    headers: Option<Vec<&str>>,
    body: Option<&str>,
) -> Result<()> {
    let tpl = Email::new_tpl_builder(config)?
        .set_some_raw_headers(headers)
        .some_text_plain_part(body)
        .build();

    printer.print(<Tpl as Into<String>>::into(tpl))
}
