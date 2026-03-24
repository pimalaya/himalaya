use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_get::{GetJmapEmails, GetJmapEmailsResult};
use io_stream::runtimes::std::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Get a JMAP email by ID (Email/get).
///
/// Downloads and displays the full message content including body.
#[derive(Debug, Parser)]
pub struct JmapEmailGetCommand {
    /// The email ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Output raw RFC 5322 message headers.
    #[arg(long, short)]
    pub raw: bool,
}

impl JmapEmailGetCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            GetJmapEmails::new(jmap.context, self.ids.clone(), None, true, true, None)?;
        let mut arg = None;

        let (emails, not_found) = loop {
            match coroutine.resume(arg.take()) {
                GetJmapEmailsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                GetJmapEmailsResult::Ok {
                    emails, not_found, ..
                } => break (emails, not_found),
                GetJmapEmailsResult::Err { err, .. } => bail!(err),
            }
        };

        for id in not_found {
            warn!("email `{id}` not found");
        }

        for email in emails {
            if self.raw {
                if let Some(headers) = &email.headers {
                    for h in headers {
                        printer.log(format!("{}: {}", h.name, h.value))?;
                    }
                }
                printer.log("")?;
            }

            if let Some(body_values) = &email.body_values {
                if let Some(text_parts) = &email.text_body {
                    for part in text_parts {
                        if let Some(part_id) = &part.part_id {
                            if let Some(body_value) = body_values.get(part_id) {
                                printer.out(&body_value.value)?;
                                continue;
                            }
                        }
                    }
                }

                if let Some(html_parts) = &email.html_body {
                    for part in html_parts {
                        if let Some(part_id) = &part.part_id {
                            if let Some(body_value) = body_values.get(part_id) {
                                printer.out(&body_value.value)?;
                                continue;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
