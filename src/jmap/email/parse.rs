use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_parse::{ParseJmapEmails, ParseJmapEmailsResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Parse RFC 5322 message blobs without storing them (Email/parse).
///
/// Useful for reading attached .eml files or message blobs that are
/// not yet stored as Email objects.
#[derive(Debug, Parser)]
pub struct ParseEmailCommand {
    /// Blob ID(s) to parse as RFC 5322 messages.
    #[arg(value_name = "BLOB-ID", required = true, num_args = 1..)]
    pub blob_ids: Vec<String>,
}

impl ParseEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = ParseJmapEmails::new(jmap.context, self.blob_ids.clone(), None)?;
        let mut arg = None;

        let (parsed, not_parsable, not_found) = loop {
            match coroutine.resume(arg.take()) {
                ParseJmapEmailsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                ParseJmapEmailsResult::Ok {
                    context,
                    parsed,
                    not_parsable,
                    not_found,
                    ..
                } => {
                    jmap.context = context;
                    break (parsed, not_parsable, not_found);
                }
                ParseJmapEmailsResult::Err { err, .. } => bail!(err),
            }
        };

        for id in &not_found {
            printer.log(format!("Blob `{id}` not found."))?;
        }

        for id in &not_parsable {
            printer.log(format!("Blob `{id}` is not a valid RFC 5322 message."))?;
        }

        for (_blob_id, email) in parsed {
            if let Some(body_values) = &email.body_values {
                if let Some(text_parts) = &email.text_body {
                    for part in text_parts {
                        if let Some(part_id) = &part.part_id {
                            if let Some(body_value) = body_values.get(part_id) {
                                printer.out(&body_value.value)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
