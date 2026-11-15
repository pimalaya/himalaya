//! # Message handler
//!
//! Where the MIME bytes a composer produced go: stdout, a mailbox, the
//! send path, or a mailbox then the send path.
//!
//! [`route`] runs one of those and prints its receipt, which is what the
//! composers want. A caller needing a richer line, `message
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
use log::debug;
use mime::{
    header::marker::{BCC, CC, FROM, SUBJECT, TO},
    tree::cst::MimeCst,
    value::address::{MimeAddressList, MimeMailbox},
};
use pimalaya_cli::{output::Bytes, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    capability::Capability,
    email::{
        address::Address,
        flag::{Flag, IanaFlag},
    },
    shared::{
        attachment::list::attachments,
        client::{EmailClient, SentCopy},
        message::part,
    },
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

    // NOTE: a backend filing every sent message itself (Graph) keeps that
    // copy wherever `--save` points, `--no-save` being refused up front.
    if send && client.files_sent_copy() {
        if let Some(mailbox) = mailbox {
            debug!("the backend files the sent copy itself, ignoring `{mailbox}`");
        }
        client.send_message(None, flags, raw, false)?;
        return Ok(Outcome::Saved {
            id: None,
            sent: true,
        });
    }

    // NOTE: checked before the send, so a backend that cannot hold the
    // copy refuses the whole command rather than send and then fail.
    if mailbox.is_some() {
        client.require(&[Capability::MessageAdd])?;
    }

    let copy = match send {
        true => {
            let sent = mailbox.or_else(|| account.mailbox_alias.get("sent").map(String::as_str));
            client.send_message(sent, flags, raw.clone(), mailbox.is_some())?
        }
        false => SentCopy::Owed,
    };

    let saved_id = match (mailbox, copy) {
        (Some(_), SentCopy::Filed(id)) => Some(id),
        (Some(mailbox), SentCopy::Owed) if send => Some(
            client
                .add_message(mailbox, flags, raw)
                .with_context(|| format!("Message sent, but saving a copy to {mailbox} failed"))?,
        ),
        (Some(mailbox), SentCopy::Owed) => Some(client.add_message(mailbox, flags, raw)?),
        (None, _) => None,
    };

    Ok(match saved_id {
        Some(id) => Outcome::Saved { id, sent: send },
        None => Outcome::Sent,
    })
}

/// Runs [`apply`] with `\Seen` as the saved flag and prints its receipt,
/// or the message itself when neither saving nor sending.
pub fn route(
    printer: &mut impl Printer,
    account: &Account,
    client: &mut EmailClient,
    raw: Vec<u8>,
    save: Option<&str>,
    send: bool,
) -> Result<()> {
    if !send && save.is_none() {
        if printer.is_json() {
            return printer.out(MessageTemplateOutput::parse(&raw)?);
        }

        return printer.out_bytes(Bytes {
            data: raw,
            mime: "message/rfc822".into(),
        });
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
    let output = match outcome {
        Outcome::Stdout => return Ok(()),
        Outcome::Saved { id, sent } => MessageRouteOutput {
            id,
            saved: true,
            sent,
            queued,
        },
        Outcome::Sent => MessageRouteOutput {
            id: None,
            saved: false,
            sent: true,
            queued,
        },
    };

    printer.out(output)
}

/// The receipt of a saved or sent message.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageRouteOutput {
    /// The id of the saved copy, `null` when not saved, queued, or not
    /// returned by the backend.
    pub id: Option<String>,
    /// Whether a copy was saved to a mailbox.
    pub saved: bool,
    /// Whether the message was sent.
    pub sent: bool,
    /// Whether a sync engine applies the write later rather than now.
    pub queued: bool,
}

impl fmt::Display for MessageRouteOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match (self.saved, self.sent) {
            (true, true) => "saving and sending",
            (true, false) => "saving",
            _ => "sending",
        };

        if self.queued {
            return writeln!(f, "Message queued for {what}");
        }

        let done = match (self.saved, self.sent) {
            (true, true) => "saved and sent",
            (true, false) => "saved",
            _ => "sent",
        };

        match &self.id {
            Some(id) => writeln!(f, "Message {id} successfully {done}"),
            None => writeln!(f, "Message successfully {done}"),
        }
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
    /// The first `From:` mailbox, `null` without one.
    pub from: Option<Address>,
    /// The `To:` mailboxes, groups flattened.
    pub to: Vec<Address>,
    /// The `Cc:` mailboxes.
    pub cc: Vec<Address>,
    /// The `Bcc:` mailboxes.
    pub bcc: Vec<Address>,
    /// The decoded `Subject:`, `null` without one.
    pub subject: Option<String>,
    /// The text body, its line breaks LF, as an editor shows them.
    pub body: String,
    /// The attached files, which ride along with the message: the template
    /// carries their names, not their bytes, so handing it back means
    /// passing them again through `--attach`.
    pub attachments: Vec<TemplateAttachment>,
}

