//! # Gmail drafts
//!
//! The `gmail drafts` command family, covering `users.drafts`.

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod send;
pub mod update;

use core::fmt;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        drafts::{
            create::GmailDraftCreateCommand, delete::GmailDraftDeleteCommand,
            get::GmailDraftGetCommand, list::GmailDraftsListCommand, send::GmailDraftSendCommand,
            update::GmailDraftUpdateCommand,
        },
    },
};

/// Manage Gmail drafts (users.drafts).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailDraftsCommand {
    List(GmailDraftsListCommand),
    Get(GmailDraftGetCommand),
    Create(GmailDraftCreateCommand),
    Update(GmailDraftUpdateCommand),
    Send(GmailDraftSendCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailDraftDeleteCommand),
}

impl GmailDraftsCommand {
    /// Runs the subcommand against the account's Gmail client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// Identity of a created or replaced Gmail draft.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailDraftWriteOutput {
    /// The immutable draft id.
    pub(crate) id: String,
    /// The id of the message currently stored in the draft, when present.
    pub(crate) message_id: Option<String>,
    /// The thread id returned by Gmail, when present.
    pub(crate) thread_id: Option<String>,
    /// The verb used only in the text confirmation.
    #[serde(skip)]
    pub(crate) action: &'static str,
}

impl fmt::Display for GmailDraftWriteOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gmail draft `{}` successfully {}", self.id, self.action)
    }
}
