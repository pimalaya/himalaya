use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    mailbox::arg::MailboxArg,
    message::{
        builder::{self, BuilderArgs, PostingStyle, SourceArgs, SourceMode},
        handler,
    },
};

/// Forward a message using the built-in flag composer.
///
/// Fetches the source, pre-fills `Fwd:` on the subject and the
/// `References` header, and quotes the source body. The produced
/// MIME is written to stdout, or routed via `--save` / `--send`.
/// For richer composition, pipe `messages read <id>` into a
/// standalone composer (`mml forward`, etc.) and feed its output
/// back into `messages send` / `messages add`.
#[derive(Debug, Parser)]
pub struct MessageForwardCommand {
    /// Identifier of the source message (IMAP UID, JMAP id, Maildir
    /// filename id).
    #[arg(value_name = "ID")]
    pub id: String,

    #[command(flatten)]
    pub mailbox: MailboxArg,

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

    /// Inline body. Conflicts with `--body-file`; stdin is used as a
    /// fallback when neither is given.
    #[arg(long, value_name = "TEXT", conflicts_with = "body_file")]
    pub body: Option<String>,

    #[arg(long = "body-file", value_name = "PATH")]
    pub body_file: Option<PathBuf>,

    /// Attachment file(s).
    #[arg(long = "attach", value_name = "PATH")]
    pub attach: Vec<PathBuf>,

    /// Signature appended after the body, separated by the standard
    /// `-- ` delimiter (RFC 3676 §4.3).
    #[arg(long, value_name = "TEXT")]
    pub signature: Option<String>,

    #[arg(
        long = "signature-file",
        value_name = "PATH",
        conflicts_with = "signature"
    )]
    pub signature_file: Option<PathBuf>,

    /// How to lay out the quoted source body relative to the user's
    /// body. Interleaved posting is left to the user; write your
    /// message inside the quoted block.
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

impl MessageForwardCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let source = client.get_message(&mailbox, &self.id)?;

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
                mode: SourceMode::Forward,
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