/// A file attached to a composed message.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateAttachment {
    /// The file name it is attached under, `None` when it has none.
    pub filename: Option<String>,
    /// The MIME type, `None` when the part carries no `Content-Type`.
    pub mime: Option<String>,
    /// The size of its content, in bytes.
    pub size: u64,
}

impl MessageTemplateOutput {
    /// Decodes the headers and the text body of raw RFC 5322 bytes.
    fn parse(raw: &[u8]) -> Result<Self> {
        let message = MimeCst::parse(raw);
        let root = &message.root;

        let mailboxes = |address: Option<MimeAddressList>| -> Vec<Address> {
            address
                .map(|address| address.mailboxes().map(mailbox).collect())
                .unwrap_or_default()
        };

        // NOTE: the text is read back from the CRLF bytes the builder wrote.
        let body = part::body_text(&message).replace("\r\n", "\n");
        let attachments = attachments(&message)
            .map(|attachment| TemplateAttachment {
                filename: attachment.filename,
                mime: attachment.mime,
                size: attachment.size,
            })
            .collect();

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
            attachments,
        })
    }
}

/// One mailbox of a decoded address list, its display name `None` when
/// empty.
fn mailbox(addr: &MimeMailbox) -> Address {
    Address {
        name: addr.name.clone().filter(|name| !name.is_empty()),
        email: addr.address.clone().unwrap_or_default(),
    }
}

/// Formats one mailbox as the composer flags parse it back, quoting a
/// display name that holds an RFC 5322 special such as a comma.
fn template_mailbox(addr: &Address) -> String {
    let email = addr.email.as_str();
    let Some(name) = addr.name.as_deref() else {
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
            writeln!(f, "From: {}", template_mailbox(from))?;
        }
        for (name, list) in [("To", &self.to), ("Cc", &self.cc), ("Bcc", &self.bcc)] {
            if !list.is_empty() {
                let list: Vec<_> = list.iter().map(template_mailbox).collect();
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

        let json = serde_json::to_value(&template).unwrap();
        assert_eq!(
            json["from"],
            serde_json::json!({"name": "Clément", "email": "c@example.org"})
        );
        assert_eq!(
            json["to"],
            serde_json::json!([
                {"name": null, "email": "a@example.org"},
                {"name": "Bob", "email": "b@example.org"},
                {"name": "Doe, J.", "email": "j@example.org"},
            ])
        );
        assert_eq!(json["cc"], serde_json::json!([]));
        assert_eq!(json["subject"], "Réunion");
        assert_eq!(template.body, "à demain\n");
        assert!(template.attachments.is_empty());

        let text = template.to_string();
        assert!(
            text.starts_with("From: Clément <c@example.org>\n"),
            "{text}"
        );
        let to = "To: a@example.org, Bob <b@example.org>, \"Doe, J.\" <j@example.org>\n";
        assert!(text.contains(to), "{text}");
    }

    /// A template without a sender prints `from` as `null`.
    #[test]
    fn a_template_without_a_sender_prints_null() {
        let raw = "To: a@example.org\r\n\r\nhi\r\n";
        let template = MessageTemplateOutput::parse(raw.as_bytes()).unwrap();

        let json = serde_json::to_value(&template).unwrap();
        assert_eq!(json["from"], serde_json::Value::Null);
        assert_eq!(json["subject"], serde_json::Value::Null);
    }

    #[test]
    fn template_lists_the_attachments_and_writes_lf() {
        let raw = concat!(
            "To: a@example.org\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=b\r\n",
            "\r\n",
            "--b\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "\r\n",
            "> quoted\r\n",
            ">\r\n",
            "mine\r\n",
            "--b\r\n",
            "Content-Type: message/rfc822\r\n",
            "Content-Disposition: attachment; filename=inner.eml\r\n",
            "\r\n",
            "Subject: inner\r\n",
            "\r\n",
            "body\r\n",
            "--b--\r\n",
        );

        let template = MessageTemplateOutput::parse(raw.as_bytes()).unwrap();

        assert_eq!(template.body, "> quoted\n>\nmine");
        assert_eq!(template.attachments.len(), 1);
        let attachment = &template.attachments[0];
        assert_eq!(attachment.filename.as_deref(), Some("inner.eml"));
        assert_eq!(attachment.mime.as_deref(), Some("message/rfc822"));
        assert!(attachment.size > 0);
    }
}
