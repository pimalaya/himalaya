//! # Gmail backend
//!
//! The Gmail adapter of the unified client, glue over the
//! io-gmail client [`GmailClient`] wraps.
//!
//! Gmail is label-based, so a label is a mailbox, nested by its name, and
//! a handful of system labels back the shared flags.

use std::{collections::BTreeSet, thread, time::Duration};

use anyhow::{Result, anyhow};
use chrono::{DateTime, FixedOffset, Utc};
use io_gmail::v1::{
    batch::{GmailBatch, GmailBatchRequest, GmailBatchResponse, GmailBatchResponseError},
    client::GmailClientStdError,
    rest::{
        labels::GmailLabel,
        messages::{
            GmailInternalDateSource, GmailMessage, GmailMessageFormat, decode_raw, encode_raw,
            insert::GmailMessageInsert, list::GmailMessagesListParams,
        },
        settings::{get_vacation::GmailVacationGet, update_vacation::GmailVacationUpdate},
    },
    send::GmailApiError,
};
use mime::header::marker::IN_REPLY_TO;
use pimalaya_cli::sanitize;
use serde::de::{DeserializeOwned, IgnoredAny};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id, parse_message_ids},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole, child_path, renamed_path, validate_level},
        outcome::Outcome,
        search::{query::SearchEmailsQuery, sort::eval::sort_envelopes},
        vacation::{Vacation, VacationPatch, date_from_millis},
    },
    error::{ErrorCode, ItemError, missing_message},
    gmail::{client::GmailClient, search},
    shared::message::header::parse_headers,
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
/// Gmail system label every sent message carries.
const SENT: &str = "SENT";

/// The calls one batch holds, as many as Google advises for Gmail.
const BATCH_SIZE: usize = 50;

/// How many times a throttled call is sent again.
const BATCH_RETRIES: usize = 3;

/// The wait before a retry when Gmail names none.
const BATCH_DEFAULT_WAIT: Duration = Duration::from_secs(1);

/// The longest `Retry-After` honoured between two tries.
const BATCH_MAX_WAIT: Duration = Duration::from_secs(10);

impl GmailClient {
    /// Lists every Gmail label as a mailbox. With `with_counts`, issues
    /// one `labels.get` per label to fill totals/unread, a label Gmail
    /// refuses it for listed without counts, beside its error.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Outcome<Vec<Mailbox>>> {
        let labels = self.labels_list()?.response.labels;

        if !with_counts {
            let mailboxes: Vec<Mailbox> = labels.into_iter().map(mailbox_from).collect();
            return Ok(Outcome::from(mailboxes));
        }

        let mut mailboxes = Vec::with_capacity(labels.len());
        let mut errors = Vec::new();
        for label in labels {
            match self.label_get(&label.id) {
                Ok(full) => mailboxes.push(mailbox_from(full.response)),
                Err(GmailClientStdError::Send(err)) if err.api().is_some() => {
                    let err = anyhow!(err).context("Cannot count the messages of the label");
                    let mut error = ItemError::new(Some(&label.id), &err);
                    error.code = Some(ErrorCode::MailboxUnreadable);
                    errors.push(error);
                    mailboxes.push(mailbox_from(label));
                }
                Err(err) => return Err(err.into()),
            }
        }

