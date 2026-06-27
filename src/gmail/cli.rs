use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::gmail::{
    attachments::GmailAttachmentsCommand, client::GmailClient, drafts::GmailDraftsCommand,
    history::GmailHistoryCommand, labels::GmailLabelsCommand, messages::GmailMessagesCommand,
    profile::GmailProfileCommand, settings::GmailSettingsCommand, threads::GmailThreadsCommand,
};

/// Gmail-specific API.
///
/// This command gives you access to the raw Gmail REST API, organized by Gmail
/// resource: profile, labels, messages (and attachments), drafts, threads,
/// history and settings.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailCommand {
    #[command(subcommand)]
    Profile(GmailProfileCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["label"])]
    Labels(GmailLabelsCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["message", "msg"])]
    Messages(GmailMessagesCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["attachment"])]
    Attachments(GmailAttachmentsCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["draft"])]
    Drafts(GmailDraftsCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["thread"])]
    Threads(GmailThreadsCommand),
    #[command(subcommand)]
    History(GmailHistoryCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["setting"])]
    Settings(GmailSettingsCommand),
}

impl GmailCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Profile(cmd) => cmd.execute(printer, account, client),
            Self::Labels(cmd) => cmd.execute(printer, account, client),
            Self::Messages(cmd) => cmd.execute(printer, account, client),
            Self::Attachments(cmd) => cmd.execute(printer, account, client),
            Self::Drafts(cmd) => cmd.execute(printer, account, client),
            Self::Threads(cmd) => cmd.execute(printer, account, client),
            Self::History(cmd) => cmd.execute(printer, account, client),
            Self::Settings(cmd) => cmd.execute(printer, account, client),
        }
    }
}
