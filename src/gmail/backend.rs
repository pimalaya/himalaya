//! # Gmail backend
//!
//! The Gmail adapter of the shared cross-protocol client, glue over the
//! io-gmail client [`GmailClient`] wraps.
//!
//! Gmail is label-based, so a label is a mailbox, nested by its name, and
//! a handful of system labels back the shared flags.

use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use chrono::{DateTime, FixedOffset};
use io_gmail::v1::rest::{
    labels::GmailLabel,
    messages::{
        GmailInternalDateSource, GmailMessage, GmailMessageFormat, decode_raw, encode_raw,
        insert::GmailMessageInsert, list::GmailMessagesListParams,
    },
    settings::{get_vacation::GmailVacationGet, update_vacation::GmailVacationUpdate},
};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id, parse_message_ids},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole, child_path, renamed_path},
        search::query::SearchEmailsQuery,
        vacation::{Vacation, VacationPatch, date_from_millis},
    },
    gmail::{client::GmailClient, search},
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

    /// Searches the `mailbox` label with the shared query: the filter
    /// half translates into a `messages.list` `q` value, the sort half
    /// orders the fetched page client-side, Gmail keeping a
    /// newest-first list order with no server-side sort.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let q = match query.and_then(|query| query.filter.as_ref()) {
            Some(filter) => Some(search::filter_to_q(filter)?),
            None => None,
        };

        let label_ids = vec![mailbox.to_string()];
        let spam_trash = include_spam_trash(mailbox);
        let skip = page.unwrap_or(1).max(1) - 1;

        let mut page_token: Option<String> = None;
        for _ in 0..skip {
            let params = GmailMessagesListParams {
                q: q.as_deref(),
                label_ids: &label_ids,
                max_results: page_size,
                page_token: page_token.as_deref(),
                include_spam_trash: spam_trash,
            };
            match self.messages_list(&params)?.response.next_page_token {
                Some(token) => page_token = Some(token),
                None => return Ok(Vec::new()),
            }
        }

        let params = GmailMessagesListParams {
            q: q.as_deref(),
            label_ids: &label_ids,
            max_results: page_size,
            page_token: page_token.as_deref(),
            include_spam_trash: spam_trash,
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

        if let Some(sorters) = query.and_then(|query| query.sort.as_deref()) {
            search::sort_envelopes(&mut envelopes, sorters);
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
    ) -> Result<usize> {
        let (add, remove) = label_patch(flags, op);

        // NOTE: an IANA flag without a label (`$Forwarded`) leaves nothing
        // to patch, and an empty `messages.modify` is a wasted request that
        // could still fail. The ids are then not checked and all count as
        // found. `\Answered` and `\Draft` never get here: the shared API
        // refuses them up front (`flags.answered`, `flags.draft`).
        if add.is_empty() && remove.is_empty() {
            return Ok(ids.len());
        }

        for id in ids {
            self.message_modify(id, &add, &remove)?;
        }
        Ok(ids.len())
    }

    /// Fetches one message's raw RFC 5322 bytes via `messages.get`
    /// (format=RAW), base64url-decoded.
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let message = self.message_get(id, GmailMessageFormat::Raw, &[])?.response;
        let raw = message
            .raw
            .ok_or_else(|| anyhow!("Gmail message has no raw payload"))?;
        let raw = decode_raw(&raw)
            .map_err(|err| anyhow!("Gmail raw message could not be decoded: {err}"))?;

        // NOTE: `messages.get` never changes read state, so `--seen` costs
        // a separate `messages.modify` removing the `UNREAD` label.
        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.store_flags(mailbox, &[id], &[seen], FlagOp::Add)?;
        }

        Ok(raw)
    }

    /// Appends a raw message under the `mailbox` label through
    /// `messages.insert`, returning the id Gmail assigned.
    ///
    /// The flags become labels, an unseen message carrying `UNREAD`, and
    /// the message's own `Date:` dates it, as an IMAP `APPEND` would.
    /// Inserting files the message without sending it or running filters.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let (mut label_ids, _) = label_patch(flags, FlagOp::Set);
        label_ids.insert(0, mailbox.to_string());

        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            label_ids,
            ..Default::default()
        };
        let coroutine = GmailMessageInsert::new(
            &self.auth,
            &self.user_id,
            &message,
            Some(GmailInternalDateSource::DateHeader),
            false,
        )?;

        Ok(self.run(coroutine)?.response.id)
    }

    /// Copies a message id set into `to` by adding `to`'s label. `from`
    /// is unused (labels are additive).
    pub fn copy_messages(&mut self, _from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let add = vec![to.to_string()];
        for id in ids {
            self.message_modify(id, &add, &[])?;
        }
        Ok(ids.len())
    }

    /// Moves a message id set from `from` to `to` by adding `to`'s label
    /// and removing `from`'s.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let add = vec![to.to_string()];
        let remove = vec![from.to_string()];
        for id in ids {
            self.message_modify(id, &add, &remove)?;
        }
        Ok(ids.len())
    }

    /// Permanently deletes `ids` (`users.messages.delete`).
    pub fn delete_messages(&mut self, ids: &[&str]) -> Result<()> {
        for id in ids {
            self.message_delete(id)?;
        }
        Ok(())
    }

    /// Creates the label `name` at the top, or under `parent` as
    /// `<parent>/<name>`, Gmail nesting labels by name, returning its id.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        let name = match parent {
            Some(parent) => {
                let parent = self.label_get(parent)?.response.name;
                child_path(&parent, '/', name)
            }
            None => name.to_string(),
        };

        let label = GmailLabel {
            name,
            ..Default::default()
        };

        let label = self.label_create(&label)?.response;
        self.forget_mailboxes();
        Ok(label.id)
    }

    /// Renames the last level of the label `mailbox` to `name`, its parent
    /// kept (`labels.patch`, so its colour and visibility survive),
    /// returning its id, which a rename leaves as it is.
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<String> {
        let current = self.label_get(mailbox)?.response.name;

        let label = GmailLabel {
            id: mailbox.to_string(),
            name: renamed_path(&current, '/', name),
            ..Default::default()
        };

        self.label_patch(&label)?;
        self.forget_mailboxes();
        Ok(mailbox.to_string())
    }

    /// Deletes the label `mailbox` (`labels.delete`), its messages staying
    /// under their other labels or in All Mail.
    pub fn delete_mailbox(&mut self, mailbox: &str) -> Result<()> {
        self.label_delete(mailbox)?;
        self.forget_mailboxes();
        Ok(())
    }

    /// Reads the auto-reply (`settings.getVacation`), its window in UTC.
    pub fn get_vacation(&mut self) -> Result<Vacation> {
        let get = GmailVacationGet::new(&self.auth, &self.user_id)?;
        let settings = self.run(get)?.response;

        Ok(Vacation {
            enabled: settings.enable_auto_reply,
            from: settings.start_time.map(date_from_millis),
            to: settings.end_time.map(date_from_millis),
            subject: settings.response_subject,
            text: settings.response_body_plain_text,
            html: settings.response_body_html,
        })
    }

    /// Changes what `patch` names of the auto-reply, the settings read
    /// first so the rest survives `settings.updateVacation`, a
    /// replacement, and returns it as it then stands.
    pub fn set_vacation(&mut self, patch: VacationPatch) -> Result<Vacation> {
        let get = GmailVacationGet::new(&self.auth, &self.user_id)?;
        let mut settings = self.run(get)?.response;

        if let Some(enabled) = patch.enabled {
            settings.enable_auto_reply = enabled;
        }
        if let Some(from) = patch.from {
            settings.start_time = Some(from.timestamp_millis().to_string());
        }
        if let Some(to) = patch.to {
            settings.end_time = Some(to.timestamp_millis().to_string());
        }
        if let Some(subject) = patch.subject {
            settings.response_subject = Some(subject);
        }
        if let Some(text) = patch.text {
            settings.response_body_plain_text = Some(text);
        }
        if let Some(html) = patch.html {
            settings.response_body_html = Some(html);
        }

        let update = GmailVacationUpdate::new(&self.auth, &self.user_id, settings)?;
        self.run(update)?;

        self.get_vacation()
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
        role: label_role(&label.id),
        id: label.id,
        name: label.name,
        total: label.messages_total,
        unread: label.messages_unread,
    }
}