        Ok(Outcome {
            value: mailboxes,
            errors,
        })
    }

    /// Lists envelopes from the `mailbox` label: `messages.list` (walking
    /// the opaque page token to the requested 1-indexed page) then the
    /// metadata of the ids, through batched `messages.get`.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Outcome<Vec<Envelope>>> {
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
                None => return Ok(Outcome::from(Vec::new())),
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

        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        let answers = self.messages_get_batch(&ids, GmailMessageFormat::Metadata)?;
        Ok(listed(&ids, answers))
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
    ) -> Result<Outcome<Vec<Envelope>>> {
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
                None => return Ok(Outcome::from(Vec::new())),
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

        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        let answers = self.messages_get_batch(&ids, GmailMessageFormat::Metadata)?;
        let mut outcome = listed(&ids, answers);

        if let Some(sort) = query.and_then(|query| query.sort.as_deref()) {
            sort_envelopes(&mut outcome.value, Some(sort));
        }

        Ok(outcome)
    }

    /// Adds, sets, or removes `flags` via `messages.modify` on the ids
    /// carrying the `mailbox` label, returning how many it modified beside
    /// the error of each id it did not.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        let (add, remove) = label_patch(flags, op);
        let found = self.ids_in(mailbox, ids)?;

        // NOTE: an IANA flag without a label (`$Forwarded`) leaves nothing
        // to patch, and an empty `messages.modify` is a wasted request that
        // could still fail.
        if add.is_empty() && remove.is_empty() {
            return Ok(found.map(|ids| ids.len()));
        }

        let modified = self.modify_each(&found.value, &add, &remove)?;
        Ok(modified.after(found.errors))
    }

    /// The ids of `ids` whose message carries the `mailbox` label, through
    /// batched `messages.get` (minimal), Gmail ids being account-wide. An
    /// id Gmail does not know, or whose message lacks the label, is an
    /// item error, as is one Gmail could not answer for.
    fn ids_in(&mut self, mailbox: &str, ids: &[&str]) -> Result<Outcome<Vec<String>>> {
        let answers = self.messages_get_batch(ids, GmailMessageFormat::Minimal)?;
        Ok(labelled(mailbox, ids, answers))
    }

    /// Adds and removes labels on each of `ids` through batched
    /// `messages.modify`.
    fn modify_each(
        &mut self,
        ids: &[String],
        add: &[String],
        remove: &[String],
    ) -> Result<Outcome<usize>> {
        let requests = ids
            .iter()
            .enumerate()
            .map(|(i, id)| GmailBatchRequest::message_modify(i, &self.user_id, id, add, remove))
            .collect::<Result<Vec<_>, _>>()?;

        self.write_each(ids, &requests)
    }

    /// Sends `requests`, one write per message of `ids`, returning how
    /// many Gmail wrote beside the error of each message it did not.
    fn write_each(
        &mut self,
        ids: &[String],
        requests: &[GmailBatchRequest],
    ) -> Result<Outcome<usize>> {
        let answers = self.batch_each::<IgnoredAny>(requests)?;
        let mut outcome = Outcome::from(0);

        for (id, answer) in ids.iter().zip(answers) {
            match answer {
                Ok(_) => outcome.value += 1,
                Err(err) => outcome.errors.push(batch_item_error(id, &err)),
            }
        }

        Ok(outcome)
    }

    /// Reads the messages of `ids` in `format` through batched
    /// `messages.get`, one answer per id in the order given.
    fn messages_get_batch(
        &mut self,
        ids: &[&str],
        format: GmailMessageFormat,
    ) -> Result<Vec<Result<GmailMessage, GmailBatchResponseError>>> {
        // NOTE: a part is named by its index, an id given on the command
        // line being no safe `Content-ID`.
        let requests = ids
            .iter()
            .enumerate()
            .map(|(i, id)| GmailBatchRequest::message_get(i, &self.user_id, id, format, &[]))
            .collect::<Result<Vec<_>, _>>()?;

        self.batch_each(&requests)
    }

    /// Sends `requests` through [`GmailBatch`], as [`batch_all`] splits
    /// and retries them.
    fn batch_each<T: DeserializeOwned>(
        &mut self,
        requests: &[GmailBatchRequest],
    ) -> Result<Vec<Result<T, GmailBatchResponseError>>> {
        batch_all(requests, |batch| {
            let coroutine = GmailBatch::<T>::new(&self.auth, batch)?;
            Ok(self.run(coroutine)?.response)
        })
    }

    /// Fetches one message's raw RFC 5322 bytes via `messages.get`
    /// (format=RAW), base64url-decoded.
    pub fn get_message(&mut self, _mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
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
            let (add, remove) = label_patch(&[seen], FlagOp::Add);
            self.message_modify(id, &add, &remove)?;
        }

        Ok(raw)
    }

    /// Appends a raw message under the `mailbox` label through
    /// `messages.insert`, returning the id Gmail assigned.
    ///
    /// The flags become labels, an unseen message carrying `UNREAD`, and
    /// the message's own `Date:` dates it, as an IMAP `APPEND` would.
    /// Inserting files the message without sending it or running filters,
    /// in the thread of the message it answers.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let (mut label_ids, _) = label_patch(flags, FlagOp::Set);
        label_ids.insert(0, mailbox.to_string());

        let message = GmailMessage {
            thread_id: self.reply_thread_id(&raw)?,
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

    /// Copies the ids carrying the `from` label into `to` by adding `to`'s
    /// label, returning how many it copied beside the error of each id it
    /// did not.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let found = self.ids_in(from, ids)?;
        let add = vec![to.to_string()];

        let copied = self.modify_each(&found.value, &add, &[])?;

        Ok(copied.after(found.errors))
    }

    /// Moves the ids carrying the `from` label to `to` by adding `to`'s
    /// label and removing `from`'s, returning how many it moved beside the
    /// error of each id it did not. A move onto the same label changes
    /// nothing.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let found = self.ids_in(from, ids)?;
        if from == to {
            return Ok(found.map(|ids| ids.len()));
        }

        let add = vec![to.to_string()];
        let remove = vec![from.to_string()];

        let moved = self.modify_each(&found.value, &add, &remove)?;

        Ok(moved.after(found.errors))
    }

    /// Permanently deletes the ids carrying the `mailbox` label
    /// (`users.messages.delete`), returning how many it deleted beside the
    /// error of each id it did not.
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let found = self.ids_in(mailbox, ids)?;

        let requests = found
            .value
            .iter()
            .enumerate()
            .map(|(i, id)| GmailBatchRequest::message_delete(i, &self.user_id, id))
            .collect::<Result<Vec<_>, _>>()?;

        let deleted = self.write_each(&found.value, &requests)?;

        Ok(deleted.after(found.errors))
    }

    /// Creates the label `name` at the top, or under `parent` as
    /// `<parent>/<name>`, Gmail nesting labels by name, returning its id.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        validate_level(name, '/')?;

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
    ///
    /// Gmail nests labels by name alone, so each sublabel is renamed too,
    /// as IMAP `RENAME` moves the inferiors.
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<String> {
        validate_level(name, '/')?;

        let current = self.label_get(mailbox)?.response.name;
        let renamed = renamed_path(&current, '/', name);
        let labels = self.labels_list()?.response.labels;

        let mut renames = vec![(mailbox.to_string(), renamed.clone())];
        renames.extend(sublabel_renames(&labels, &current, &renamed));

        for (id, name) in renames {
            let label = GmailLabel {
                id,
                name,
                ..Default::default()
            };
            self.label_patch(&label)?;
        }

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
        let millis = |date: DateTime<Utc>| date.timestamp_millis().to_string();

        if let Some(from) = patch.from {
            settings.start_time = from.map(millis);
        }
        if let Some(to) = patch.to {
            settings.end_time = to.map(millis);
        }
        if let Some(subject) = patch.subject {
            settings.response_subject = subject;
        }
        if let Some(text) = patch.text {
            settings.response_body_plain_text = text;
        }
        if let Some(html) = patch.html {
            settings.response_body_html = html;
        }

        let update = GmailVacationUpdate::new(&self.auth, &self.user_id, settings)?;
        self.run(update)?;

        self.get_vacation()
    }

    /// Sends a raw RFC 5322 message via `messages.send` (Gmail both
    /// stores it in Sent and delivers it), in the thread of the message
    /// it answers, returning the id Gmail gave the sent message.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<String> {
        let message = GmailMessage {
            thread_id: self.reply_thread_id(&raw)?,
            raw: Some(encode_raw(&raw)),
            ..Default::default()
        };
        Ok(self.message_send(&message)?.response.id)
    }

    /// Files the sent message `id` under the `mailbox` label with `flags`,
    /// in place of a second copy: Gmail already files it under `SENT`.
    pub fn file_sent(&mut self, id: &str, mailbox: &str, flags: &[Flag]) -> Result<()> {
        let (mut add, remove) = label_patch(flags, FlagOp::Add);
        if mailbox != SENT {
            add.push(mailbox.to_string());
        }

        if !add.is_empty() || !remove.is_empty() {
            self.message_modify(id, &add, &remove)?;
        }

        Ok(())
    }

    /// The thread of the first `In-Reply-To:` parent of `raw` the account
    /// holds, found by `rfc822msgid:`, `None` when none is.
    ///
    /// Gmail threads a message given its `threadId` only, headers alone
    /// starting a new thread.
    fn reply_thread_id(&mut self, raw: &[u8]) -> Result<Option<String>> {
        for parent in reply_parents(raw) {
            let q = format!("rfc822msgid:{parent}");
            let params = GmailMessagesListParams {
                q: Some(&q),
                max_results: Some(1),
                include_spam_trash: true,
                ..Default::default()
            };

            let messages = self.messages_list(&params)?.response.messages;
            if let Some(thread_id) = messages.into_iter().find_map(|m| m.thread_id) {
                return Ok(Some(thread_id));
            }
        }

        Ok(None)
    }
}

