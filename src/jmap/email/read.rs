use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::email::EmailAddress;
use log::warn;
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

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
        let mut client = account.new_jmap_client()?;
        let output = client.email_get(self.ids.clone(), None, !self.html, self.html, 0)?;

        for id in output.not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let mut content = String::new();

        for email in &output.emails {
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
