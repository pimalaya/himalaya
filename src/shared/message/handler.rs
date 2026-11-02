//! # Message handler
//!
//! Where the MIME bytes a composer produced go: stdout, a mailbox, the
//! send path, or a mailbox then the send path.
//!
//! [`route`] runs one of those and prints a generic success line, which
//! is what the composers want. A caller needing a richer line, `message
//! add` naming the id it appended, calls [`apply`] and renders the
//! [`Outcome`] itself.
//!
//! Under `--json`, a message neither saved nor sent comes out as a
//! [`MessageTemplateOutput`] rather than raw bytes, so an editor can lay it
//! out.

use std::{
    fmt,
    io::{Write, stdout},
};

use anyhow::{Context, Result};
use mime::{
    header::marker::{BCC, CC, FROM, SUBJECT, TO},
    tree::cst::MimeCst,
    value::address::{MimeAddressList, MimeMailbox},
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    capability::Capability,
    email::flag::{Flag, IanaFlag},
    shared::{client::EmailClient, message::part},
};

/// What [`apply`] did with the bytes.
pub enum Outcome {
    /// Neither saved nor sent, so written to stdout.
    Stdout,
    /// Saved to a mailbox, and sent too when asked.
    Saved {
        /// The id the backend assigned the new message, absent when it
        /// did not report one.
        id: Option<String>,
        /// Whether it was sent as well as saved.
        sent: bool,
    },
    /// Sent without being saved, the send path returning no id.
    Sent,
}

/// Sends the bytes, saves them, or both, printing nothing.
///
/// Saving resolves the mailbox through the account's aliases and attaches
/// the given flags. Both asked for, the send goes first, so a failed send
/// leaves no copy behind, unless the backend files the copy itself as
/// part of the send. With neither, the bytes go to stdout.
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

    let mailbox = save.map(|name| account.resolve_mailbox(name));

    // NOTE: checked before the send, so a backend that cannot hold the
    // copy refuses the whole command rather than send and then fail.
    if mailbox.is_some() {
        client.require(&[Capability::MessageAdd])?;
    }

    let carried = match send {
        true => {
            let sent = mailbox.or_else(|| account.mailbox_alias.get("sent").map(String::as_str));
            client.send_message(sent, raw.clone(), mailbox.is_some())?
        }
        false => false,
    };

    let saved_id = match mailbox {
        Some(_) if carried => Some(None),
        Some(mailbox) if send => Some(
            client
                .add_message(mailbox, flags, raw)
                .with_context(|| format!("Message sent, but saving a copy to {mailbox} failed"))?,
        ),
        Some(mailbox) => Some(client.add_message(mailbox, flags, raw)?),
        None => None,
    };

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
    if printer.is_json() && !send && save.is_none() {
        return printer.out(MessageTemplateOutput::parse(&raw)?);
    }

    let queued = client.queues_writes();
    let outcome = apply(
        account,
        client,
        raw,
        &[Flag::from_iana(IanaFlag::Seen)],
        save,
        send,
    )?;
    let message = match (outcome, queued) {
        (Outcome::Stdout, _) => return Ok(()),
        (Outcome::Saved { sent: true, .. }, false) => "Message successfully saved and sent",
        (Outcome::Saved { sent: false, .. }, false) => "Message successfully saved",
        (Outcome::Sent, false) => "Message successfully sent",
        (Outcome::Saved { sent: true, .. }, true) => "Message queued for saving and sending",
        (Outcome::Saved { sent: false, .. }, true) => "Message queued for saving",
        (Outcome::Sent, true) => "Message queued for sending",
    };
    printer.out(MessageRouteOutput {
        message: message.to_owned(),
    })
}

/// The confirmation line of a saved or sent message.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageRouteOutput {
    /// What happened, for a human.
    pub message: String,
}

impl fmt::Display for MessageRouteOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.message)
    }
}

/// The decoded fields of a composed message, which an editor lays out
/// and hands back through the composer flags.
///
/// The body carries no signature: sending or saving appends the one the
/// account configures.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageTemplateOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub body: String,
}

impl MessageTemplateOutput {
    /// Decodes the headers and the text body of raw RFC 5322 bytes.
    fn parse(raw: &[u8]) -> Result<Self> {
        let message = MimeCst::parse(raw);
        let root = &message.root;

        let mailboxes = |address: Option<MimeAddressList>| -> Vec<String> {
            address
                .map(|address| address.mailboxes().map(mailbox).collect())
                .unwrap_or_default()
        };

        let body = part::body_text(&message);

        Ok(Self {
            from: root
                .header::<FROM>()
                .as_ref()
                .and_then(MimeAddressList::first)
                .map(mailbox),
            to: mailboxes(root.header::<TO>()),
            cc: mailboxes(root.header::<CC>()),
            bcc: mailboxes(root.header::<BCC>()),
            subject: root.header::<SUBJECT>(),
            body,
        })
    }
}

/// Formats one mailbox as the composer flags parse it back, quoting a
/// display name that holds an RFC 5322 special such as a comma.
fn mailbox(addr: &MimeMailbox) -> String {
    let email = addr.address.as_deref().unwrap_or_default();
    let Some(name) = addr.name.as_deref().filter(|name| !name.is_empty()) else {
        return email.to_owned();
    };

    if name.contains(|c| "()<>[]:;@\\,.\"".contains(c)) {
        let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\" <{email}>")
    } else {
        format!("{name} <{email}>")
    }
}

impl fmt::Display for MessageTemplateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(from) = &self.from {
            writeln!(f, "From: {from}")?;
        }
        for (name, list) in [("To", &self.to), ("Cc", &self.cc), ("Bcc", &self.bcc)] {
            if !list.is_empty() {
                writeln!(f, "{name}: {}", list.join(", "))?;
            }
        }
        if let Some(subject) = &self.subject {
            writeln!(f, "Subject: {subject}")?;
        }
        writeln!(f)?;
        write!(f, "{}", self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::MessageTemplateOutput;

    #[test]
    fn template_decodes_headers_and_body_and_quotes_names() {
        let raw = concat!(
            "From: =?utf-8?q?Cl=C3=A9ment?= <c@example.org>\r\n",
            "To: a@example.org, Bob <b@example.org>, \"Doe, J.\" <j@example.org>\r\n",
            "Subject: =?utf-8?q?R=C3=A9union?=\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "Content-Transfer-Encoding: quoted-printable\r\n",
            "\r\n",
            "=C3=A0 demain\r\n",
        );

        let template = MessageTemplateOutput::parse(raw.as_bytes()).unwrap();

        assert_eq!(template.from.as_deref(), Some("Clément <c@example.org>"));
        assert_eq!(
            template.to,
            [
                "a@example.org",
                "Bob <b@example.org>",
                "\"Doe, J.\" <j@example.org>",
            ]
        );
        assert!(template.cc.is_empty());
        assert_eq!(template.subject.as_deref(), Some("Réunion"));
        assert_eq!(template.body, "à demain\r\n");
    }
}
