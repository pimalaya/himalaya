use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{
    email::EmailAddress,
    email_get::{JmapEmailGet, JmapEmailGetResult},
};
use log::warn;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Read the content of a JMAP email (Email/get with body).
///
/// Shows headers and plain text body by default.
#[derive(Debug, Parser)]
pub struct JmapEmailReadCommand {
    /// The email ID(s) to read.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Show HTML body instead of plain text.
    #[arg(long)]
    pub html: bool,
}

impl JmapEmailReadCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = JmapEmailGet::new(
            &jmap.session,
            &jmap.http_auth,
            self.ids.clone(),
            None,
            !self.html,
            self.html,
            0,
        )?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let (emails, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailGetResult::Ok {
                    emails, not_found, ..
                } => break (emails, not_found),
                JmapEmailGetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailGetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailGetResult::Err(err) => bail!("{err}"),
            }
        };

        for id in not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let mut content = String::new();

        for email in &emails {
            if self.html {
                if let Some(body_values) = &email.body_values {
                    if let Some(html_parts) = &email.html_body {
                        for part in html_parts {
                            if let Some(part_id) = &part.part_id {
                                if let Some(body_value) = body_values.get(part_id) {
                                    content.push_str(&body_value.value);
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(addrs) = &email.from {
                    content.push_str(&format!("From: {}\n", format_addresses(addrs)));
                }
                if let Some(addrs) = &email.to {
                    content.push_str(&format!("To: {}\n", format_addresses(addrs)));
                }
                if let Some(addrs) = &email.cc {
                    content.push_str(&format!("Cc: {}\n", format_addresses(addrs)));
                }
                if let Some(subject) = &email.subject {
                    content.push_str(&format!("Subject: {subject}\n"));
                }
                if let Some(date) = &email.sent_at {
                    content.push_str(&format!("Date: {date}\n"));
                }

                if let Some(body_values) = &email.body_values {
                    if let Some(text_parts) = &email.text_body {
                        for part in text_parts {
                            if let Some(part_id) = &part.part_id {
                                if let Some(body_value) = body_values.get(part_id) {
                                    content.push('\n');
                                    content.push_str(&body_value.value);
                                }
                            }
                        }
                    }
                }
            }
        }

        printer.out(Message::new(content))
    }
}

fn format_addresses(addrs: &[EmailAddress]) -> String {
    addrs
        .iter()
        .map(|a| match &a.name {
            Some(name) if !name.is_empty() => format!("{name} <{}>", a.email),
            _ => a.email.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