/// The `In-Reply-To:` message ids of `raw` a Gmail query can carry as
/// they are, an id holding a space, a quote, a brace or a parenthesis
/// being skipped rather than read as an operator.
fn reply_parents(raw: &[u8]) -> Vec<String> {
    let message = parse_headers(raw);

    message
        .root
        .header::<IN_REPLY_TO>()
        .unwrap_or_default()
        .iter()
        .filter_map(|id| normalize_message_id(id))
        .filter(|id| {
            id.chars()
                .all(|c| c.is_ascii_graphic() && !matches!(c, '"' | '{' | '}' | '(' | ')'))
        })
        .collect()
}

/// Whether listing `mailbox` requires `includeSpamTrash`: Gmail hides
/// SPAM and TRASH messages from `messages.list` unless asked.
fn include_spam_trash(mailbox: &str) -> bool {
    matches!(mailbox, SPAM | "TRASH")
}

/// The `(id, name)` patches moving the sublabels of the label `old` under
/// `new`, Gmail nesting labels by their `/`-separated names.
fn sublabel_renames(labels: &[GmailLabel], old: &str, new: &str) -> Vec<(String, String)> {
    let prefix = format!("{old}/");

    labels
        .iter()
        .filter_map(|label| {
            let rest = label.name.strip_prefix(&prefix)?;
            Some((label.id.clone(), format!("{new}/{rest}")))
        })
        .collect()
}

