use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::email_parse::{JmapEmailParse, JmapEmailParseResult};
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::jmap::account::JmapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Parse RFC 5322 message blobs without storing them (Email/parse).
///
/// Useful for reading attached .eml files or message blobs that are
/// not yet stored as Email objects.
#[derive(Debug, Parser)]
pub struct JmapEmailParseCommand {
    /// Blob ID(s) to parse as RFC 5322 messages.
    #[arg(value_name = "ID", required = true)]
    pub blob_ids: Vec<String>,
}

impl JmapEmailParseCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            JmapEmailParse::new(&jmap.session, &jmap.http_auth, self.blob_ids.clone(), None)?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let (parsed, not_parsable, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailParseResult::Ok {
                    parsed,
                    not_parsable,
                    not_found,
                    ..
                } => break (parsed, not_parsable, not_found),
                JmapEmailParseResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailParseResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailParseResult::Err(err) => bail!("{err}"),
            }
        };

        for id in not_found {
            warn!("blob `{id}` not found, ignoring it");
        }

        for id in not_parsable {
            warn!("blob `{id}` not valid MIME message, ignoring it");
        }

        let mut bodies = Vec::new();

        for (_blob_id, email) in parsed {
            if let Some(body_values) = &email.body_values {
                if let Some(text_parts) = &email.text_body {
                    for part in text_parts {
                        if let Some(part_id) = &part.part_id {
                            if let Some(body_value) = body_values.get(part_id) {
                                bodies.push(body_value.value.clone());
                            }
                        }
                    }
                }
            }
        }

        printer.out(ParsedBodies { bodies })
    }
}

#[derive(Serialize)]
struct ParsedBodies {
    bodies: Vec<String>,
}

impl std::fmt::Display for ParsedBodies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for body in &self.bodies {
            write!(f, "{body}")?;
        }
        Ok(())
    }
}
