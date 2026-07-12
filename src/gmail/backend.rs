//! Gmail adapter for the shared cross-protocol client.
//!
//! Thin glue over [`GmailClient`], which wraps io_gmail's high-level
//! client (`labels_list`, `label_get`, `messages_list`, `message_get`,
//! `message_modify`, `message_send`). Gmail is label-based: labels are
//! mailboxes, and a subset of system labels back the shared flags. No
//! `add_message` (Gmail has no append). The conversion is lifted from
//! the retired io-email Gmail drivers.

use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use chrono::{DateTime, FixedOffset};
use io_gmail::v1::rest::{
    labels::GmailLabel,
    messages::{
        GmailMessage, GmailMessageFormat, decode_raw, encode_raw, list::GmailMessagesListParams,
    },
};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
    },
    gmail::client::GmailClient,
};

/// Gmail system label marking an unread message; its absence means seen.
const UNREAD: &str = "UNREAD";
/// Gmail system label backing the shared `\Flagged` flag.
const STARRED: &str = "STARRED";
/// Gmail system label backing the shared `$Important` flag.
const IMPORTANT: &str = "IMPORTANT";
/// Gmail system label backing the shared `\Draft` flag.
const DRAFT: &str = "DRAFT";
/// Gmail system label backing the shared `$Junk` flag.
const SPAM: &str = "SPAM";

impl GmailClient {
    /// Lists every Gmail label as a mailbox. With `with_counts`, issues
    /// one `labels.get` per label to fill totals/unread.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let labels = self.labels_list()?.response.labels;

        if !with_counts {
            return Ok(labels.into_iter().map(mailbox_from).collect());
        }

        let mut mailboxes = Vec::with_capacity(labels.len());
        for label in &labels {
            let full = self.label_get(&label.id)?.response;
            mailboxes.push(mailbox_from(full));
        }
        Ok(mailboxes)
    }

    /// Lists envelopes from the `mailbox` label: `messages.list` (walking
    /// the opaque page token to the requested 1-indexed page) then one
    /// `messages.get` (metadata) per id.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let label_ids = vec![mailbox.to_string()];
        let spam_trash = include_spam_trash(mailbox);
        let skip = page.unwrap_or(1).max(1) - 1;

        let mut page_token: Option<String> = None;
        for _ in 0..skip {
            let params = GmailMessagesListParams {
                label_ids: &label_ids,
                max_results: page_size,
                page_token: page_token.as_deref(),
                include_spam_trash: spam_trash,
                ..Default::default()
            };
            match self.messages_list(&params)?.response.next_page_token {
                Some(token) => page_token = Some(token),
                None => return Ok(Vec::new()),
            }
        }

        let params = GmailMessagesListParams {
            label_ids: &label_ids,
            max_results: page_size,
            page_token: page_token.as_deref(),
            include_spam_trash: spam_trash,
            ..Default::default()
        };
        let ids: Vec<String> = self
            .messages_list(&params)?
            .response
            .messages
            .into_iter()
            .map(|message| message.id)
            .collect();

        let mut envelopes = Vec::with_capacity(ids.len());
        for id in &ids {
            let message = self
                .message_get(id, GmailMessageFormat::Metadata, &[])?
                .response;
            envelopes.push(envelope_from(message));
        }
        Ok(envelopes)
    }

    /// Adds, sets, or removes `flags` on a message id set via
    /// `messages.modify`. `mailbox` is unused: Gmail labels are global.
    pub fn store_flags(
        &mut self,
        _mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let (add, remove) = label_patch(flags, op);
        for id in ids {
            self.message_modify(id, &add, &remove)?;
        }
        Ok(())
    }

    /// Fetches one message's raw RFC 5322 bytes via `messages.get`
    /// (format=RAW), base64url-decoded.
    pub fn get_message(&mut self, _mailbox: &str, id: &str) -> Result<Vec<u8>> {
        let message = self.message_get(id, GmailMessageFormat::Raw, &[])?.response;
        let raw = message
            .raw
            .ok_or_else(|| anyhow!("Gmail message has no raw payload"))?;
        decode_raw(&raw).map_err(|err| anyhow!("Gmail raw message could not be decoded: {err}"))
    }

    /// Copies a message id set into `to` by adding `to`'s label. `from`
    /// is unused (labels are additive).
    pub fn copy_messages(&mut self, _from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let add = vec![to.to_string()];
        for id in ids {
            self.message_modify(id, &add, &[])?;
        }
        Ok(())
    }

    /// Moves a message id set from `from` to `to` by adding `to`'s label
    /// and removing `from`'s.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let add = vec![to.to_string()];
        let remove = vec![from.to_string()];
        for id in ids {
            self.message_modify(id, &add, &remove)?;
        }
        Ok(())
    }

    /// Sends a raw RFC 5322 message via `messages.send` (Gmail both
    /// stores it in Sent and delivers it).
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            ..Default::default()
        };
        self.message_send(&message)?;
        Ok(())
    }
}

/// Whether listing `mailbox` requires `includeSpamTrash`: Gmail hides
/// SPAM and TRASH messages from `messages.list` unless asked.
fn include_spam_trash(mailbox: &str) -> bool {
    matches!(mailbox, SPAM | "TRASH")
}

/// Converts one Gmail label into the shared [`Mailbox`] shape.
fn mailbox_from(label: GmailLabel) -> Mailbox {
    Mailbox {
        id: label.id,
        name: label.name,
        total: label.messages_total,
        unread: label.messages_unread,
    }
}

