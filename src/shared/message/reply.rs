//! # Message reply
//!
//! The `message reply` command, composing a reply to a fetched message.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        mailbox::arg::MailboxArg,
        message::{
            builder::{self, BuilderArgs, PostingStyle, SourceArgs, SourceMode},
            handler,
        },
    },
};

/// Reply to a message from flags.
///
/// The source is fetched to pre-fill `In-Reply-To`, `References` and the
/// `Re:` subject, derive the recipients from `Reply-To` or `From`, and
/// quote the text body. The result goes to stdout, `--save` or `--send`.
///
/// Richer composition is `message read <id>` piped into a standalone
/// composer, whose output feeds `message send` or `message add`.
#[derive(Debug, Parser)]
pub struct MessageReplyCommand {
    /// Identifier of the source message.
    #[arg(value_name = "ID")]
    pub id: String,
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Sender address, defaulting to the account's `email` under its
    /// `display-name`.
    #[arg(long, value_name = "ADDR")]
    pub from: Option<String>,
    /// Recipient addresses, the flag repeating or taking a
    /// comma-separated list.
    #[arg(long, short = 't', value_name = "ADDR", value_delimiter = ',')]
    pub to: Vec<String>,
    /// Carbon-copy recipients.
    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub cc: Vec<String>,
    /// Blind carbon-copy recipients.
    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub bcc: Vec<String>,
    /// Subject line.
    #[arg(long, short = 's', value_name = "TEXT")]
    pub subject: Option<String>,
    /// Inline body, the standard input answering when neither this nor
    /// `--body-file` is given.
    #[arg(long, value_name = "TEXT", conflicts_with = "body_file")]
    pub body: Option<String>,
    /// Read the body from a file, exclusive with `--body` and the
    /// standard input.
    #[arg(long = "body-file", value_name = "PATH")]
    pub body_file: Option<PathBuf>,
    /// Files to attach.
    #[arg(long = "attach", value_name = "PATH")]
    pub attach: Vec<PathBuf>,
    /// Signature appended after the body, defaulting to the account's
    /// `signature`.
    ///
    /// The account's `signature-delim` introduces it, the RFC 3676
    /// section 4.3 `-- ` by default.
    #[arg(long, value_name = "TEXT")]
    pub signature: Option<String>,
    /// Read the signature from a file, exclusive with `--signature`.
    #[arg(
        long = "signature-file",
        value_name = "PATH",
        conflicts_with = "signature"
    )]
    pub signature_file: Option<PathBuf>,
    /// Where the quoted source body sits relative to the written one.
    ///
    /// Interleaved posting is left to the writer: put the reply inside
    /// the quoted block.
    #[arg(
        long = "posting-style",
        short = 'P',
        value_name = "STYLE",
        default_value = "top"
    )]
    pub posting_style: PostingStyle,
    /// Plain-text headline placed before the quoted source body.
    ///
    /// Nothing is substituted in it, so pass the literal string wanted.
    #[arg(long = "quote-headline", short = 'Q', value_name = "TEXT")]
    pub quote_headline: Option<String>,
    /// Append a copy of the composed message to this mailbox name or
    /// alias.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,
    /// Send the composed message, which combines with `--save` to keep a
    /// copy too.
    #[arg(long)]
    pub send: bool,
}

impl MessageReplyCommand {
    /// Fetches the source, builds the reply and hands it to the handler.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let source = client.get_message(&mailbox, &self.id, false)?;

        let (from, from_name) = account.resolve_from(self.from.as_deref());
        let signature =
            account.resolve_signature(self.signature.as_deref(), self.signature_file.as_deref());

        let raw = builder::build(
            BuilderArgs {
                from,
                from_name,
                to: &self.to,
                cc: &self.cc,
                bcc: &self.bcc,
                subject: self.subject.as_deref(),
                body: self.body.as_deref(),
                body_file: self.body_file.as_deref(),
                attach: &self.attach,
                signature,
                signature_file: self.signature_file.as_deref(),
                signature_delim: account.signature_delim(),
            },
            Some(SourceArgs {
                raw: &source,
                mode: SourceMode::Reply,
                posting_style: self.posting_style,
                quote_headline: self.quote_headline.as_deref().unwrap_or(""),
            }),
        )?;

        handler::route(
            printer,
            account,
            client,
            raw,
            self.save.as_deref(),
            self.send,
        )
    }
}
