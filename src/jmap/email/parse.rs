use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::coroutines::email_parse::{JmapEmailParse, JmapEmailParseResult};
use io_stream::runtimes::std::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Parse RFC 5322 message blobs without storing them (Email/parse).
///
/// Useful for reading attached .eml files or message blobs that are
/// not yet stored as Email objects.
#[derive(Debug, Parser)]
pub struct ParseEmailCommand {
    /// Blob ID(s) to parse as RFC 5322 messages.
    #[arg(value_name = "ID", required = true)]
    pub blob_ids: Vec<String>,
}

impl ParseEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            JmapEmailParse::new(&jmap.session, &jmap.http_auth, self.blob_ids.clone(), None)?;
        let mut arg = None;

        let (parsed, not_parsable, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailParseResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapEmailParseResult::Ok {
                    parsed,
                    not_parsable,
                    not_found,
                    ..
                } => {
                    break (parsed, not_parsable, not_found);
                }
                JmapEmailParseResult::Err { err, .. } => bail!(err),
            }
        };

        for id in not_found {
            warn!("blob `{id}` not found, ignoring it");
        }

        for id in not_parsable {
            warn!("blob `{id}` not valid MIME message, ignoring it");
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