/// Whether Gmail refused a message id as unknown: `404` for an id of the
/// right shape, `400 Invalid id value` for one it cannot parse.
fn is_unknown_id(err: &GmailApiError) -> bool {
    err.status == 404 || (err.status == 400 && err.message == "Invalid id value")
}

/// Sends `requests` through `send`, at most [`BATCH_SIZE`] to a batch,
/// sending a part Gmail throttled or failed transiently again after the
/// longest `Retry-After` its round asked for, up to [`BATCH_RETRIES`]
/// times.
///
/// Returns one answer per request, in order, a part still failing keeping
/// its last error and one left unanswered read as missing.
fn batch_all<T>(
    requests: &[GmailBatchRequest],
    mut send: impl FnMut(&[GmailBatchRequest]) -> Result<Vec<GmailBatchResponse<T>>>,
) -> Result<Vec<Result<T, GmailBatchResponseError>>> {
    let mut answers: Vec<Option<Result<T, GmailBatchResponseError>>> =
        requests.iter().map(|_| None).collect();
    let mut pending: Vec<usize> = (0..requests.len()).collect();

    for attempt in 0..=BATCH_RETRIES {
        let mut retry = Vec::new();
        let mut wait = Duration::ZERO;

        for chunk in pending.chunks(BATCH_SIZE) {
            let batch: Vec<GmailBatchRequest> =
                chunk.iter().map(|i| requests[*i].clone()).collect();

            for response in send(&batch)? {
                let Some(index) = response.id.parse().ok().filter(|i| chunk.contains(i)) else {
                    continue;
                };

                match response.result {
                    Err(err) if attempt < BATCH_RETRIES && is_retryable(&err) => {
                        wait = wait.max(err.retry_after().unwrap_or(BATCH_DEFAULT_WAIT));
                        retry.push(index);
                    }
                    result => answers[index] = Some(result),
                }
            }
        }

        if retry.is_empty() {
            break;
        }

        thread::sleep(wait.min(BATCH_MAX_WAIT));
        pending = retry;
    }

    let missing = || Err(GmailBatchResponseError::Missing);
    Ok(answers
        .into_iter()
        .map(|answer| answer.unwrap_or_else(missing))
        .collect())
}