/// Folds a Gmail message (metadata format) into the shared [`Envelope`]
/// shape. `has_attachment` is left `None`: Gmail metadata does not
/// expose the MIME structure.
fn envelope_from(message: GmailMessage) -> Envelope {
    let flags = flags_from_labels(&message.label_ids);
    let size = message.size_estimate.unwrap_or(0);

    let mut subject = String::new();
    let mut from = Vec::new();
    let mut to = Vec::new();
    let mut date = None;
    let mut message_id = None;

    if let Some(payload) = &message.payload {
        subject = payload.header("Subject").unwrap_or_default().to_string();
        from = parse_addresses(payload.header("From").unwrap_or_default());
        to = parse_addresses(payload.header("To").unwrap_or_default());
        date = payload.header("Date").and_then(parse_rfc2822);
        message_id = payload.header("Message-ID").and_then(normalize_message_id);
    }

    Envelope {
        id: message.id,
        message_id,
        flags,
        subject,
        from,
        to,
        date,
        size,
        has_attachment: None,
    }
}

/// Derives the shared flag set from a Gmail message's label ids. Only
/// the flag-like system labels are surfaced.
fn flags_from_labels(label_ids: &[String]) -> BTreeSet<Flag> {
    let has = |label: &str| label_ids.iter().any(|id| id == label);

    let mut flags = BTreeSet::new();
    if !has(UNREAD) {
        flags.insert(Flag::from_iana(IanaFlag::Seen));
    }
    if has(STARRED) {
        flags.insert(Flag::from_iana(IanaFlag::Flagged));
    }
    if has(IMPORTANT) {
        flags.insert(Flag::from_iana(IanaFlag::Important));
    }
    if has(DRAFT) {
        flags.insert(Flag::from_iana(IanaFlag::Draft));
    }
    if has(SPAM) {
        flags.insert(Flag::from_iana(IanaFlag::Junk));
    }
    flags
}

/// Translates a flag-store operation into Gmail `(addLabelIds,
/// removeLabelIds)`. `\Seen` maps to the absence of `UNREAD`, so its
/// polarity is inverted; flags with no Gmail equivalent are dropped;
/// custom keywords pass through as label ids.
fn label_patch(flags: &[Flag], op: FlagOp) -> (Vec<String>, Vec<String>) {
    let mut add = Vec::new();
    let mut remove = Vec::new();

    match op {
        FlagOp::Add => {
            for flag in flags {
                if let Some((label, inverted)) = label_of(flag) {
                    if inverted { &mut remove } else { &mut add }.push(label);
                }
            }
        }
        FlagOp::Remove => {
            for flag in flags {
                if let Some((label, inverted)) = label_of(flag) {
                    if inverted { &mut add } else { &mut remove }.push(label);
                }
            }
        }
        FlagOp::Set => {
            target(
                flags.iter().any(Flag::is_seen),
                UNREAD,
                true,
                &mut add,
                &mut remove,
            );
            target(
                flags.iter().any(Flag::is_flagged),
                STARRED,
                false,
                &mut add,
                &mut remove,
            );
            target(
                flags.iter().any(Flag::is_important),
                IMPORTANT,
                false,
                &mut add,
                &mut remove,
            );
            target(
                flags.iter().any(Flag::is_draft),
                DRAFT,
                false,
                &mut add,
                &mut remove,
            );
            target(
                flags.iter().any(Flag::is_junk),
                SPAM,
                false,
                &mut add,
                &mut remove,
            );

            for flag in flags {
                if flag.iana().is_none() {
                    add.push(flag.raw().to_string());
                }
            }
        }
    }

    (add, remove)
}

/// Maps a shared flag to its Gmail `(label, inverted)` pair, or `None`
/// when Gmail has no equivalent and the flag should be dropped.
fn label_of(flag: &Flag) -> Option<(String, bool)> {
    match flag.iana() {
        Some(IanaFlag::Seen) => Some((UNREAD.into(), true)),
        Some(IanaFlag::Flagged) => Some((STARRED.into(), false)),
        Some(IanaFlag::Important) => Some((IMPORTANT.into(), false)),
        Some(IanaFlag::Draft) => Some((DRAFT.into(), false)),
        Some(IanaFlag::Junk) => Some((SPAM.into(), false)),
        Some(_) => None,
        None => Some((flag.raw().to_string(), false)),
    }
}

/// Pushes `label` onto `add` or `remove` so its final presence matches
/// `wanted`, honouring the inverted polarity of `UNREAD`.
fn target(
    wanted: bool,
    label: &str,
    inverted: bool,
    add: &mut Vec<String>,
    remove: &mut Vec<String>,
) {
    let present = wanted ^ inverted;
    if present { add } else { remove }.push(label.into());
}

/// Parses an RFC 5322 address-list header value into shared addresses
/// (best-effort: split on commas, extract `<addr>` plus optional name).
fn parse_addresses(raw: &str) -> Vec<Address> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(parse_address)
        .collect()
}

fn parse_address(part: &str) -> Address {
    if let Some(open) = part.rfind('<') {
        if let Some(end) = part[open..].find('>') {
            let email = part[open + 1..open + end].trim().to_string();
            let name = part[..open].trim().trim_matches('"').trim();
            let name = (!name.is_empty()).then(|| name.to_string());
            return Address { name, email };
        }
    }
    Address {
        name: None,
        email: part.to_string(),
    }
}

fn parse_rfc2822(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc2822(trimmed).ok()
}
