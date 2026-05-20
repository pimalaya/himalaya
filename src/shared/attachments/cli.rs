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

use crate::shared::{
    attachments::{download::AttachmentDownloadCommand, list::AttachmentListCommand},
    client::EmailClient,
};

/// Shared API to manage attachments for the active account.
///
/// An attachment is a binary part of a message.
#[derive(Debug, Subcommand)]
pub enum AttachmentCommand {
    #[command(visible_alias = "ls")]
    List(AttachmentListCommand),
    #[command(visible_alias = "dl")]
    Download(AttachmentDownloadCommand),
}

impl AttachmentCommand {
    pub fn execute(self, printer: &mut impl Printer, client: EmailClient) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, client),
            Self::Download(cmd) => cmd.execute(printer, client),
        }
    }
}
