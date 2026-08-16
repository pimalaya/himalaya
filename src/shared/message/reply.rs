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

/// Reply to a message using the built-in flag composer.
///
/// Fetches the source message, pre-fills `In-Reply-To` / `References`
/// and the `Re:` subject, optionally derives recipients from
/// `Reply-To`/`From`, and quotes the source text body. The produced
/// MIME is written to stdout, or routed via `--save` / `--send`.
/// For richer composition, pipe `messages read <id>` into a
/// standalone composer (`mml reply`, etc.) and feed its output back
/// into `messages send` / `messages add`.
#[derive(Debug, Parser)]
pub struct MessageReplyCommand {
    /// Identifier of the source message.
    #[arg(value_name = "ID")]
    pub id: String,

    #[command(flatten)]
    pub mailbox: MailboxArg,

    /// Sender address (`From` header). Defaults to the account's
    /// `email`, named by its `display-name`.
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

    /// Inline body. Conflicts with `--body-file`; stdin is used as a
    /// fallback when neither is given.
    #[arg(long, value_name = "TEXT", conflicts_with = "body_file")]
    pub body: Option<String>,

    #[arg(long = "body-file", value_name = "PATH")]
    pub body_file: Option<PathBuf>,

    /// Attachment file(s).
    #[arg(long = "attach", value_name = "PATH")]
    pub attach: Vec<PathBuf>,

    /// Signature appended after the body, introduced by the account's
    /// `signature-delim` (RFC 3676 §4.3 `-- ` by default). Defaults to
    /// the account's `signature`.
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
