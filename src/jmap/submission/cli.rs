use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient,
    submission::{
        cancel::JmapSubmissionCancelCommand, create::JmapSubmissionCreateCommand,
        get::JmapSubmissionGetCommand, query::JmapSubmissionQueryCommand,
    },
};

/// Manage JMAP email submissions.
#[derive(Debug, Subcommand)]
pub enum JmapSubmissionCommand {
    /// Fetch submissions by ID (EmailSubmission/get).
    Get(JmapSubmissionGetCommand),
    /// Query and list submissions (EmailSubmission/query + EmailSubmission/get).
    #[command(aliases = ["lst", "list"])]
    Query(JmapSubmissionQueryCommand),
    /// Submit a draft email for sending (EmailSubmission/set).
    #[command(aliases = ["send", "submit"])]
    Create(JmapSubmissionCreateCommand),
    /// Cancel a pending submission (EmailSubmission/set).
    Cancel(JmapSubmissionCancelCommand),
}

impl JmapSubmissionCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Query(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, account, client),
            Self::Cancel(cmd) => cmd.execute(printer, client),
        }
    }
}
