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
        builder::{self, BuilderArgs, PostingStyle, SourceArgs, SourceMode},
        output,
    },
};

/// Reply to a message using the built-in flag composer.
///
/// Fetches the source message, pre-fills `In-Reply-To` / `References`
/// and the `Re:` subject, optionally derives recipients from
/// `Reply-To`/`From`, and quotes the source text body. The produced
/// MIME is written to stdout, or routed via `--save` / `--send`.
/// For non-default composition, use `reply-with <id> <name>`.
#[derive(Debug, Parser)]
pub struct MessageReplyCommand {
    /// Identifier of the source message (IMAP UID, JMAP id, Maildir
    /// filename id).
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mailbox the source message lives in. Ignored for JMAP, which
    /// addresses messages by id directly.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    #[arg(long, value_name = "ADDR")]
    pub from: Option<String>,

    #[arg(long, short = 't', value_name = "ADDR", value_delimiter = ',')]
    pub to: Vec<String>,

    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub cc: Vec<String>,

    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub bcc: Vec<String>,

    #[arg(long, short = 's', value_name = "TEXT")]
    pub subject: Option<String>,

    #[arg(long, value_name = "TEXT", conflicts_with = "body_file")]
    pub body: Option<String>,

    #[arg(long = "body-file", value_name = "PATH")]
    pub body_file: Option<PathBuf>,

    #[arg(long = "attach", value_name = "PATH")]
    pub attach: Vec<PathBuf>,

    #[arg(long, value_name = "TEXT")]
    pub signature: Option<String>,

    #[arg(
        long = "signature-file",
        value_name = "PATH",
        conflicts_with = "signature"
    )]
    pub signature_file: Option<PathBuf>,

    /// How to lay out the quoted source body relative to the user's
    /// body. Interleaved posting is left to the user — write your
    /// reply inside the quoted block.
    #[arg(
        long = "posting-style",
        short = 'P',
        value_name = "STYLE",
        default_value = "top"
    )]
    pub posting_style: PostingStyle,

    /// Plain-text headline placed before the quoted source body
    /// (e.g. `"On {date}, {from} wrote:"`). No substitution is
    /// performed; pass the literal string you want.
    #[arg(long = "quote-headline", short = 'Q', value_name = "TEXT")]
    pub quote_headline: Option<String>,

    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    #[arg(long)]
    pub send: bool,
}

impl MessageReplyCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let source = client.get_message(&self.mailbox, &self.id)?;

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
            Some(SourceArgs {
                raw: &source,
                mode: SourceMode::Reply,
                posting_style: self.posting_style,
                quote_headline: self.quote_headline.as_deref().unwrap_or(""),
            }),
        )?;

        output::route(printer, &mut client, raw, self.save.as_deref(), self.send)
    }
}
