//! # Message compose
//!
//! The `message compose` command, assembling a new message from flags.

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

/// Compose a new message from flags.
///
/// The RFC 5322 bytes go to stdout, unless `--save` appends a copy to a
/// mailbox, `--send` pushes the message out, or both. With `--json` and
/// neither, the decoded fields come out instead, without the signature
/// that sending appends.
///
/// Multipart MIME, MML directives, signing and editor-driven workflows
/// belong to a standalone composer such as mml, piped into `message send`
/// or `message add`.
#[derive(Debug, Parser)]
pub struct MessageComposeCommand {
    /// Sender address, defaulting to the account's `email` under its
    /// `display-name`.
    #[arg(long, value_name = "ADDR")]
    pub from: Option<String>,
    /// Recipient addresses, the flag repeating or taking a
    /// comma-separated list.
    #[arg(long, short = 't', value_name = "ADDR")]
    pub to: Vec<String>,
    /// Carbon-copy recipients.
    #[arg(long, value_name = "ADDR")]
    pub cc: Vec<String>,
    /// Blind carbon-copy recipients.
    #[arg(long, value_name = "ADDR")]
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
    /// Append a copy of the composed message to this mailbox name, alias
    /// or role, overriding `message.send.save-copy` when sending.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,
    /// Skip the copy `message.send.save-copy` configures.
    #[arg(long, conflicts_with = "save")]
    pub no_save: bool,
    /// Send the composed message, which combines with `--save` to keep a
    /// copy too.
    #[arg(long)]
    pub send: bool,
}

impl MessageComposeCommand {
    /// Builds the message and hands it to the handler.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let (from, from_name) = account.resolve_from(self.from.as_deref());
        let template = printer.is_json() && !self.send && self.save.is_none();
        let signature_file = self.signature_file.as_deref().filter(|_| !template);
        let signature = match template {
            true => None,
            false => account.resolve_signature(self.signature.as_deref(), signature_file),
        };

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
                signature_file,
                signature_delim: account.signature_delim(),
            },
            None,
        )?;

        handler::route(
            printer,
            account,
            client,
            raw,
            account.resolve_save(self.save.as_deref(), self.no_save, self.send),
            self.send,
        )
    }
}
