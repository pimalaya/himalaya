//! # Submission
//!
//! The RFC 5321 envelope of a message about to be sent, derived from its
//! headers: `From:` becomes the sender, and `To:`, `Cc:` and `Bcc:` the
//! recipients.
//!
//! Every path that sends a raw message derives it here, so the SMTP
//! transport and the pimdir queue cannot disagree on who receives it.

use anyhow::{Result, anyhow, bail};
use mime::{
    header::marker::{BCC, CC, FROM, SUBJECT, TO},
    tree::{cst::MimeCst, parse::MimeParseOptions},
    value::address::MimeAddressList,
};

/// The sender and recipients a message is submitted with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmissionEnvelope {
    /// The first `From:` address, bare.
    pub from: String,
    /// Every `To:`, `Cc:` and `Bcc:` address, bare, in header order.
    pub rcpts: Vec<String>,
    /// The decoded `Subject:`, when there is one.
    pub subject: Option<String>,
}

impl SubmissionEnvelope {
    /// Derives the envelope from the headers of a raw RFC 5322 message.
    ///
    /// A message with no `From:` address or no recipient is refused, since
    /// no transport can submit it.
    pub fn parse(raw: &[u8]) -> Result<Self> {
        // NOTE: the header block alone, the body left unsplit.
        let opts = MimeParseOptions {
            structure_depth: Some(0),
            ..Default::default()
        };
        let parsed = MimeCst::parse_with(raw, opts);
        let root = &parsed.root;

        let from = root
            .header::<FROM>()
            .and_then(|list| addresses(&list).into_iter().next())
            .ok_or_else(|| anyhow!("No `From:` header found in raw message"))?;

        let rcpts: Vec<String> = [
            root.header::<TO>(),
            root.header::<CC>(),
            root.header::<BCC>(),
        ]
        .into_iter()
        .flatten()
        .flat_map(|list| addresses(&list))
        .collect();

        if rcpts.is_empty() {
            bail!("No `To:` / `Cc:` / `Bcc:` recipients found in raw message");
        }

        Ok(Self {
            from,
            rcpts,
            subject: root.header::<SUBJECT>(),
        })
    }
}

/// Flattens an address list, groups included, into bare
/// `local-part@domain` strings.
fn addresses(list: &MimeAddressList) -> Vec<String> {
    list.mailboxes()
        .filter_map(|mailbox| mailbox.address.clone())
        .filter(|email| !email.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::SubmissionEnvelope;

    #[test]
    fn every_recipient_header_feeds_the_envelope() {
        let raw = concat!(
            "From: Alice <a@x.org>\r\n",
            "To: b@y.org\r\n",
            "Cc: Carol <c@y.org>\r\n",
            "Bcc: d@y.org\r\n",
            "Subject: =?utf-8?q?R=C3=A9union?=\r\n",
            "\r\n",
            "body",
        );

        let envelope = SubmissionEnvelope::parse(raw.as_bytes()).unwrap();

        assert_eq!(envelope.from, "a@x.org");
        assert_eq!(envelope.rcpts, ["b@y.org", "c@y.org", "d@y.org"]);
        assert_eq!(envelope.subject.as_deref(), Some("Réunion"));
    }

    #[test]
    fn no_sender_or_no_recipient_is_refused() {
        let no_from = SubmissionEnvelope::parse(b"To: b@y.org\r\n\r\nbody");
        let no_rcpt = SubmissionEnvelope::parse(b"From: a@x.org\r\n\r\nbody");

        assert!(no_from.unwrap_err().to_string().contains("From:"));
        assert!(no_rcpt.unwrap_err().to_string().contains("recipients"));
    }
}
