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

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{mailbox::MailboxCreate, mailbox_set::JmapMailboxSetArgs};
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

/// Create a JMAP mailbox.
#[derive(Debug, Parser)]
pub struct JmapMailboxCreateCommand {
    /// The name of the new mailbox.
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Attach the new mailbox to the parent mailbox matching the
    /// given identifier.
    #[arg(long, value_name = "ID")]
    pub parent_id: Option<String>,

    /// Should subscribe to the new mailbox.
    #[arg(long, value_name = "NAME")]
    pub subscribe: bool,
}

impl JmapMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let new_mailbox = MailboxCreate {
            name: Some(self.name.clone()),
            parent_id: self.parent_id,
            is_subscribed: if self.subscribe { Some(true) } else { None },
            ..Default::default()
        };

        let mut create = BTreeMap::new();
        create.insert(self.name.clone(), new_mailbox);

        let mut args = JmapMailboxSetArgs::default();
        args.create = Some(create);

        let output = client.mailbox_set(args)?;

        if let Some(err) = output.not_created.get(&self.name) {
            let mut msg = format!("Create JMAP mailbox `{}` error", self.name);
            msg.push_str(&format_set_error(err));
            bail!(msg)
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
