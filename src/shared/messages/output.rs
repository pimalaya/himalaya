//! Post-composer routing: where the produced MIME bytes go.
//!
//! Used by `compose` / `reply` / `forward` (and their `-with`
//! variants). The same `--save <mbox>` / `--send` flags can combine:
//! `--save Sent --send` sends the message *and* appends a copy to the
//! `Sent` mailbox. With neither flag, the raw bytes are written to
//! stdout — same shape as a manual `mml compile > out.eml`.

use std::io::{stdout, Write};

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
        let opts = client.send_opts.clone();
        client.send_message(raw, opts)?;
        return printer.out(Message::new("Message successfully sent"));
    }

    printer.out(Message::new("Message saved"))
}
