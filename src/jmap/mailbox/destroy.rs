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

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::mailbox_set::JmapMailboxSetArgs;
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

/// Delete a JMAP mailbox.
#[derive(Debug, Parser)]
pub struct JmapMailboxDestroyCommand {
    /// The ID of the mailbox to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Destroy all emails in the mailbox when deleting.
    #[arg(long, default_value_t)]
    pub purge: bool,
}

impl JmapMailboxDestroyCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let args = JmapMailboxSetArgs {
            destroy: Some(self.ids.clone()),
            on_destroy_remove_emails: if self.purge { Some(true) } else { None },
            ..Default::default()
        };

        let output = client.mailbox_set(args)?;

        if !output.not_destroyed.is_empty() {
            let mut msg = String::from("Destroy JMAP mailbox(es) error");

            for (id, err) in output.not_destroyed {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
