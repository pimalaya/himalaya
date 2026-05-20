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
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    messages::{output, runner},
};

/// Forward a message by delegating to a user-defined composer.
///
/// Same shape as `reply-with`: fetches the source, pipes it on
/// stdin to the named (or default) composer, captures stdout as the
/// MIME draft.
#[derive(Debug, Parser)]
pub struct MessageForwardWithCommand {
    #[arg(value_name = "ID")]
    pub id: String,

    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    #[arg(value_name = "NAME", conflicts_with = "command")]
    pub name: Option<String>,

    #[arg(long, value_name = "SHELL")]
    pub command: Option<String>,

    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    #[arg(long)]
    pub send: bool,
}

impl MessageForwardWithCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let source = client.get_message(&self.mailbox, &self.id)?;

        let command = match self.command.as_deref() {
            Some(cmd) => cmd.to_owned(),
            None => {
                runner::resolve_composer(&client.account.composer, self.name.as_deref())?.to_owned()
            }
        };

        let raw = runner::run(&command, &source)?;
        if raw.is_empty() {
            bail!("composer `{command}` produced no output");
        }

        output::route(printer, &mut client, raw, self.save.as_deref(), self.send)
    }
}