/// Whether a batched call is worth sending again: throttled, or a
/// transient server failure. A part left unanswered is not.
fn is_retryable(err: &GmailBatchResponseError) -> bool {
    err.api().is_some_and(GmailApiError::is_retryable)
}

/// The envelopes of the `answers` to batched `messages.get` of `ids`,
/// beside the error of each one Gmail could not answer for.
///
/// A message answered 404 was deleted since the list was read, so it is
/// left out rather than reported.
fn listed(
    ids: &[&str],
    answers: Vec<Result<GmailMessage, GmailBatchResponseError>>,
) -> Outcome<Vec<Envelope>> {
    let mut outcome = Outcome::from(Vec::with_capacity(ids.len()));

    for (id, answer) in ids.iter().zip(answers) {
        match answer {
            Ok(message) => outcome.value.push(envelope_from(message)),
            Err(err) if err.is_not_found() => continue,
            Err(err) => outcome.errors.push(batch_item_error(id, &err)),
        }
    }

    outcome
}

/// The ids of `ids` whose message, in the `answers` to batched
/// `messages.get`, carries the `mailbox` label, beside the error of each
/// other one.
fn labelled(
    mailbox: &str,
    ids: &[&str],
    answers: Vec<Result<GmailMessage, GmailBatchResponseError>>,
) -> Outcome<Vec<String>> {
    let mut outcome = Outcome::from(Vec::with_capacity(ids.len()));

    for (id, answer) in ids.iter().zip(answers) {
        match answer {
            Ok(message) if message.label_ids.iter().any(|label| label == mailbox) => {
                outcome.value.push(id.to_string());
            }
            Ok(_) => outcome.errors.push(missing_message(id)),
            Err(GmailBatchResponseError::Api(err)) if is_unknown_id(&err) => {
                outcome.errors.push(missing_message(id));
            }
            Err(err) => outcome.errors.push(batch_item_error(id, &err)),
        }
    }

    outcome
}

/// The item error of the message `id` a batched call failed on: Gmail's
/// refusal, an answer that does not parse, or no answer at all.
fn batch_item_error(id: &str, err: &GmailBatchResponseError) -> ItemError {
    match err {
        GmailBatchResponseError::Api(err) => api_item_error(id, err),
        GmailBatchResponseError::ParseResponse(_) => ItemError {
            id: Some(id.to_owned()),
            code: Some(ErrorCode::MessageUnreadable),
            error: String::from("Cannot parse the Gmail message"),
            sources: vec![err.to_string()],
        },
        GmailBatchResponseError::Missing => ItemError {
            id: Some(id.to_owned()),
            code: None,
            error: err.to_string(),
            sources: Vec::new(),
        },
    }
}

