//! Post-build routing: where the produced MIME bytes go.
//!
//! [`apply`] performs the requested side-effects (stdout dump,
//! save-to-mailbox, send, or save-then-send) and returns an
//! [`Outcome`] describing what happened. [`route`] is a thin wrapper
//! that prints a generic "Message successfully X" line based on the
//! outcome, used by the built-in flag composers
//! (`compose` / `reply` / `forward`) and by `messages send`.
//!
//! Callers that need a richer success line (e.g. `messages add`
//! reporting the appended backend id) call [`apply`] directly and
//! render their own output from the [`Outcome`].
//!
//! [`Account::resolve_mailbox`]: crate::account::context::Account::resolve_mailbox

use std::io::{Write, stdout};

use anyhow::Result;
use io_email::flag::types::{Flag, IanaFlag};
use pimalaya_cli::printer::{Message, Printer};

use crate::{account::context::Account, shared::client::EmailClient};

/// What [`apply`] actually did with `raw`.
pub enum Outcome {
    /// Neither `save` nor `send`: bytes were written to stdout.
    Stdout,
    /// Saved to a mailbox; `id` is the backend-assigned id of the
    /// new message, `sent` is `true` when `send` was also requested.
    Saved { id: String, sent: bool },
    /// Sent only (no save). The send path returns no id.
    Sent,
}

/// Performs the requested combination of side-effects without
/// printing anything. `save` writes a copy to the named mailbox
/// (resolved through the account's alias map) with `flags` attached;
/// `send` pushes the message through the configured SMTP / JMAP send
/// path. With neither set, dumps `raw` to stdout.
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

/// Generic wrapper over [`apply`]: hard-codes `\Seen` as the saved
/// flag and prints a "Message successfully X" line. Used by the
/// built-in flag composers and by `messages send`.
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
