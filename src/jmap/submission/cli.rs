// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

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
    pub fn execute(self, printer: &mut impl Printer, client: JmapClient) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Query(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Cancel(cmd) => cmd.execute(printer, client),
        }
    }
}
