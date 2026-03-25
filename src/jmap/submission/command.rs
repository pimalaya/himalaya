use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
    submission::{
        cancel::CancelSubmissionCommand, create::CreateSubmissionCommand,
        get::GetSubmissionCommand, query::QuerySubmissionCommand,
    },
};

/// Manage JMAP email submissions.
#[derive(Debug, Subcommand)]
pub enum SubmissionCommand {
    /// Fetch submissions by ID (EmailSubmission/get).
    Get(GetSubmissionCommand),
    /// Query and list submissions (EmailSubmission/query + EmailSubmission/get).
    #[command(aliases = ["lst", "list"])]
    Query(QuerySubmissionCommand),
    /// Submit a draft email for sending (EmailSubmission/set).
    #[command(aliases = ["send", "submit"])]
    Create(CreateSubmissionCommand),
    /// Cancel a pending submission (EmailSubmission/set).
    Cancel(CancelSubmissionCommand),
}

impl SubmissionCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Query(cmd) => cmd.execute(printer, account),
            Self::Create(cmd) => cmd.execute(printer, account),
            Self::Cancel(cmd) => cmd.execute(printer, account),
        }
    }
}
