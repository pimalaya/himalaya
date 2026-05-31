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

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_email::flag::{Flag, FlagOp};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::shared::{
    client::EmailClient,
    flags::arg::{FlagsArg, MessageIdsArg},
    mailboxes::arg::MailboxArg,
};

/// Replace flag(s) of message(s) for the active account.
#[derive(Debug, Parser)]
pub struct FlagSetCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
}

impl FlagSetCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let mailbox = self.mailbox.resolve(&client.account)?;
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        client.store_flags(&mailbox, &ids, &flags, FlagOp::Set)?;

        let flags: Vec<String> = self.flags.inner.iter().map(ToString::to_string).collect();
        printer.out(SetFlags { flags })
    }
}

#[derive(Debug, Serialize)]
struct SetFlags {
    flags: Vec<String>,
}

impl fmt::Display for SetFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Successfully set flags: {}", self.flags.join(", "))
    }
}
