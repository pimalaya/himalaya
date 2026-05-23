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

use crate::{
    config::Composer,
    shared::{
        client::EmailClient,
        messages::{output, runner},
    },
};

/// Compose a new message by delegating to a user-defined composer.
///
/// Looks `<name>` up in `[message.composer.<name>]` and runs its
/// `command` via `sh -c`. With no `<name>`, falls back to the entry
/// flagged `default = true`. The escape hatch `--command "<sh>"`
/// lets you run an ad-hoc command without editing the config.
///
/// The composer takes the terminal: stdin is left empty (new
/// message — no source), stderr is inherited (composer prompts/
/// errors). The composer's stdout must be a valid RFC 5322 message,
/// which himalaya then routes through `--save` / `--send`, or to
/// stdout if neither is set.
#[derive(Debug, Parser)]
pub struct MessageComposeWithCommand {
    /// Name of an entry in `[message.composer.*]`. Optional — when
    /// omitted, the composer flagged `default = true` is used.
    #[arg(value_name = "NAME", conflicts_with = "command")]
    pub name: Option<String>,

    /// Ad-hoc shell command, mutually exclusive with `<name>`.
    /// Useful for trying the feature before editing the config.
    #[arg(long, value_name = "SHELL")]
    pub command: Option<String>,

    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    #[arg(long)]
    pub send: bool,
}

impl MessageComposeWithCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let command = match self.command.as_deref() {
            Some(cmd) => cmd.to_owned(),
            None => runner::resolve_composer(
                Composer::Compose,
                &client.account.composer,
                self.name.as_deref(),
            )?
            .to_owned(),
        };

        let raw = runner::run(&command, &[])?;
        if raw.is_empty() {
            bail!("composer `{command}` produced no output");
        }

        output::route(printer, &mut client, raw, self.save.as_deref(), self.send)
    }
}
