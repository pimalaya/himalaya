use std::{fmt, fs, path::PathBuf};

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use convert_case::ccase;
use io_maildir::maildir::Maildir;
use mail_parser::MimeHeaders;
use mime_guess::{get_mime_extensions_str, mime::OCTET_STREAM};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdArg},
};

/// Export a message.
///
/// This command exports a message in various formats:
/// - raw: Output raw RFC822 message to stdout
/// - eml: Save as .eml file
/// - parts: Export all MIME parts to separate files
#[derive(Debug, Parser)]
pub struct MaildirMessageExportCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    #[command(flatten)]
    pub id: MessageIdArg,

    /// Type of the export.
    #[arg(long, short, value_enum, default_value_t)]
    pub r#type: ExportType,

    /// Output directory (for eml and parts types).
    #[arg(long, short, value_name = "DIR")]
    pub directory: Option<PathBuf>,

    /// Open exported content in default application, when applicable.
    #[arg(long, short)]
    pub open: bool,
}

impl MaildirMessageExportCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(&self.maildir.inner))?,
        };

        let client = account.new_maildir_client();
        let msg = client.get(maildir, &self.id.inner)?;

        match self.r#type {
            ExportType::Raw => {
                let contents = String::from_utf8(msg.into())?;
                printer.out(ExportRaw { contents })?;
            }
            ExportType::Parts => {
                let path = msg.path().to_owned();

                let Some(parsed) = msg.parsed() else {
                    bail!("Invalid MIME message at {}", path.display());
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
