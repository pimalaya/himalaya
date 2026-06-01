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
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{arg::MaildirPathFlag, client::MaildirClient};

/// Delete the given mailbox.
///
/// This command allows you to delete a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxDeleteCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
}

impl MaildirMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let path = client
            .root
            .join(&self.maildir_path.inner)
            .to_string_lossy()
            .into_owned();

        client.delete_maildir(path)?;
        printer.out(Message::new("Maildir successfully deleted"))
    }
}
