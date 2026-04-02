use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Query(cmd) => cmd.execute(printer, account),
            Self::Create(cmd) => cmd.execute(printer, account),
            Self::Cancel(cmd) => cmd.execute(printer, account),
        }
    }
}
