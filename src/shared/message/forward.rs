use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
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
    #[arg(value_name = "ID")]
    pub id: String,

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

    #[arg(
        long = "posting-style",
        short = 'P',
        value_name = "STYLE",
        default_value = "top"
    )]
    pub posting_style: PostingStyle,

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
        let mailbox = account.resolve_mailbox(&self.mailbox).to_owned();
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
