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

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    messages::{
        builder::{self, BuilderArgs},
        output,
    },
};

/// Compose a new message from CLI arguments (built-in flag composer).
///
/// Use this for the simple case: pass `--from`, `--to`, `--body`,
/// etc., and the message is assembled with `mail_builder`. The
/// produced RFC 5322 bytes are written to stdout by default; pass
/// `--save <mailbox>` to append a copy, `--send` to push through the
/// account's SMTP/JMAP send path, or both. For richer composition
/// (multipart MIME, MML directives, signing/encryption, TUI editing,
/// …) use `compose-with <name>` instead.
#[derive(Debug, Parser)]
pub struct MessageComposeCommand {
    /// Sender address (`From` header).
    #[arg(long, value_name = "ADDR")]
    pub from: Option<String>,

    /// Recipient address(es) (`To` header). Repeat the flag or use a
    /// comma-separated list.
    #[arg(long, short = 't', value_name = "ADDR", value_delimiter = ',')]
    pub to: Vec<String>,

    /// Carbon-copy recipient(s) (`Cc` header).
    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub cc: Vec<String>,

    /// Blind carbon-copy recipient(s) (`Bcc` header).
    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub bcc: Vec<String>,

    /// Subject line.
    #[arg(long, short = 's', value_name = "TEXT")]
    pub subject: Option<String>,

    /// Inline body. Mutually exclusive with `--body-file` and stdin.
    #[arg(long, value_name = "TEXT", conflicts_with = "body_file")]
    pub body: Option<String>,

    /// Read the body from a file. Mutually exclusive with `--body`
    /// and stdin.
    #[arg(long = "body-file", value_name = "PATH")]
    pub body_file: Option<PathBuf>,

    /// Attachment file(s).
    #[arg(long = "attach", value_name = "PATH")]
    pub attach: Vec<PathBuf>,

    /// Signature appended after the body, separated by the standard
    /// `-- ` delimiter (RFC 3676 §4.3).
    #[arg(long, value_name = "TEXT")]
    pub signature: Option<String>,

    /// Read the signature from a file. Mutually exclusive with
    /// `--signature`.
    #[arg(
        long = "signature-file",
        value_name = "PATH",
        conflicts_with = "signature"
    )]
    pub signature_file: Option<PathBuf>,

    /// Append a copy of the composed message to this mailbox.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    /// Send the composed message through the account's SMTP/JMAP path.
    /// Combines with `--save` to also keep a copy.
    #[arg(long)]
    pub send: bool,
}

impl MessageComposeCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let raw = builder::build(
            BuilderArgs {
                from: self.from.as_deref(),
                to: &self.to,
                cc: &self.cc,
                bcc: &self.bcc,
                subject: self.subject.as_deref(),
                body: self.body.as_deref(),
                body_file: self.body_file.as_deref(),
                attach: &self.attach,
                signature: self.signature.as_deref(),
                signature_file: self.signature_file.as_deref(),
            },
            None,
        )?;

        output::route(printer, &mut client, raw, self.save.as_deref(), self.send)
    }
}
