use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        message::{
            builder::{self, BuilderArgs},
            handler,
        },
    },
};

/// Compose a new message from CLI arguments (built-in flag composer).
///
/// Use this for the simple case: pass `--from`, `--to`, `--body`,
/// etc., and the message is assembled with `mail_builder`. The
/// produced RFC 5322 bytes are written to stdout by default; pass
/// `--save <mailbox>` to append a copy, `--send` to push through the
/// account's SMTP/JMAP send path, or both. For richer composition
/// (multipart MIME, MML directives, signing/encryption, editor-driven
/// workflows), chain a standalone composer like
/// [`mml`](https://github.com/pimalaya/mml) into `messages send` /
/// `messages add` via a tempfile or bash/zsh process substitution.
#[derive(Debug, Parser)]
pub struct MessageComposeCommand {
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

    /// Read the body from a file. Mutually exclusive with `--body`
    /// and stdin.
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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
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
            None,
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
