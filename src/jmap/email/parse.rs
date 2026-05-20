// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::jmap::client::JmapClient;

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
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let output = client.email_parse(self.blob_ids.clone(), None)?;

        for id in output.not_found {
            warn!("blob `{id}` not found, ignoring it");
        }

        for id in output.not_parsable {
            warn!("blob `{id}` not valid MIME message, ignoring it");
        }

        let mut bodies = Vec::new();

        for (_blob_id, email) in output.parsed {
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
