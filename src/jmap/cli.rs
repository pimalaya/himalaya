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
    client::JmapClient, email::cli::JmapEmailCommand, identity::cli::JmapIdentityCommand,
    mailbox::cli::JmapMailboxCommand, query::JmapQueryCommand,
    submission::cli::JmapSubmissionCommand, thread::cli::JmapThreadCommand,
    vacation::cli::JmapVacationCommand,
};

/// JMAP CLI.
///
/// This command gives you access to the JMAP CLI API, and allows you
/// to manage JMAP mailboxes, threads, emails, identities, submissions
/// and vacation responses.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum JmapCommand {
    #[command(subcommand)]
    #[command(visible_aliases = ["mbox"])]
    Mailboxes(JmapMailboxCommand),

    #[command(subcommand)]
    #[command(visible_aliases = ["msg"])]
    Emails(JmapEmailCommand),

    #[command(subcommand)]
    Threads(JmapThreadCommand),
    #[command(subcommand)]
    #[command(aliases = ["identities"])]
    Identity(JmapIdentityCommand),
    #[command(subcommand)]
    #[command(aliases = ["submissions", "submit"])]
    Submission(JmapSubmissionCommand),
    #[command(subcommand)]
    #[command(alias = "vacation-response")]
    Vacation(JmapVacationCommand),
    Query(JmapQueryCommand),
}

impl JmapCommand {
    pub fn execute(self, printer: &mut impl Printer, client: JmapClient) -> Result<()> {
        match self {
            Self::Mailboxes(cmd) => cmd.execute(printer, client),
            Self::Emails(cmd) => cmd.execute(printer, client),

            Self::Threads(cmd) => cmd.execute(printer, client),
            Self::Identity(cmd) => cmd.execute(printer, client),
            Self::Submission(cmd) => cmd.execute(printer, client),
            Self::Vacation(cmd) => cmd.execute(printer, client),
            Self::Query(cmd) => cmd.execute(printer, client),
        }
    }
}
