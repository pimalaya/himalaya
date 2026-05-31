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

//! Post-composer routing: where the produced MIME bytes go.
//!
//! Used by `compose` / `reply` / `forward` (and their `-with`
//! variants). The same `--save <mbox>` / `--send` flags can combine:
//! `--save Sent --send` sends the message *and* appends a copy to the
//! `Sent` mailbox. With neither flag, the raw bytes are written to
//! stdout — same shape as a manual `mml compile > out.eml`.

use std::io::{Write, stdout};

use anyhow::Result;
use pimalaya_cli::printer::{Message, Printer};

use crate::shared::client::EmailClient;

/// Routes `raw` through the requested combination of side-effects.
/// `save` writes a copy to the named mailbox before sending; `send`
/// pushes the message through the configured SMTP / JMAP send path.
/// With neither set, dumps `raw` to stdout and returns.
pub fn route(
    printer: &mut impl Printer,
    client: &mut EmailClient,
    raw: Vec<u8>,
    save: Option<&str>,
    send: bool,
) -> Result<()> {
    if !send && save.is_none() {
        let mut out = stdout().lock();
        out.write_all(&raw)?;
        return Ok(());
    }

    if let Some(mailbox) = save {
        client.add_message(mailbox, &[], raw.clone())?;
    }

    if send {
        client.send_message(raw)?;
        return printer.out(Message::new("Message successfully sent"));
    }

    printer.out(Message::new("Message saved"))
}