/// The item error of the message `id` Gmail refused, coded by its status
/// and reasons, which `sources` keeps.
fn api_item_error(id: &str, err: &GmailApiError) -> ItemError {
    let code = if err.is_rate_limited() {
        Some(ErrorCode::RateLimited)
    } else if err.status == 403 {
        Some(ErrorCode::PermissionDenied)
    } else if is_unknown_id(err) {
        Some(ErrorCode::MessageNotFound)
    } else {
        None
    };

    let mut sources = vec![format!("HTTP {}", err.status)];
    sources.extend(
        err.reasons
            .iter()
            .map(|reason| sanitize::text(reason).into_owned()),
    );

    ItemError {
        id: Some(id.to_owned()),
        code,
        error: format!(
            "Gmail refused the message: {}",
            sanitize::text(&err.message)
        ),
        sources,
    }
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
        SENT => Some(MailboxRole::Sent),
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
            // NOTE: no `DRAFT` target: Gmail refuses it from a client in
            // either list, failing the whole request.
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use io_gmail::v1::{
        batch::{GmailBatchRequest, GmailBatchResponse, GmailBatchResponseError},
        rest::{labels::GmailLabel, messages::GmailMessage},
        send::GmailApiError,
    };

    use crate::{
        email::flag::{Flag, FlagOp, IanaFlag},
        error::{ErrorCode, ItemError},
        gmail::backend::{
            batch_all, batch_item_error, is_unknown_id, label_patch, labelled, listed,
            reply_parents, sublabel_renames,
        },
    };

    /// A batch answering each call with what `answer` says for its index
    /// and round, recording the indexes each round sent.
    fn batch(
        count: usize,
        answer: impl Fn(usize, usize) -> Result<u32, GmailBatchResponseError>,
    ) -> (Vec<Result<u32, GmailBatchResponseError>>, Vec<Vec<usize>>) {
        let requests: Vec<_> = (0..count)
            .map(|i| GmailBatchRequest {
                id: i.to_string(),
                ..Default::default()
            })
            .collect();
        let mut sent: Vec<Vec<usize>> = Vec::new();

        let answers = batch_all(&requests, |batch| {
            let round = sent.len();
            let indexes: Vec<usize> = batch.iter().map(|r| r.id.parse().unwrap()).collect();
            sent.push(indexes.clone());

            Ok(indexes
                .into_iter()
                .map(|i| GmailBatchResponse {
                    id: i.to_string(),
                    status: None,
                    result: answer(i, round),
                })
                .collect())
        })
        .unwrap();

        (answers, sent)
    }

    fn throttled() -> GmailBatchResponseError {
        let mut err = api_error(429, "Too many requests");
        err.retry_after = Some(Duration::ZERO);
        GmailBatchResponseError::Api(err)
    }

    /// A batch holds 50 calls at most; a throttled part is sent again
    /// alone, a refused or unanswered one is not.
    #[test]
    fn a_batch_retries_only_the_throttled_parts() {
        let (answers, sent) = batch(52, |i, round| match (i, round) {
            (1, 0) => Err(throttled()),
            (2, _) => Err(refused(403, "insufficientPermissions")),
            (3, _) => Err(GmailBatchResponseError::Missing),
            (i, _) => Ok(i as u32),
        });

        let lens: Vec<usize> = sent.iter().map(Vec::len).collect();
        assert_eq!(lens, [50, 2, 1]);
        assert_eq!(sent[2], [1]);

        assert_eq!(answers.len(), 52);
        assert_eq!(*answers[1].as_ref().unwrap(), 1);
        assert_eq!(answers[2].as_ref().unwrap_err().status(), Some(403));
        assert!(matches!(answers[3], Err(GmailBatchResponseError::Missing)));
        assert_eq!(*answers[51].as_ref().unwrap(), 51);
    }

    /// A part still throttled after the retries keeps its last error,
    /// reported `rate-limited`.
    #[test]
    fn a_part_throttled_after_the_retries_is_rate_limited() {
        let (answers, sent) = batch(1, |_, _| Err(throttled()));

        assert_eq!(sent.len(), 4);
        let err = answers[0].as_ref().unwrap_err();
        let error = batch_item_error("a", err);
        assert_eq!(error.code, Some(ErrorCode::RateLimited));
    }

    /// A write Gmail answers 404 is not found.
    #[test]
    fn a_write_answered_404_is_not_found() {
        let error = batch_item_error("a", &refused(404, "notFound"));
        assert_eq!(error.code, Some(ErrorCode::MessageNotFound));
    }

    fn label(id: &str, name: &str) -> GmailLabel {
        GmailLabel {
            id: id.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    fn api_error(status: u16, message: &str) -> GmailApiError {
        GmailApiError {
            status,
            message: message.into(),
            ..Default::default()
        }
    }

    fn refused(status: u16, reason: &str) -> GmailBatchResponseError {
        let mut err = api_error(status, "Refused");
        err.reasons = vec![reason.into()];
        GmailBatchResponseError::Api(err)
    }

    fn message(id: &str, labels: &[&str]) -> GmailMessage {
        GmailMessage {
            id: id.into(),
            label_ids: labels.iter().map(|label| label.to_string()).collect(),
            ..Default::default()
        }
    }

    fn codes(errors: &[ItemError]) -> Vec<(Option<&str>, Option<ErrorCode>)> {
        errors
            .iter()
            .map(|error| (error.id.as_deref(), error.code))
            .collect()
    }

    /// A part answered 404 is left out of a listing, the others coded by
    /// what Gmail answered, an unanswered one coded `null`.
    #[test]
    fn a_listing_reports_the_parts_gmail_could_not_answer() {
        let ids = ["a", "gone", "slow", "denied", "lost"];
        let answers = vec![
            Ok(message("a", &["INBOX"])),
            Err(refused(404, "notFound")),
            Err(refused(429, "rateLimitExceeded")),
            Err(refused(403, "insufficientPermissions")),
            Err(GmailBatchResponseError::Missing),
        ];

        let outcome = listed(&ids, answers);

        let listed: Vec<&str> = outcome.value.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(listed, ["a"]);
        assert_eq!(
            codes(&outcome.errors),
            [
                (Some("slow"), Some(ErrorCode::RateLimited)),
                (Some("denied"), Some(ErrorCode::PermissionDenied)),
                (Some("lost"), None),
            ]
        );
    }

    /// An id Gmail does not know, or whose message lacks the label, is not
    /// found; a 403 throttle is a rate limit, not a refusal.
    #[test]
    fn a_lookup_keeps_the_ids_carrying_the_label() {
        let ids = ["in", "elsewhere", "unknown", "malformed", "throttled"];
        let answers = vec![
            Ok(message("in", &["Label_1"])),
            Ok(message("elsewhere", &["INBOX"])),
            Err(refused(404, "notFound")),
            Err(GmailBatchResponseError::Api(api_error(
                400,
                "Invalid id value",
            ))),
            Err(refused(403, "userRateLimitExceeded")),
        ];

        let outcome = labelled("Label_1", &ids, answers);

        assert_eq!(outcome.value, ["in"]);
        assert_eq!(
            codes(&outcome.errors),
            [
                (Some("elsewhere"), Some(ErrorCode::MessageNotFound)),
                (Some("unknown"), Some(ErrorCode::MessageNotFound)),
                (Some("malformed"), Some(ErrorCode::MessageNotFound)),
                (Some("throttled"), Some(ErrorCode::RateLimited)),
            ]
        );
        assert_eq!(
            outcome.errors[3].sources,
            ["HTTP 403", "userRateLimitExceeded"]
        );
    }

    #[test]
    fn a_flag_set_leaves_the_draft_label_alone() {
        let flagged = Flag::from_iana(IanaFlag::Flagged);
        let (add, remove) = label_patch(&[flagged], FlagOp::Set);

        assert_eq!(add, ["UNREAD", "STARRED"]);
        assert_eq!(remove, ["IMPORTANT", "SPAM"]);
    }

    #[test]
    fn a_rename_moves_the_sublabels_and_only_them() {
        let labels = [
            label("1", "A/Sub"),
            label("2", "A/Sub/Deep"),
            label("3", "A/Sub/Deep/Er"),
            label("4", "A/Subway"),
            label("5", "B/A/Sub/X"),
        ];

        assert_eq!(
            sublabel_renames(&labels, "A/Sub", "A/Sub2"),
            [
                ("2".to_string(), "A/Sub2/Deep".to_string()),
                ("3".to_string(), "A/Sub2/Deep/Er".to_string()),
            ]
        );
    }

    #[test]
    fn unknown_ids_are_told_from_other_failures() {
        assert!(is_unknown_id(&api_error(
            404,
            "Requested entity was not found."
        )));
        assert!(is_unknown_id(&api_error(400, "Invalid id value")));
        assert!(!is_unknown_id(&api_error(400, "Invalid label: DRAFT")));
        assert!(!is_unknown_id(&api_error(403, "Forbidden")));
    }

    #[test]
    fn a_reply_names_the_parents_a_query_can_carry() {
        let raw = b"In-Reply-To: <a@x.org> <{d}@x.org> <e@x.org>\r\n\r\nbody\r\n";
        assert_eq!(reply_parents(raw), ["a@x.org", "e@x.org"]);
        assert!(reply_parents(b"Subject: hi\r\n\r\nbody\r\n").is_empty());
    }
}
