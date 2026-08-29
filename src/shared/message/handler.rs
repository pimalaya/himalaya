//! # Message handler
//!
//! Where the MIME bytes a composer produced go: stdout, a mailbox, the
//! send path, or a mailbox then the send path.
//!
//! [`route`] runs one of those and prints a generic success line, which
//! is what the composers want. A caller needing a richer line, `message
//! add` naming the id it appended, calls [`apply`] and renders the
//! [`Outcome`] itself.

use std::io::{Write, stdout};

use anyhow::Result;
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    account::context::Account,
    email::flag::{Flag, IanaFlag},
    shared::client::EmailClient,
};

/// What [`apply`] did with the bytes.
pub enum Outcome {
    /// Neither saved nor sent, so written to stdout.
    Stdout,
    /// Saved to a mailbox, and sent too when asked.
    Saved {
        /// The id the backend assigned the new message.
        id: String,
        /// Whether it was sent as well as saved.
        sent: bool,
    },
    /// Sent without being saved, the send path returning no id.
    Sent,
}

/// Saves the bytes, sends them, or both, printing nothing.
///
/// Saving resolves the mailbox through the account's aliases and attaches
/// the given flags. With neither asked for, the bytes go to stdout.
pub fn apply(
    account: &Account,
    client: &mut EmailClient,
    raw: Vec<u8>,
    flags: &[Flag],
    save: Option<&str>,
    send: bool,
) -> Result<Outcome> {
    if !send && save.is_none() {
        let mut out = stdout().lock();
        out.write_all(&raw)?;
        return Ok(Outcome::Stdout);
    }

    let saved_id = match save {
        Some(name) => {
            let mailbox = account.resolve_mailbox(name);
            Some(client.add_message(mailbox, flags, raw.clone())?)
        }
        None => None,
    };

    if send {
        client.send_message(raw)?;
    }

    Ok(match saved_id {
        Some(id) => Outcome::Saved { id, sent: send },
        None => Outcome::Sent,
    })
}

/// Runs [`apply`] with `\Seen` as the saved flag and prints a generic
/// success line.
pub fn route(
    printer: &mut impl Printer,
    account: &Account,
    client: &mut EmailClient,
    raw: Vec<u8>,
    save: Option<&str>,
    send: bool,
) -> Result<()> {
    let outcome = apply(
        account,
        client,
        raw,
        &[Flag::from_iana(IanaFlag::Seen)],
        save,
        send,
    )?;
    let msg = match outcome {
        Outcome::Stdout => return Ok(()),
        Outcome::Saved { sent: true, .. } => "Message successfully saved and sent",
        Outcome::Saved { sent: false, .. } => "Message successfully saved",
        Outcome::Sent => "Message successfully sent",
    };
    printer.out(Message::new(msg))
}
