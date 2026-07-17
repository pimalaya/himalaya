//! Microsoft Graph adapter for the shared cross-protocol client.
//!
//! Thin glue over [`MsgraphClient`], which wraps io_msgraph's high-level
//! client (`mail_folders_list`, `messages_list`, `message_get_raw`,
//! `message_update`, `message_copy`, `message_move`, `mail_send_mime`).
//! Graph is folder-based; flags map to the message's scalar fields
//! (`isRead`, follow-up flag, importance) plus `categories`. No
//! `add_message`. The conversion is lifted from the retired io-email
//! Graph drivers.

use std::collections::BTreeSet;

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use io_msgraph::v1::rest::users::{
    mail_folders::{MsgraphMailFolder, list::MsgraphMailFoldersListParams},
    messages::{
        MsgraphFlagStatus, MsgraphFollowupFlag, MsgraphImportance, MsgraphMessage,
        MsgraphRecipient, list::MsgraphMessagesListParams,
    },
};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
    },
    msgraph::client::MsgraphClient,
};

/// OData `$select` for envelope listing: the message fields backing the
/// shared [`Envelope`], returned in one round-trip.
const ENVELOPE_SELECT: &str = "id,subject,from,toRecipients,receivedDateTime,isRead,isDraft,hasAttachments,internetMessageId,importance,flag,categories";

impl MsgraphClient {
    /// Lists every Graph mail folder as a mailbox. Graph carries counts
    /// inline, so `with_counts` is ignored.
    pub fn list_mailboxes(&mut self, _with_counts: bool) -> Result<Vec<Mailbox>> {
        let params = MsgraphMailFoldersListParams {
            top: Some(100),
            ..Default::default()
        };
        let folders = self.mail_folders_list(&params)?.response.value;
        Ok(folders.into_iter().map(mailbox_from).collect())
    }

    /// Lists envelopes from the `mailbox` folder (id or well-known name
    /// such as `inbox`). Page 1 is the most recent window; the offset is
    /// `(page - 1) * page_size` via OData `$skip`.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let page = page.unwrap_or(1).max(1);
        let (top, skip) = match page_size {
            Some(size) => (Some(size), Some((page - 1) * size)),
            None => (None, None),
        };

        let params = MsgraphMessagesListParams {
            top,
            skip,
            select: Some(ENVELOPE_SELECT),
            ..Default::default()
        };
        let messages = self.messages_list(Some(mailbox), &params)?.response.value;

        Ok(messages.into_iter().map(envelope_from).collect())
    }

    /// Adds, sets, or removes `flags` on a message id set via one
    /// `PATCH /messages/{id}` per id. `mailbox` is unused.
    pub fn store_flags(
        &mut self,
        _mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let patch = flag_patch(flags, op);
        for id in ids {
            self.message_update(id, &patch)?;
        }
        Ok(())
    }

    /// Fetches one message's raw RFC 5322 bytes via
    /// `GET /messages/{id}/$value`. `mailbox` is unused.
    pub fn get_message(&mut self, _mailbox: &str, id: &str) -> Result<Vec<u8>> {
        Ok(self.message_get_raw(id)?.response)
    }

    /// Copies a message id set into `to`. `from` is unused.
    pub fn copy_messages(&mut self, _from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        for id in ids {
            self.message_copy(id, to)?;
        }
        Ok(ids.len())
    }

    /// Moves a message id set into `to`. `from` is unused.
    pub fn move_messages(&mut self, _from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        for id in ids {
            self.message_move(id, to)?;
        }
        Ok(ids.len())
    }

    /// Sends a raw RFC 5322 message via `POST /sendMail` (MIME form);
    /// Graph saves it to Sent Items.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        self.mail_send_mime(&raw)?;
        Ok(())
    }
}

/// Converts one Graph mail folder into the shared [`Mailbox`] shape;
/// Graph folders carry their counts inline.
fn mailbox_from(folder: MsgraphMailFolder) -> Mailbox {
    Mailbox {
        id: folder.id,
        name: folder.display_name,
        total: folder.total_item_count,
        unread: folder.unread_item_count,
    }
}

