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

use std::{fmt, fs, path::PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use convert_case::ccase;
use mail_parser::{MessageParser, MimeHeaders};
use mime_guess::{get_mime_extensions_str, mime::OCTET_STREAM};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::m2dir::{
    arg::{M2dirNameFlag, MessageIdArg},
    client::M2dirClient,
};

/// Export an m2dir message.
///
/// Two output modes are supported:
/// - raw: write the raw RFC 5322 bytes to stdout
/// - parts: explode every MIME part into a separate file under
///   `--directory <DIR>` (defaults to the message id)
#[derive(Debug, Parser)]
pub struct M2dirMessageExportCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    #[command(flatten)]
    pub id: MessageIdArg,

    /// Type of the export.
    #[arg(long, short, value_enum, default_value_t)]
    pub r#type: ExportType,

    /// Output directory (for parts type).
    #[arg(long, short, value_name = "DIR")]
    pub directory: Option<PathBuf>,

    /// Open exported HTML parts in the default application.
    #[arg(long, short)]
    pub open: bool,
}

impl M2dirMessageExportCommand {
    pub fn execute(self, printer: &mut impl Printer, client: M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let (entry, bytes) = client.get(m2dir, &self.id.inner)?;

        match self.r#type {
            ExportType::Raw => {
                let contents = String::from_utf8(bytes)?;
                printer.out(ExportRaw { contents })?;
            }
            ExportType::Parts => {
                let Some(parsed) = MessageParser::new().parse(&bytes) else {
                    let path = entry.path();
                    bail!("Invalid MIME message at {path}");
                };

                let dir = match self.directory {
                    Some(dir) => dir,
                    None => PathBuf::from(self.id.inner),
                };

                fs::create_dir_all(&dir)?;

                let mut parts = Vec::new();

                for (i, part) in parsed.parts.iter().enumerate() {
                    let cr = part.content_type().map(|ct| match &ct.c_subtype {
                        Some(sub) => format!("{}/{}", ct.c_type, sub),
                        None => ct.c_type.to_string(),
                    });

                    if let Some(ref ct) = cr {
                        if ct.starts_with("multipart/") {
                            continue;
                        }
                    }

                    let filename = match part.attachment_name() {
                        Some(name) => ccase!(kebab, name),
                        None => {
                            let ext = match cr.as_deref().unwrap_or(OCTET_STREAM.as_str()) {
                                "text/plain" => Some(&"txt"),
                                "text/html" => Some(&"html"),
                                ct => get_mime_extensions_str(ct).and_then(|ext| ext.first()),
                            };

                            match ext {
                                Some(ext) => format!("part_{i}.{ext}"),
                                None => format!("part_{i}"),
                            }
                        }
                    };

                    let path = dir.join(&filename);
                    let contents = part.contents();
                    fs::write(&path, contents)?;
                    parts.push(path);
                }

                if self.open {
                    for path in &parts {
                        if let Some(ext) = path.extension() {
                            if ext == "html" {
                                open::that(path)?;
                            }
                        }
                    }
                }

                printer.out(ExportParts { parts })?;
            }
        };

        Ok(())
    }
}

/// Export type for message export.
#[derive(Clone, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ExportType {
    #[default]
    /// Output raw RFC822 message to stdout.
    Raw,
    /// Export all MIME parts to separate files.
    Parts,
}

#[derive(Serialize)]
struct ExportRaw {
    contents: String,
}

impl fmt::Display for ExportRaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.contents)
    }
}

#[derive(Serialize)]
struct ExportParts {
    parts: Vec<PathBuf>,
}

impl fmt::Display for ExportParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for path in &self.parts {
            writeln!(f, " - {}", path.display())?;
        }

        writeln!(f)?;
        write!(f, "Exported {} part(s)", self.parts.len())
    }
}