/// The special-use role of a label, keyed by the system-label ids every
/// Gmail account shares. There is no archive label, archiving being the
/// loss of the `INBOX` one.
fn label_role(id: &str) -> Option<MailboxRole> {
    match id {
        "INBOX" => Some(MailboxRole::Inbox),
        "SENT" => Some(MailboxRole::Sent),
        DRAFT => Some(MailboxRole::Drafts),
        "TRASH" => Some(MailboxRole::Trash),
        SPAM => Some(MailboxRole::Junk),
        STARRED => Some(MailboxRole::Flagged),
        IMPORTANT => Some(MailboxRole::Important),
        _ => None,
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
    let mut in_reply_to = Vec::new();

    if let Some(payload) = &message.payload {
        subject = payload.header("Subject").unwrap_or_default().to_string();
        from = parse_addresses(payload.header("From").unwrap_or_default());
        to = parse_addresses(payload.header("To").unwrap_or_default());
        date = payload.header("Date").and_then(parse_rfc2822);
        message_id = payload.header("Message-ID").and_then(normalize_message_id);
        in_reply_to = parse_message_ids(payload.header("In-Reply-To").unwrap_or_default());
    }

    Envelope {
        id: message.id,
        message_id,
        in_reply_to,
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

/// Parses one address of a header value into a shared [`Address`].
fn parse_address(part: &str) -> Address {
    if let Some(open) = part.rfind('<')
        && let Some(end) = part[open..].find('>')
    {
        let email = part[open + 1..open + end].trim().to_string();
        let name = part[..open].trim().trim_matches('"').trim();
        let name = (!name.is_empty()).then(|| name.to_string());
        return Address { name, email };
    }
    Address {
        name: None,
        email: part.to_string(),
    }
}

/// Parses an RFC 2822 `Date:` header value.
fn parse_rfc2822(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc2822(trimmed).ok()
}