/// Folds a Graph message into the shared [`Envelope`] shape. `size` is
/// left `0`: the listing selection does not expose a raw size.
fn envelope_from(message: MsgraphMessage) -> Envelope {
    let flags = flags_from_message(&message);
    let message_id = message
        .internet_message_id
        .as_deref()
        .and_then(normalize_message_id);
    let date = message
        .received_date_time
        .as_deref()
        .and_then(parse_rfc3339);
    let from = message.from.map(address_from).into_iter().collect();
    let to = message
        .to_recipients
        .into_iter()
        .map(address_from)
        .collect();

    Envelope {
        id: message.id,
        message_id,
        flags,
        subject: message.subject.unwrap_or_default(),
        from,
        to,
        date,
        size: 0,
        has_attachment: message.has_attachments,
    }
}

/// Builds the message PATCH body applying `flags` under `op`. `Set`
/// drives every scalar flag-field to its target and replaces
/// `categories`; `Add`/`Remove` only touch the scalar fields mentioned.
fn flag_patch(flags: &[Flag], op: FlagOp) -> MsgraphMessage {
    let mut patch = MsgraphMessage::default();

    match op {
        FlagOp::Set => {
            patch.is_read = Some(flags.iter().any(Flag::is_seen));
            patch.flag = Some(followup(flags.iter().any(Flag::is_flagged)));
            patch.importance = Some(importance(flags.iter().any(Flag::is_important)));
            patch.categories = flags
                .iter()
                .filter(|flag| flag.iana().is_none())
                .map(|flag| flag.raw().to_string())
                .collect();
        }
        FlagOp::Add => {
            for flag in flags {
                if flag.is_seen() {
                    patch.is_read = Some(true);
                }
                if flag.is_flagged() {
                    patch.flag = Some(followup(true));
                }
                if flag.is_important() {
                    patch.importance = Some(MsgraphImportance::High);
                }
            }
        }
        FlagOp::Remove => {
            for flag in flags {
                if flag.is_seen() {
                    patch.is_read = Some(false);
                }
                if flag.is_flagged() {
                    patch.flag = Some(followup(false));
                }
                if flag.is_important() {
                    patch.importance = Some(MsgraphImportance::Normal);
                }
            }
        }
    }

    patch
}

/// Derives the shared flag set from a Graph message's scalar fields and
/// categories.
fn flags_from_message(message: &MsgraphMessage) -> BTreeSet<Flag> {
    let mut flags = BTreeSet::new();

    if message.is_read == Some(true) {
        flags.insert(Flag::from_iana(IanaFlag::Seen));
    }
    if matches!(
        message.flag.as_ref().and_then(|flag| flag.flag_status),
        Some(MsgraphFlagStatus::Flagged)
    ) {
        flags.insert(Flag::from_iana(IanaFlag::Flagged));
    }
    if message.importance == Some(MsgraphImportance::High) {
        flags.insert(Flag::from_iana(IanaFlag::Important));
    }
    if message.is_draft == Some(true) {
        flags.insert(Flag::from_iana(IanaFlag::Draft));
    }
    for category in &message.categories {
        flags.insert(Flag::from_raw(category.clone()));
    }

    flags
}

/// Converts a Graph recipient into a shared [`Address`].
fn address_from(recipient: MsgraphRecipient) -> Address {
    Address {
        name: recipient.email_address.name,
        email: recipient.email_address.address.unwrap_or_default(),
    }
}

fn followup(flagged: bool) -> MsgraphFollowupFlag {
    let flag_status = if flagged {
        MsgraphFlagStatus::Flagged
    } else {
        MsgraphFlagStatus::NotFlagged
    };
    MsgraphFollowupFlag {
        flag_status: Some(flag_status),
    }
}

fn importance(important: bool) -> MsgraphImportance {
    if important {
        MsgraphImportance::High
    } else {
        MsgraphImportance::Normal
    }
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(trimmed).ok()
}
