//! # IMAP backend
//!
//! The IMAP adapter of the unified client, glue over the
//! io-imap session [`ImapClient`] already wraps.
//!
//! Each method takes and returns the CLI's own [`crate::unified::email`]
//! types, so the work here is converting between those and io-imap's wire
//! types.

use io_imap::client::ImapClient as _;
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    str::from_utf8,
};

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, FixedOffset};
use io_imap::{
    client::ImapClientError,
    rfc3501::{
        append::ImapMessageAppendOptions,
        copy::{ImapCopyUid, ImapMessageCopyOptions},
        fetch::ImapMessageFetchOptions,
        search::ImapMessageSearchOptions,
        select::{ImapMailboxSelectData, ImapMailboxSelectOptions},
        store::ImapMessageStoreOptions,
    },
    rfc5256::sort::ImapMessageSortOptions,
    rfc6851::r#move::ImapMessageMoveOptions,
    send::ImapOutput,
    types::{
        body::{BodyStructure, SpecificFields},
        core::{AString, Atom, Vec1},
        datetime::NaiveDate as ImapNaiveDate,
        extensions::sort::{SortCriterion, SortKey},
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        flag::{Flag as ImapFlag, FlagFetch, FlagNameAttribute, FlagPerm, StoreType},
        mailbox::{ListMailbox, Mailbox as ImapMailbox},
        response::{Code, StatusKind},
        search::SearchKey,
        sequence::SequenceSet,
        status::{StatusDataItem, StatusDataItemName},
    },
};
use log::debug;
use mime::header::marker::MESSAGE_ID;
use pimalaya_cli::error::CodedError;

use crate::{
    error::{
        ErrorCode, ItemError, existing_mailbox, mailbox_not_found, message_not_found,
        missing_message, unknown_mailbox,
    },
    imap::{
        client::ImapClient,
        utils::{output_data, to_addresses},
    },
    unified::{
        capability,
        email::{
            envelope::{
                Envelope, EnvelopePage, normalize_message_id, page_follows, parse_message_ids,
                parse_rfc2822_date,
            },
            flag::{Flag, FlagOp, IanaFlag},
            mailbox::{
                Mailbox, MailboxIndex, MailboxRole, child_path, renamed_path, validate_level,
            },
            outcome::Outcome,
            search::{
                error::unsupported,
                filter::query::SearchEmailsFilterQuery,
                query::SearchEmailsQuery,
                sort::query::{
                    SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder,
                },
            },
        },
        message::{header, part},
    },
};

impl ImapClient {
    /// Lists every selectable mailbox, following each with a `STATUS`
    /// when counts were asked for.
    ///
    /// A `LIST` line that does not decode is an item error naming no
    /// mailbox, and a mailbox whose `STATUS` the server refuses or that
    /// does not decode is listed without counts, beside its error.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Outcome<Vec<Mailbox>>> {
        let reference: ImapMailbox<'static> = ""
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list reference"))?;
        let pattern: ListMailbox<'static> = "*"
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list pattern"))?;

        let output = self.list(reference, pattern)?;
        let mut errors: Vec<ItemError> = output
            .skipped
            .iter()
            .map(|_| unreadable_mailbox(None, "Cannot decode a line of the LIST response"))
            .collect();
        let rows = output_data("LIST", output);

        let mut mailboxes: Vec<Mailbox> = rows
            .into_iter()
            .filter(is_selectable)
            .map(mailbox_from)
            .collect();

        if with_counts {
            for mailbox in &mut mailboxes {
                let mbox = parse_mailbox(&mailbox.id)?;
                let names = vec![StatusDataItemName::Messages, StatusDataItemName::Unseen];
                let output = match self.status(mbox, names.into()) {
                    Ok(output) => output,
                    Err(err) if err.status().is_some() => {
                        let gone = err
                            .status()
                            .and_then(|status| status.code.as_ref())
                            .is_some_and(names_no_mailbox);
                        let id = Some(mailbox.id.as_str());
                        let err = anyhow!(err).context("Cannot count the messages");
                        let mut error = ItemError::new(id, &err);
                        error.code = Some(match gone {
                            true => ErrorCode::MailboxNotFound,
                            false => ErrorCode::MailboxUnreadable,
                        });
                        errors.push(error);
                        continue;
                    }
                    Err(err) => return Err(err.into()),
                };

                let undecoded = !output.skipped.is_empty();
                let items = output_data("STATUS", output);
                if items.is_empty() && undecoded {
                    let error = "Cannot decode the STATUS response";
                    errors.push(unreadable_mailbox(Some(&mailbox.id), error));
                    continue;
                }

                apply_status(mailbox, items);
            }
        }

        Ok(Outcome {
            value: mailboxes,
            errors,
        })
    }

    /// Maps a role (`sent`, `trash`...) onto its special-use mailbox, one
    /// literally named so winning; other names pass through untouched.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        match MailboxRole::known(mailbox) {
            None | Some(MailboxRole::Inbox) => Ok(mailbox.to_string()),
            Some(_) => MailboxIndex(self.list_mailboxes(false)?.value).resolve(mailbox),
        }
    }

    /// The mailbox carrying `role`, `None` when none does: `INBOX`, else
    /// the one whose special-use attribute names it.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        if *role == MailboxRole::Inbox {
            return Ok(Some(String::from("INBOX")));
        }

        MailboxIndex(self.list_mailboxes(false)?.value).with_role(role)
    }

    /// Lists envelopes most recent first, the whole mailbox when neither
    /// page nor page size is given.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Outcome<Vec<Envelope>>> {
        self.list_envelopes_counted(mailbox, page, page_size, with_attachment)
            .map(|(listed, _)| listed.map(|page| page.envelopes))
    }

    /// Sends `NOOP`, keeping an idle connection from the server's
    /// autologout.
    pub fn keep_alive(&mut self) -> Result<()> {
        self.noop()?;
        Ok(())
    }

    /// [`Self::list_envelopes`] beside the mailbox's message count, the
    /// `EXISTS` of the `SELECT` the listing sends anyway.
    pub fn list_envelopes_counted(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<(Outcome<EnvelopePage>, Option<u64>)> {
        let select = self.select_mailbox(mailbox)?;
        let exists = output_data("SELECT", select).exists;
        let total = exists.map(u64::from);

        let Some(window) = compute_window(exists.unwrap_or(0), page, page_size) else {
            return Ok((Outcome::from(EnvelopePage::default()), total));
        };
        let more = page_follows(total.unwrap_or(0), page, page_size);
        let sequence_set: SequenceSet = window
            .as_str()
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP sequence-set window `{window}`"))?;

        let output = self.fetch(
            sequence_set,
            build_item_names(with_attachment),
            ImapMessageFetchOptions::default(),
        )?;

        let Outcome { value, errors } = fetched_messages(output);
        let envelopes: Vec<Envelope> = value
            .into_iter()
            .rev()
            .map(|(seq, items)| envelope_from(seq.get(), items.into_inner()))
            .collect();

        let ids: Vec<&str> = envelopes.iter().map(|e| e.id.as_str()).collect();
        let errors = Outcome { value: (), errors }.sorted_by(&ids).errors;

        let listed = Outcome {
            value: EnvelopePage { envelopes, more },
            errors,
        };

        Ok((listed, total))
    }

    /// Searches a mailbox with the unified query.
    ///
    /// A UID `SORT`, or its client-side fallback, is constrained by the
    /// translated query, paginated, then fetched and reordered back into
    /// the order the sort gave.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Outcome<EnvelopePage>> {
        let select = self.select_mailbox(mailbox)?;
        if output_data("SELECT", select).exists.unwrap_or(0) == 0 {
            return Ok(Outcome::from(EnvelopePage::default()));
        }

        let search_criteria = search_keys(query.and_then(|q| q.filter.as_ref()))?;
        let sort_criteria = sort_criteria(query.and_then(|q| q.sort.as_deref()));
        let fallback = self.sort_fallback();

        let uids = self.sort(
            sort_criteria,
            search_criteria.clone(),
            ImapMessageSortOptions {
                uid: true,
                fallback,
            },
        )?;
        let mut uids = output_data("SORT", uids);
        let instants = root_instants(query.and_then(|q| q.filter.as_ref()));
        if !instants.is_empty() {
            let outside = self.outside_instants(&search_criteria, &instants)?;
            uids.retain(|uid| !outside.contains(uid));
        }
        let (page_uids, more) = paginate_uids(&uids, page, page_size);
        if page_uids.is_empty() {
            return Ok(Outcome::from(EnvelopePage::default()));
        }
        let uid_str = page_uids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let sequence_set: SequenceSet = uid_str
            .as_str()
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP UID set `{uid_str}`"))?;

        let output = self.fetch(
            sequence_set,
            build_item_names(with_attachment),
            ImapMessageFetchOptions {
                uid: true,
                ..Default::default()
            },
        )?;

        let Outcome { value, errors } = fetched_messages(output);
        let mut envelopes = reorder_envelopes(value, &page_uids);

        let ids = root_message_ids(query.and_then(|q| q.filter.as_ref()));
        envelopes.retain(|envelope| {
            let id = envelope.message_id.as_deref();
            ids.iter().all(|wanted| id == Some(*wanted))
        });

        let order: Vec<String> = page_uids.iter().map(u32::to_string).collect();
        let order: Vec<&str> = order.iter().map(String::as_str).collect();
        let errors = Outcome { value: (), errors }.sorted_by(&order).errors;

        Ok(Outcome {
            value: EnvelopePage { envelopes, more },
            errors,
        })
    }

    /// The UIDs under `criteria` that an instant does not hold: those of
    /// each [`InstantBand`] dated on its wrong side, or undated.
    fn outside_instants(
        &mut self,
        criteria: &Vec1<SearchKey<'static>>,
        instants: &[&SearchEmailsFilterQuery],
    ) -> Result<BTreeSet<NonZeroU32>> {
        let mut banded = BTreeSet::new();

        for instant in instants {
            let (SearchEmailsFilterQuery::AfterInstant(at)
            | SearchEmailsFilterQuery::BeforeInstant(at)) = instant
            else {
                continue;
            };
            let band = InstantBand::of(at);
            let mut keys = criteria.as_ref().to_vec();
            keys.push(SearchKey::SentSince(imap_date(band.first)?));
            keys.push(SearchKey::SentBefore(imap_date(band.end)?));
            let keys = Vec1::try_from(keys).map_err(|_| anyhow!("Empty IMAP search"))?;
            let found = self.search(keys, ImapMessageSearchOptions { uid: true })?;
            banded.extend(output_data("SEARCH", found));
        }

        let banded: Vec<NonZeroU32> = banded.into_iter().collect();
        let mut outside = BTreeSet::new();

        for chunk in banded.chunks(255) {
            let set = chunk
                .iter()
                .map(|uid| uid.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let set: SequenceSet = set
                .as_str()
                .try_into()
                .map_err(|_| anyhow!("Invalid IMAP UID set `{set}`"))?;
            let names = vec![MessageDataItemName::Uid, MessageDataItemName::Envelope];
            let output = self.fetch(
                set,
                MacroOrMessageDataItemNames::MessageDataItemNames(names),
                ImapMessageFetchOptions {
                    uid: true,
                    ..Default::default()
                },
            )?;

            for (seq, items) in output_data("FETCH", output) {
                let envelope = envelope_from(seq.get(), items.into_inner());
                let Some(uid) = envelope.id.parse().ok() else {
                    continue;
                };
                let holds = instants
                    .iter()
                    .all(|instant| match (instant, envelope.date) {
                        (SearchEmailsFilterQuery::AfterInstant(at), Some(date)) => date > *at,
                        (SearchEmailsFilterQuery::BeforeInstant(at), Some(date)) => date < *at,
                        _ => false,
                    });
                if !holds {
                    outside.insert(uid);
                }
            }
        }

        Ok(outside)
    }

    /// Adds, sets or removes flags on a UID set, returning how many of
    /// the UIDs exist in the mailbox.
    ///
    /// `UID STORE` answers `OK` for UIDs matching nothing (RFC 3501 §6.4.8)
    /// and may skip unchanged echoes, so too few `FETCH` echoes are settled
    /// by a `UID SEARCH`; a UID found by neither is an item error.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        let sequence_set = parse_uids(ids)?;
        let imap_flags: Vec<ImapFlag<'static>> = flags.iter().map(flag_from).collect();
        let kind = match op {
            FlagOp::Add => StoreType::Add,
            FlagOp::Set => StoreType::Replace,
            FlagOp::Remove => StoreType::Remove,
        };

        let selected = self.select_mailbox(mailbox)?;
        let selected = output_data("SELECT", selected);

        if let Some(permanent) = &selected.permanent_flags {
            refuse_impermanent(mailbox, flags, permanent)?;
        }

        let echoes = self.store(
            sequence_set.clone(),
            kind,
            imap_flags,
            ImapMessageStoreOptions { uid: true },
        )?;

        let present = self.stored_uids(echoes, sequence_set, ids.len())?;
        Ok(found_uids(ids, &present))
    }

    /// Fetches one message's raw RFC 5322 bytes and flags, marking it seen
    /// only when asked.
    pub fn get_message(
        &mut self,
        mailbox: &str,
        id: &str,
        seen: bool,
    ) -> Result<(Vec<u8>, BTreeSet<Flag>)> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        let sequence_set = parse_uids(&[id])?;

        self.select_mailbox(mailbox)?;

        let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
            MessageDataItemName::Flags,
            MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: !seen,
            },
        ]);
        let output = self.fetch(
            sequence_set,
            item_names,
            ImapMessageFetchOptions {
                uid: true,
                ..Default::default()
            },
        )?;
        let unreadable = !output.skipped.is_empty() || !output.partial.is_empty();
        let data = output_data("FETCH", output);
        let found = !data.is_empty();

        let mut body = None;
        let mut flags = BTreeSet::new();

        for item in data.into_values().flat_map(|items| items.into_inner()) {
            match item {
                MessageDataItem::BodyExt { data, .. } if body.is_none() => {
                    body = data.0.map(|d| d.as_ref().to_vec());
                }
                MessageDataItem::Flags(fs) => {
                    flags = fs.into_iter().filter_map(flag_from_fetch).collect();
                }
                _ => {}
            }
        }

        if seen {
            flags.insert(Flag::from_iana(IanaFlag::Seen));
        }

        match body {
            Some(body) => Ok((body, flags)),
            None if unreadable => {
                let message = format!(
                    "Cannot read message `{id}` of mailbox `{mailbox}`: its FETCH response does not decode"
                );
                Err(CodedError::new(ErrorCode::MessageUnreadable, message).into())
            }
            None if !found => Err(message_not_found(format!(
                "Message `{id}` not found in mailbox `{mailbox}`"
            ))),
            None => Err(anyhow!("FETCH returned no body for the requested message")),
        }
    }

    /// Appends a message and returns its UID, from the UIDPLUS
    /// `APPENDUID` or, failing that, a UID `SEARCH` on its `Message-ID`.
    ///
    /// RFC 4315 §3 makes `APPENDUID` a SHOULD, so an acknowledged append
    /// whose UID cannot be recovered is still a success, with no UID.
    pub fn add_message(
        &mut self,
        mailbox: &str,
        flags: &[Flag],
        raw: Vec<u8>,
    ) -> Result<Option<String>> {
        let mbox = parse_mailbox(mailbox)?;
        let imap_flags: Vec<ImapFlag<'static>> = flags.iter().map(flag_from).collect();

        let appended = self.append(
            mbox,
            &raw,
            ImapMessageAppendOptions {
                flags: imap_flags,
                date: None,
                non_sync: false,
            },
        );
        let (_, appenduid) = appended.map_err(|err| self.explain(mailbox, err, None))?;

        if let Some((_, uid)) = appenduid {
            return Ok(Some(uid.to_string()));
        }

        let message_id = header::parse_headers(&raw)
            .root
            .header::<MESSAGE_ID>()
            .and_then(|ids| ids.into_iter().next())
            .filter(|id| !id.is_empty());
        let Some(message_id) = message_id else {
            debug!("no APPENDUID and no Message-ID to search, appended UID unknown");
            return Ok(None);
        };

        self.select_mailbox(mailbox)?;

        let criteria = Vec1::from(message_id_key(&message_id)?);
        let uids = self.search(criteria, ImapMessageSearchOptions { uid: true })?;
        let uids = output_data("SEARCH", uids);

        let uid = uids.into_iter().max().map(|uid| uid.to_string());
        if uid.is_none() {
            debug!("no APPENDUID and no match for {message_id}, appended UID unknown");
        }

        Ok(uid)
    }

    /// Copies a UID set between two mailboxes, a UID not copied being an
    /// item error.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select_mailbox(from)?;
        let copied = self.copy(
            sequence_set.clone(),
            target,
            ImapMessageCopyOptions { uid: true },
        );
        let copy_uid = copied.map_err(|err| self.explain(to, err, None))?;

        let copied = self.copied_uids(copy_uid, sequence_set, ids.len())?;
        Ok(found_uids(ids, &copied))
    }

    /// Moves a UID set between two mailboxes.
    ///
    /// Without RFC 6851 MOVE: `UID COPY`, `\Deleted`, then with UIDPLUS a
    /// `UID EXPUNGE` of the UIDs really copied; without it they stay flagged,
    /// a plain `EXPUNGE` removing every other `\Deleted` message too.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select_mailbox(from)?;

        if same_mailbox(from, to) {
            let present = self.present_uids(sequence_set)?;
            return Ok(found_uids(ids, &present));
        }

        if self.supports_move() {
            let present = self.present_uids(sequence_set)?;
            if present.is_empty() {
                return Ok(found_uids(ids, &present));
            }
            let moved = self.r#move(
                uid_sequence_set(present.clone())?,
                target,
                ImapMessageMoveOptions { uid: true },
            );
            let copy_uid = moved.map_err(|err| self.explain(to, err, None))?;
            let mut outcome = found_uids(ids, &present);
            if let Some(copy_uid) = copy_uid
                && copy_uid.count() < present.len() as u64
            {
                outcome.value = copy_uid.count() as usize;
            }
            return Ok(outcome);
        }

        let copied = self.copy(
            sequence_set.clone(),
            target,
            ImapMessageCopyOptions { uid: true },
        );
        let copy_uid = copied.map_err(|err| self.explain(to, err, None))?;
        let copied = self.copied_uids(copy_uid, sequence_set, ids.len())?;
        let outcome = found_uids(ids, &copied);
        if copied.is_empty() {
            return Ok(outcome);
        }

        let to_remove = uid_sequence_set(copied)?;

        self.store(
            to_remove.clone(),
            StoreType::Add,
            vec![ImapFlag::Deleted],
            ImapMessageStoreOptions { uid: true },
        )?;

        if self.supports_uidplus() {
            self.uid_expunge(to_remove)?;
        } else {
            debug!(
                "server lacks UIDPLUS, skipping UID EXPUNGE; \
                 messages stay flagged \\Deleted in the source"
            );
        }

        Ok(outcome)
    }

    /// Creates the one level `name` at the top or under `parent`, returning
    /// its full name; a NAME holding the delimiter is refused.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        let delimiter = self.delimiter()?;
        if let Some(delimiter) = delimiter {
            validate_level(name, delimiter)?;
        }

        let path = match parent {
            None => name.to_string(),
            Some(parent) => {
                let Some(delimiter) = delimiter else {
                    bail!("IMAP mailbox `{parent}` holds no other: the server keeps no hierarchy");
                };
                let Some((parent, holds_others)) = self
                    .mailbox_rows()?
                    .into_iter()
                    .find(|(known, _)| same_name(known, parent))
                else {
                    return Err(mailbox_not_found(format!(
                        "IMAP mailbox `{parent}` not found"
                    )));
                };
                if !holds_others {
                    bail!("IMAP mailbox `{parent}` can hold no other (\\Noinferiors)");
                }
                child_path(&parent, delimiter, name)
            }
        };

        match self.create(parse_mailbox(&path)?) {
            Ok(()) => Ok(path),
            Err(err) => Err(self.create_refusal(&path, err)),
        }
    }

    /// A refused `CREATE`, `mailbox-duplicate` on `ALREADYEXISTS` (RFC 5530
    /// §3) or on an uncoded refusal of a mailbox the server lists.
    fn create_refusal(&mut self, path: &str, err: ImapClientError) -> anyhow::Error {
        let (exists, refused) = match err.status() {
            Some(status) => {
                let exists = matches!(
                    &status.code,
                    Some(Code::Other(code)) if code.inner().eq_ignore_ascii_case(b"ALREADYEXISTS")
                );
                (
                    exists,
                    status.kind == StatusKind::No && status.code.is_none(),
                )
            }
            None => return err.into(),
        };

        let listed = !exists
            && refused
            && self
                .mailbox_rows()
                .is_ok_and(|rows| rows.iter().any(|(known, _)| same_name(known, path)));

        match exists || listed {
            true => existing_mailbox(path, err),
            false => err.into(),
        }
    }

    /// Renames the last level of `mailbox` to `name`, returning its new full
    /// name; a NAME holding the delimiter is refused.
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<String> {
        let path = match self.delimiter()? {
            Some(delimiter) => {
                validate_level(name, delimiter)?;
                renamed_path(mailbox, delimiter, name)
            }
            None => name.to_string(),
        };

        if let Err(err) = self.rename(parse_mailbox(mailbox)?, parse_mailbox(&path)?) {
            return Err(self.explain(mailbox, err, None));
        }

        Ok(path)
    }

    /// Deletes `mailbox` and the messages it holds (RFC 3501 section
    /// 6.3.4).
    ///
    /// A server keeping inferior names refuses a mailbox holding others,
    /// which the error then names rather than the server's own text.
    pub fn delete_mailbox(&mut self, mailbox: &str) -> Result<()> {
        let Err(err) = self.delete(parse_mailbox(mailbox)?) else {
            return Ok(());
        };

        let delimiter = self.delimiter().ok().flatten();
        Err(self.explain(mailbox, err, delimiter))
    }

    /// Explains the server's refusal of a command addressing `mailbox`.
    ///
    /// `NONEXISTENT` (RFC 5530 §3), `TRYCREATE` (RFC 3501 §6.3.11) or a plain
    /// `NO` whose `LIST` finds nothing is `mailbox-not-found`; with
    /// `delimiter`, a `NO` on a mailbox holding others names them.
    fn explain(
        &mut self,
        mailbox: &str,
        err: ImapClientError,
        delimiter: Option<char>,
    ) -> anyhow::Error {
        let Some(status) = err.status().filter(|s| s.kind == StatusKind::No) else {
            return err.into();
        };
        let plain = status.code.is_none();
        if status.code.as_ref().is_some_and(names_no_mailbox) {
            return missing_mailbox(mailbox, err);
        }
        if !plain && delimiter.is_none() {
            return err.into();
        }

        let Ok(names) = self.mailbox_names() else {
            return err.into();
        };

        if plain && !names.iter().any(|known| same_name(known, mailbox)) {
            return missing_mailbox(mailbox, err);
        }

        let Some(delimiter) = delimiter else {
            return err.into();
        };

        let prefix = format!("{mailbox}{delimiter}");
        let children: Vec<String> = names
            .into_iter()
            .filter(|name| name.starts_with(&prefix))
            .collect();

        if children.is_empty() {
            return err.into();
        }

        let children = children.join("`, `");
        anyhow!("Cannot delete IMAP mailbox `{mailbox}`: it holds `{children}`; delete them first")
    }

    /// `SELECT`s `mailbox`, a refusal explained as [`Self::explain`] does.
    fn select_mailbox(&mut self, mailbox: &str) -> Result<ImapOutput<ImapMailboxSelectData>> {
        let mbox = parse_mailbox(mailbox)?;
        let selected = self.select(mbox, ImapMailboxSelectOptions::default());
        selected.map_err(|err| self.explain(mailbox, err, None))
    }

    /// The server's hierarchy delimiter, `None` for a flat namespace.
    ///
    /// Read from `LIST "" ""` (RFC 3501 section 6.3.8), so no mailbox name
    /// goes out as a pattern: one is not encoded in modified UTF-7 there,
    /// and a `*` or `%` in it would match others.
    fn delimiter(&mut self) -> Result<Option<char>> {
        let reference: ImapMailbox<'static> = ""
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list reference"))?;
        let pattern: ListMailbox<'static> = ""
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list pattern"))?;

        let rows = self.list(reference, pattern)?;
        let delimiter = output_data("LIST", rows)
            .into_iter()
            .next()
            .and_then(|(_, delimiter, _)| delimiter);
        Ok(delimiter.map(|delimiter| delimiter.inner()))
    }

    /// The full name of every mailbox of the server, the ones that cannot
    /// be selected included.
    fn mailbox_names(&mut self) -> Result<Vec<String>> {
        let rows = self.mailbox_rows()?;
        Ok(rows.into_iter().map(|(name, _)| name).collect())
    }

    /// Every mailbox name the server lists, with whether it can hold
    /// others (no `\Noinferiors`, RFC 3501 section 7.2.2).
    fn mailbox_rows(&mut self) -> Result<Vec<(String, bool)>> {
        let reference: ImapMailbox<'static> = ""
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list reference"))?;
        let pattern: ListMailbox<'static> = "*"
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list pattern"))?;

        let rows = output_data("LIST", self.list(reference, pattern)?)
            .into_iter()
            .map(|(mailbox, _, attrs)| {
                let name = match mailbox {
                    ImapMailbox::Inbox => String::from("INBOX"),
                    ImapMailbox::Other(other) => {
                        String::from_utf8_lossy(other.inner().as_ref()).into_owned()
                    }
                };
                (name, !attrs.contains(&FlagNameAttribute::Noinferiors))
            })
            .collect();

        Ok(rows)
    }

    /// Permanently deletes a UID set from the trash, returning how many it
    /// held and whether the mailbox removes or only flags, even if none.
    ///
    /// Only RFC 4315 UIDPLUS `UID EXPUNGE` leaves every other `\Deleted`
    /// message alone, so without it the UIDs are only flagged.
    pub fn delete_messages(
        &mut self,
        mailbox: &str,
        ids: &[&str],
    ) -> Result<Outcome<(usize, bool)>> {
        let sequence_set = parse_uids(ids)?;

        self.select_mailbox(mailbox)?;
        let echoes = self.store(
            sequence_set.clone(),
            StoreType::Add,
            vec![ImapFlag::Deleted],
            ImapMessageStoreOptions { uid: true },
        )?;

        let present = self.stored_uids(echoes, sequence_set.clone(), ids.len())?;
        let outcome = found_uids(ids, &present);

        let removes = self.supports_uidplus();
        if removes && outcome.value > 0 {
            self.uid_expunge(sequence_set)?;
        }

        Ok(outcome.map(|count| (count, removes)))
    }

    /// The UIDs a `UID STORE` found: its `echoes` when they number `asked`,
    /// else a `UID SEARCH`, a server being free to skip unchanged echoes.
    fn stored_uids(
        &mut self,
        echoes: ImapOutput<BTreeMap<NonZeroU32, Vec1<MessageDataItem<'static>>>>,
        requested: SequenceSet,
        asked: usize,
    ) -> Result<Vec<u32>> {
        let echoes = output_data("STORE", echoes);

        let mut uids: Vec<u32> = echoes
            .into_values()
            .flat_map(|items| items.into_inner())
            .filter_map(|item| match item {
                MessageDataItem::Uid(uid) => Some(uid.get()),
                _ => None,
            })
            .collect();

        if uids.len() < asked {
            uids.extend(self.present_uids(requested)?);
        }

        uids.sort_unstable();
        uids.dedup();
        Ok(uids)
    }

    /// The source UIDs a `UID COPY` of `requested` really copied, the
    /// source still selected.
    ///
    /// An RFC 4315 `COPYUID` answers alone (even with UIDVALIDITY 0, [#781]);
    /// else, or over `limit`, a `UID SEARCH` of the untouched source does.
    ///
    /// [#781]: https://github.com/pimalaya/himalaya/issues/781
    fn copied_uids(
        &mut self,
        copy_uid: Option<ImapCopyUid>,
        requested: SequenceSet,
        limit: usize,
    ) -> Result<Vec<u32>> {
        match copy_uid {
            Some(copy_uid) if copy_uid.count() <= limit as u64 => {
                Ok(copy_uid.pairs().map(|(source, _)| source.get()).collect())
            }
            _ => self.present_uids(requested),
        }
    }

    /// The UIDs of `requested` present in the selected mailbox, by a
    /// `UID SEARCH UID`, which leaves the absent ones out.
    fn present_uids(&mut self, requested: SequenceSet) -> Result<Vec<u32>> {
        let criteria = Vec1::from(SearchKey::Uid(requested));
        let uids = self.search(criteria, ImapMessageSearchOptions { uid: true })?;
        let uids = output_data("SEARCH", uids);
        let mut uids: Vec<u32> = uids.into_iter().map(NonZeroU32::get).collect();
        uids.sort_unstable();
        uids.dedup();
        Ok(uids)
    }
}

/// One `LIST` row: a mailbox, its delimiter and its attributes.
type ListRow = (
    ImapMailbox<'static>,
    Option<io_imap::types::core::QuotedChar>,
    Vec<FlagNameAttribute<'static>>,
);

/// Drops `\Noselect` containers (RFC 3501 section 6.3.8): they cannot hold
/// messages and would error out on any later unified-API op.
fn is_selectable(row: &ListRow) -> bool {
    !row.2.contains(&FlagNameAttribute::Noselect)
}

/// Converts one `LIST` row into the unified [`Mailbox`] shape.
///
/// The role is the reserved `INBOX`, else the first special-use attribute
/// the row carries (RFC 6154 §2 lets a plain `LIST` return them).
fn mailbox_from(row: ListRow) -> Mailbox {
    let role = match row.0 {
        ImapMailbox::Inbox => Some(MailboxRole::Inbox),
        ImapMailbox::Other(_) => {
            row.2
                .iter()
                .find_map(|attr| match MailboxRole::known(&attr.to_string())? {
                    MailboxRole::Subscribed => None,
                    role => Some(role),
                })
        }
    };
    let name = match row.0 {
        ImapMailbox::Inbox => "Inbox".to_string(),
        ImapMailbox::Other(other) => String::from_utf8_lossy(other.inner().as_ref()).into_owned(),
    };

    Mailbox {
        id: name.clone(),
        name,
        role,
        total: None,
        unread: None,
    }
}

/// Folds a `STATUS` response into the matching mailbox row.
fn apply_status(mailbox: &mut Mailbox, items: Vec<StatusDataItem>) {
    for item in items {
        match item {
            StatusDataItem::Messages(n) => mailbox.total = Some(u64::from(n)),
            StatusDataItem::Unseen(n) => mailbox.unread = Some(u64::from(n)),
            _ => {}
        }
    }
}

/// The `FETCH` item names a listing asks for, `BODYSTRUCTURE` included
/// only when the caller asks whether each message has an attachment.
fn build_item_names(with_attachment: bool) -> MacroOrMessageDataItemNames<'static> {
    let mut names = vec![
        MessageDataItemName::Uid,
        MessageDataItemName::Flags,
        MessageDataItemName::Envelope,
        MessageDataItemName::Rfc822Size,
    ];
    if with_attachment {
        names.push(MessageDataItemName::BodyStructure);
    }
    MacroOrMessageDataItemNames::MessageDataItemNames(names)
}

/// The sequence set of one page, `None` when the window is empty. Page 1
/// is the most recent one.
fn compute_window(exists: u32, page: Option<u32>, page_size: Option<u32>) -> Option<String> {
    if exists == 0 {
        return None;
    }
    let page = page.unwrap_or(1).max(1);
    let Some(size) = page_size else {
        return Some("1:*".to_string());
    };
    if size == 0 {
        return None;
    }
    let skip = (page - 1).saturating_mul(size);
    if skip >= exists {
        return None;
    }
    let end = exists - skip;
    let start = end.saturating_sub(size - 1).max(1);
    Some(format!("{start}:{end}"))
}

/// Folds one `FETCH` row into a unified [`Envelope`].
pub(crate) fn envelope_from(seq: u32, items: Vec<MessageDataItem<'static>>) -> Envelope {
    let mut id = String::new();
    let mut message_id: Option<String> = None;
    let mut in_reply_to = Vec::new();
    let mut flags = BTreeSet::new();
    let mut subject = None;
    let mut from = Vec::new();
    let mut to = Vec::new();
    let mut date: Option<DateTime<FixedOffset>> = None;
    let mut size: Option<u64> = None;
    let mut has_attachment: Option<bool> = None;

    for item in items {
        match item {
            MessageDataItem::Uid(uid) => id = uid.get().to_string(),
            MessageDataItem::Flags(fs) => {
                flags = fs.into_iter().filter_map(flag_from_fetch).collect();
            }
            MessageDataItem::Envelope(env) => {
                if let Some(s) = env.subject.into_option() {
                    subject = Some(header::decode_bytes(s.as_ref()));
                }
                if let Some(d) = env.date.into_option() {
                    date = parse_rfc2822_date(&bytes_to_string(d.as_ref()));
                }
                if let Some(m) = env.message_id.into_option() {
                    message_id = normalize_message_id(&bytes_to_string(m.as_ref()));
                }
                if let Some(m) = env.in_reply_to.into_option() {
                    in_reply_to = parse_message_ids(&bytes_to_string(m.as_ref()));
                }
                from = to_addresses(&env.from);
                to = to_addresses(&env.to);
            }
            MessageDataItem::Rfc822Size(n) => size = Some(u64::from(n)),
            MessageDataItem::BodyStructure(structure) => {
                has_attachment = Some(body_structure_has_attachment(&structure));
            }
            _ => {}
        }
    }

    if id.is_empty() {
        id = seq.to_string();
    }

    Envelope {
        id,
        message_id,
        in_reply_to,
        flags,
        subject,
        from,
        to,
        date,
        size,
        has_attachment,
    }
}

/// Converts a fetched flag into a unified [`Flag`], dropping the
/// non-flag fetch items.
fn flag_from_fetch(fetch: FlagFetch<'_>) -> Option<Flag> {
    let FlagFetch::Flag(flag) = fetch else {
        return None;
    };
    Some(Flag::from_raw(flag.to_string()))
}

/// Whether a body structure holds an attachment, see
/// [`part::marks_attachment`].
fn body_structure_has_attachment(structure: &BodyStructure<'_>) -> bool {
    part_has_attachment(structure, false)
}

/// Whether a part of a body structure, or one under it, marks the message.
fn part_has_attachment(structure: &BodyStructure<'_>, is_protocol_part: bool) -> bool {
    match structure {
        BodyStructure::Single {
            body,
            extension_data,
        } => {
            let mime = match &body.specific {
                SpecificFields::Basic { r#type, subtype } => {
                    part::media_type(r#type.as_ref(), subtype.as_ref())
                }
                SpecificFields::Message { .. } => "message/rfc822".to_string(),
                SpecificFields::Text { .. } => "text/plain".to_string(),
            };
            let disposition = extension_data
                .as_ref()
                .and_then(|ext| ext.tail.as_ref())
                .and_then(|disposition| disposition.disposition.as_ref())
                .map(|(kind, _)| kind.as_ref());
            part::marks_attachment(
                &mime,
                disposition,
                body.basic.id.0.is_some(),
                is_protocol_part,
            )
        }
        BodyStructure::Multi {
            bodies, subtype, ..
        } => bodies.as_ref().iter().enumerate().any(|(index, body)| {
            part_has_attachment(body, part::is_protocol_part(subtype.as_ref(), index + 1))
        }),
    }
}

/// Maps a unified [`Flag`] onto its IMAP wire counterpart.
///
/// An IANA flag becomes the matching system flag, and a custom keyword a
/// Keyword atom, sanitised when it is not atom-safe.
fn flag_from(flag: &Flag) -> ImapFlag<'static> {
    match flag.iana() {
        Some(IanaFlag::Seen) => ImapFlag::Seen,
        Some(IanaFlag::Answered) => ImapFlag::Answered,
        Some(IanaFlag::Flagged) => ImapFlag::Flagged,
        Some(IanaFlag::Draft) => ImapFlag::Draft,
        Some(IanaFlag::Deleted) => ImapFlag::Deleted,
        Some(_) => ImapFlag::keyword(
            Atom::try_from(String::from(flag.raw()))
                .expect("canonical IANA keyword is a valid IMAP atom"),
        ),
        None => match Atom::try_from(String::from(flag.raw())) {
            Ok(atom) => ImapFlag::keyword(atom),
            Err(_) => ImapFlag::keyword(
                Atom::try_from(sanitise_atom(flag.raw()))
                    .expect("sanitised atom contains only atom-safe ASCII"),
            ),
        },
    }
}

/// Refuses a keyword `PERMANENTFLAGS` lists neither by name nor as `\*`
/// (RFC 3501 §7.1), so a flag never seems set and then vanishes.
///
/// The system flags are left to the server.
fn refuse_impermanent(mailbox: &str, flags: &[Flag], permanent: &[FlagPerm<'_>]) -> Result<()> {
    if permanent.contains(&FlagPerm::Asterisk) {
        return Ok(());
    }

    for flag in flags {
        let ImapFlag::Keyword(keyword) = flag_from(flag) else {
            continue;
        };

        let kept = permanent.iter().any(|perm| match perm {
            FlagPerm::Flag(ImapFlag::Keyword(other)) => {
                other.as_ref().eq_ignore_ascii_case(keyword.as_ref())
            }
            _ => false,
        });

        if !kept {
            let name = match flag.capability() {
                Some(capability) => format!("`{capability}` (`{keyword}`)"),
                None => format!("`{keyword}`"),
            };
            return Err(capability::missing(format!(
                "Mailbox `{mailbox}` cannot keep {name}: its PERMANENTFLAGS list neither it nor `\\*`"
            )));
        }
    }

    Ok(())
}

/// Replaces every non-atom-safe byte with `_`, so a keyword carrying
/// spaces, controls or brackets survives a `STORE`.
fn sanitise_atom(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii()
                && !c.is_control()
                && !matches!(
                    c,
                    ' ' | '(' | ')' | '{' | '%' | '*' | '"' | '\\' | ']' | '\x7f'
                )
            {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Whether `a` and `b` name one mailbox, `INBOX` in any case (RFC 3501
/// section 5.1).
fn same_mailbox(a: &str, b: &str) -> bool {
    a == b || (a.eq_ignore_ascii_case("INBOX") && b.eq_ignore_ascii_case("INBOX"))
}

/// Parses a unified mailbox name into an IMAP Mailbox token.
fn parse_mailbox(name: &str) -> Result<ImapMailbox<'static>> {
    String::from(name)
        .try_into()
        .map_err(|_| anyhow!("Invalid IMAP mailbox `{name}`"))
}

/// Whether two full mailbox names name the same mailbox, `INBOX` being
/// case-insensitive (RFC 3501 section 5.1).
fn same_name(left: &str, right: &str) -> bool {
    left == right || (left.eq_ignore_ascii_case("INBOX") && right.eq_ignore_ascii_case("INBOX"))
}

/// Whether `code` says the mailbox does not exist: `NONEXISTENT` (RFC 5530
/// §3) or `TRYCREATE` (RFC 3501 §6.3.11, RFC 6851 §3.3).
fn names_no_mailbox(code: &Code<'_>) -> bool {
    match code {
        Code::TryCreate => true,
        Code::Other(code) => code.inner().eq_ignore_ascii_case(b"NONEXISTENT"),
        _ => false,
    }
}

/// The failure of a command addressing `mailbox`, which the server does
/// not hold, under `mailbox-not-found`, its refusal `err` kept.
fn missing_mailbox(mailbox: &str, err: ImapClientError) -> anyhow::Error {
    unknown_mailbox(format!("IMAP mailbox `{mailbox}` not found"), err)
}

/// How many of `ids` the server found among the `found` UIDs, beside the
/// error of each one it did not.
fn found_uids(ids: &[&str], found: &[u32]) -> Outcome<usize> {
    let mut count = 0;
    let mut errors = Vec::new();

    for id in ids {
        match id.parse() {
            Ok(uid) if found.contains(&uid) => count += 1,
            _ => errors.push(missing_message(id)),
        }
    }

    Outcome {
        value: count,
        errors,
    }
}

/// The item error of a mailbox the client could not read, `None` for one
/// whose `LIST` line did not decode.
fn unreadable_mailbox(id: Option<&str>, error: &str) -> ItemError {
    ItemError {
        id: id.map(String::from),
        code: Some(ErrorCode::MailboxUnreadable),
        error: error.to_owned(),
        sources: Vec::new(),
    }
}

/// The messages a `FETCH` answered, beside an item error for each line
/// that did not decode whole.
///
/// A line decoded in part keeps its message with the items that did,
/// unless its UID is among the failed ones; a line skipped whole names
/// no message.
fn fetched_messages(output: ImapOutput<FetchData>) -> Outcome<FetchData> {
    let partial = output.partial.clone();
    let skipped = output.skipped.len();
    let mut data = output_data("FETCH", output);
    let mut errors = Vec::new();

    for line in partial {
        let uid = line.uid.map(NonZeroU32::get).or_else(|| {
            let items = data.get(&line.seq)?;
            items.as_ref().iter().find_map(|item| match item {
                MessageDataItem::Uid(uid) => Some(uid.get()),
                _ => None,
            })
        });

        if uid.is_none() {
            data.remove(&line.seq);
        }

        let names: Vec<&str> = line.failed.iter().map(|item| item.name.as_str()).collect();
        errors.push(ItemError {
            id: uid.map(|uid| uid.to_string()),
            code: Some(ErrorCode::MessageUnreadable),
            error: format!("Cannot decode {} of the FETCH response", names.join(", ")),
            sources: Vec::new(),
        });
    }

    for _ in 0..skipped {
        errors.push(ItemError {
            id: None,
            code: Some(ErrorCode::MessageUnreadable),
            error: String::from("Cannot decode a line of the FETCH response"),
            sources: Vec::new(),
        });
    }

    Outcome {
        value: data,
        errors,
    }
}

/// The messages of a `FETCH` answer, by sequence number.
type FetchData = BTreeMap<NonZeroU32, Vec1<MessageDataItem<'static>>>;

/// Parses stringified UIDs into an IMAP [`SequenceSet`].
fn parse_uids(ids: &[&str]) -> Result<SequenceSet> {
    let uids = ids
        .iter()
        .map(|s| s.parse().map_err(|_| anyhow!("Invalid message UID `{s}`")))
        .collect::<Result<_>>()?;

    uid_sequence_set(uids)
}

/// Builds an IMAP [`SequenceSet`] from numeric UIDs.
fn uid_sequence_set(uids: Vec<u32>) -> Result<SequenceSet> {
    if uids.is_empty() {
        bail!("Empty UID set");
    }

    let uids: Vec<NonZeroU32> = uids
        .into_iter()
        .map(|uid| NonZeroU32::new(uid).ok_or_else(|| anyhow!("Invalid message UID `{uid}`")))
        .collect::<Result<_>>()?;

    SequenceSet::try_from(uids).map_err(|_| anyhow!("Invalid UID set"))
}

/// Renders bytes as UTF-8, falling back to latin-1 when they are not.
fn bytes_to_string(bytes: &[u8]) -> String {
    from_utf8(bytes).map(str::to_string).unwrap_or_else(|_| {
        let mut out = String::with_capacity(bytes.len());
        for b in bytes {
            out.push(*b as char);
        }
        out
    })
}

/// The `SEARCH` keys of a unified filter, `ALL` when there is none, a
/// root `and` sent as the list of its keys.
fn search_keys(filter: Option<&SearchEmailsFilterQuery>) -> Result<Vec1<SearchKey<'static>>> {
    match filter {
        None => Ok(Vec1::from(SearchKey::All)),
        Some(filter) => match convert_filter(filter)? {
            SearchKey::And(keys) => Ok(keys),
            key => Ok(Vec1::from(key)),
        },
    }
}

/// The `SORT` criteria of a unified sort, `REVERSE DATE` when there is
/// none.
fn sort_criteria(sort: Option<&[SearchEmailsSorter]>) -> Vec1<SortCriterion> {
    let criteria: Vec<SortCriterion> = match sort {
        Some(chain) if !chain.is_empty() => chain.iter().map(convert_sorter).collect(),
        _ => vec![SortCriterion {
            reverse: true,
            key: SortKey::Date,
        }],
    };

    Vec1::try_from(criteria).expect("non-empty by construction")
}

/// Translates a unified filter into an IMAP search key, its negations
/// pushed down to the single keys.
///
/// `NOT` then only wraps a single key: iCloud reads `NOT OR a b` as all,
/// `NOT NOT a` as `NOT a`, and refuses `NOT (a b)`.
fn convert_filter(filter: &SearchEmailsFilterQuery) -> Result<SearchKey<'static>> {
    convert_filter_with(filter, false, true)
}

/// Translates `filter` or its negation; an instant is searched by its
/// [`InstantBand`] when `root` (on the root `and` chain), else refused.
fn convert_filter_with(
    filter: &SearchEmailsFilterQuery,
    negated: bool,
    root: bool,
) -> Result<SearchKey<'static>> {
    use SearchEmailsFilterQuery as Q;

    let key = match filter {
        Q::Not(inner) => return convert_filter_with(inner, !negated, false),
        Q::And(left, right) if !negated => {
            return Ok(and_key(
                convert_filter_with(left, false, root)?,
                convert_filter_with(right, false, root)?,
            ));
        }
        Q::And(left, right) => {
            return Ok(SearchKey::Or(
                Box::new(convert_filter_with(left, true, false)?),
                Box::new(convert_filter_with(right, true, false)?),
            ));
        }
        Q::Or(left, right) if !negated => {
            return Ok(SearchKey::Or(
                Box::new(convert_filter_with(left, false, false)?),
                Box::new(convert_filter_with(right, false, false)?),
            ));
        }
        Q::Or(left, right) => {
            return Ok(and_key(
                convert_filter_with(left, true, false)?,
                convert_filter_with(right, true, false)?,
            ));
        }
        Q::AfterInstant(_) | Q::BeforeInstant(_) if !root => {
            return Err(unsupported(
                filter,
                "IMAP searches days alone, so an instant holds on the chain of `and` from the root alone",
            ));
        }
        Q::AfterInstant(instant) => {
            SearchKey::SentSince(imap_date(InstantBand::of(instant).first)?)
        }
        Q::BeforeInstant(instant) => {
            SearchKey::SentBefore(imap_date(InstantBand::of(instant).end)?)
        }
        Q::Address(pattern) => return address_key(pattern, negated),

        Q::Date(date) => SearchKey::SentOn(imap_date(*date)?),

        Q::AfterDate(date) => {
            let bumped = date.succ_opt().unwrap_or(*date);
            SearchKey::SentSince(imap_date(bumped)?)
        }

        Q::BeforeDate(date) => SearchKey::SentBefore(imap_date(*date)?),

        Q::From(pattern) => SearchKey::From(astring(pattern)?),
        Q::To(pattern) => SearchKey::To(astring(pattern)?),
        Q::Cc(pattern) => SearchKey::Cc(astring(pattern)?),
        Q::Bcc(pattern) => SearchKey::Bcc(astring(pattern)?),
        Q::Subject(pattern) => SearchKey::Subject(astring(pattern)?),
        Q::Body(pattern) => SearchKey::Body(astring(pattern)?),
        Q::Text(pattern) => SearchKey::Text(astring(pattern)?),
        Q::MessageId(id) => message_id_key(id)?,
        Q::Attachment => {
            return Err(unsupported(
                filter,
                "IMAP SEARCH has no key for attachments",
            ));
        }
        Q::Flag(flag) => flag_key(flag)?,
    };

    if negated {
        Ok(SearchKey::Not(Box::new(key)))
    } else {
        Ok(key)
    }
}

/// The days whose messages may be sent on either side of an instant, a
/// `Date:` day being in its own offset (UTC-12:00 to UTC+14:00).
///
/// It runs from the day before the instant's UTC day to, excluded, the
/// second day after; a message dated outside lies on that side.
struct InstantBand {
    /// The first day of the band.
    first: chrono::NaiveDate,
    /// The day after its last.
    end: chrono::NaiveDate,
}

impl InstantBand {
    fn of(instant: &DateTime<FixedOffset>) -> Self {
        let day = instant.naive_utc().date();
        Self {
            first: day.pred_opt().unwrap_or(day),
            end: day.succ_opt().and_then(|day| day.succ_opt()).unwrap_or(day),
        }
    }
}

/// The instant clauses on the chain of `and` from the root, which a
/// message of their band is checked against once its date is fetched.
fn root_instants(filter: Option<&SearchEmailsFilterQuery>) -> Vec<&SearchEmailsFilterQuery> {
    match filter {
        Some(SearchEmailsFilterQuery::And(left, right)) => {
            let mut instants = root_instants(Some(left));
            instants.extend(root_instants(Some(right)));
            instants
        }
        Some(
            instant @ (SearchEmailsFilterQuery::AfterInstant(_)
            | SearchEmailsFilterQuery::BeforeInstant(_)),
        ) => vec![instant],
        _ => Vec::new(),
    }
}

/// The ids of the `message-id` clauses on the chain of `and` from the root,
/// which a fetched envelope is checked against.
fn root_message_ids(filter: Option<&SearchEmailsFilterQuery>) -> Vec<&str> {
    match filter {
        Some(SearchEmailsFilterQuery::And(left, right)) => {
            let mut ids = root_message_ids(Some(left));
            ids.extend(root_message_ids(Some(right)));
            ids
        }
        Some(SearchEmailsFilterQuery::MessageId(id)) => vec![id.as_str()],
        _ => Vec::new(),
    }
}

/// Joins two keys under `AND`, a side that is an `AND` already giving
/// its keys rather than a nested list.
fn and_key(left: SearchKey<'static>, right: SearchKey<'static>) -> SearchKey<'static> {
    let mut keys = Vec::new();

    for key in [left, right] {
        match key {
            SearchKey::And(inner) => keys.extend(inner.into_inner()),
            key => keys.push(key),
        }
    }

    SearchKey::And(Vec1::try_from(keys).expect("non-empty by construction"))
}

/// The `address` clause: an `OR` of `FROM`, `TO`, `CC` and `BCC`, or
/// negated, the four negated keys joined by `AND`.
fn address_key(pattern: &str, negated: bool) -> Result<SearchKey<'static>> {
    let keys = [
        SearchKey::From(astring(pattern)?),
        SearchKey::To(astring(pattern)?),
        SearchKey::Cc(astring(pattern)?),
        SearchKey::Bcc(astring(pattern)?),
    ];

    if negated {
        let keys = keys.map(|key| SearchKey::Not(Box::new(key)));
        let keys = Vec1::try_from(Vec::from(keys)).expect("non-empty by construction");
        return Ok(SearchKey::And(keys));
    }

    let [from, to, cc, bcc] = keys;
    let cc_or_bcc = SearchKey::Or(Box::new(cc), Box::new(bcc));
    let to_or_rest = SearchKey::Or(Box::new(to), Box::new(cc_or_bcc));
    Ok(SearchKey::Or(Box::new(from), Box::new(to_or_rest)))
}

/// The key of a `flag` clause, a keyword of the IANA registry going by
/// its canonical spelling: `flag junk` searches `$Junk`.
fn flag_key(flag: &Flag) -> Result<SearchKey<'static>> {
    let raw = match flag.iana() {
        Some(IanaFlag::Seen) => return Ok(SearchKey::Seen),
        Some(IanaFlag::Answered) => return Ok(SearchKey::Answered),
        Some(IanaFlag::Flagged) => return Ok(SearchKey::Flagged),
        Some(IanaFlag::Draft) => return Ok(SearchKey::Draft),
        Some(IanaFlag::Deleted) => return Ok(SearchKey::Deleted),
        Some(iana) => Flag::from_iana(iana).raw().to_owned(),
        None => flag.raw().to_owned(),
    };

    let keyword =
        Atom::try_from(raw.clone()).map_err(|_| anyhow!("Invalid IMAP keyword `{raw}`"))?;
    Ok(SearchKey::Keyword(keyword))
}

/// Translates one unified sorter into an IMAP sort criterion.
fn convert_sorter(sorter: &SearchEmailsSorter) -> SortCriterion {
    let SearchEmailsSorter(kind, order) = sorter;

    let key = match kind {
        SearchEmailsSorterKind::Date => SortKey::Date,
        SearchEmailsSorterKind::From => SortKey::From,
        SearchEmailsSorterKind::To => SortKey::To,
        SearchEmailsSorterKind::Subject => SortKey::Subject,
    };

    SortCriterion {
        reverse: matches!(order, SearchEmailsSorterOrder::Descending),
        key,
    }
}

/// Wraps a search pattern in an IMAP astring.
fn astring(pattern: &str) -> Result<AString<'static>> {
    AString::try_from(String::from(pattern))
        .map_err(|_| anyhow!("Invalid IMAP search pattern `{pattern}`"))
}

/// The `HEADER Message-ID` key of a bare id, sent in angle brackets.
///
/// RFC 3501 matches a substring, so the bracketed id matches where the
/// bare one does, and some servers (iCloud) match the whole value alone.
fn message_id_key(id: &str) -> Result<SearchKey<'static>> {
    Ok(SearchKey::Header(
        astring("Message-ID")?,
        astring(&format!("<{id}>"))?,
    ))
}

/// Converts a date into the IMAP wire date a search key takes.
fn imap_date(date: chrono::NaiveDate) -> Result<ImapNaiveDate> {
    ImapNaiveDate::try_from(date).map_err(|_| anyhow!("Invalid IMAP date `{date}`"))
}

/// Slices one page out of the sorted UIDs, keeping their order, beside
/// whether a page follows it.
///
/// A UID listed twice is kept at its first place: Fastmail can answer a
/// `SORT` under `OR` and `NOT` with one UID twice.
fn paginate_uids(
    uids: &[NonZeroU32],
    page: Option<u32>,
    page_size: Option<u32>,
) -> (Vec<u32>, bool) {
    let mut seen = BTreeSet::new();
    let uids: Vec<u32> = uids
        .iter()
        .map(|uid| uid.get())
        .filter(|uid| seen.insert(*uid))
        .collect();
    let total = uids.len();
    let more = page_follows(total as u64, page, page_size);
    let size = page_size.map(|n| n as usize);
    let start = ((page.unwrap_or(1).max(1) - 1) as usize).saturating_mul(size.unwrap_or(0));

    if start >= total {
        return (Vec::new(), more);
    }

    let end = match size {
        Some(n) => start.saturating_add(n).min(total),
        None => total,
    };

    (uids[start..end].to_vec(), more)
}

/// Reorders a `FETCH` response into the requested UID order, dropping the
/// UIDs the server skipped.
fn reorder_envelopes(
    data: BTreeMap<NonZeroU32, Vec1<MessageDataItem<'static>>>,
    order: &[u32],
) -> Vec<Envelope> {
    let by_uid: BTreeMap<u32, Envelope> = data
        .into_iter()
        .map(|(seq, items)| {
            let items = items.into_inner();
            let uid = items.iter().find_map(|item| match item {
                MessageDataItem::Uid(u) => Some(u.get()),
                _ => None,
            });
            let envelope = envelope_from(seq.get(), items);
            (uid.unwrap_or(seq.get()), envelope)
        })
        .collect();

    order
        .iter()
        .filter_map(|u| by_uid.get(u).cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use chrono::NaiveDate;
    use pimalaya_cli::error::code_of;

    use io_imap::{
        codec::{CommandCodec, ResponseCodec, decode::Decoder, encode::Encoder},
        types::{
            command::{Command, CommandBody},
            core::Atom,
            fetch::MessageDataItem,
            flag::{Flag as ImapFlag, FlagPerm},
            response::{Data, Response},
        },
    };

    use super::{
        SearchKey, body_structure_has_attachment, convert_filter, envelope_from, paginate_uids,
        refuse_impermanent, root_message_ids, search_keys,
    };
    use crate::{
        error::ErrorCode,
        unified::email::{
            flag::{Flag, IanaFlag},
            search::{filter::query::SearchEmailsFilterQuery, query::SearchEmailsQuery},
        },
    };

    #[test]
    fn a_keyword_the_mailbox_keeps_for_the_session_only_is_refused() {
        let junk = [Flag::from_iana(IanaFlag::Junk)];
        let seen = [Flag::from_iana(IanaFlag::Seen)];
        let listed = FlagPerm::Flag(ImapFlag::keyword(Atom::try_from("$junk").unwrap()));

        let refused = refuse_impermanent("INBOX", &junk, &[FlagPerm::Flag(ImapFlag::Seen)]);
        assert_eq!(
            code_of::<ErrorCode>(&refused.unwrap_err()),
            Some(ErrorCode::CapabilityMissing)
        );

        assert!(refuse_impermanent("INBOX", &junk, &[listed]).is_ok());
        assert!(refuse_impermanent("INBOX", &junk, &[FlagPerm::Asterisk]).is_ok());
        assert!(refuse_impermanent("INBOX", &seen, &[]).is_ok());
    }

    #[test]
    fn copies_addresses_and_before_become_search_keys() {
        let day = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let before = convert_filter(&SearchEmailsFilterQuery::BeforeDate(day)).unwrap();
        let address = convert_filter(&SearchEmailsFilterQuery::Address("eve".into())).unwrap();

        assert!(matches!(before, SearchKey::SentBefore(_)));
        assert!(matches!(
            convert_filter(&SearchEmailsFilterQuery::Cc("c".into())).unwrap(),
            SearchKey::Cc(_)
        ));
        assert!(matches!(address, SearchKey::Or(..)));
    }

    /// The `UID SEARCH` line a query's filter is sent as.
    fn search_line(query: &str) -> String {
        let query: SearchEmailsQuery = query.parse().unwrap();
        let criteria = search_keys(query.filter.as_ref()).unwrap();
        let body = CommandBody::Search {
            charset: None,
            criteria,
            uid: true,
        };
        let command = Command::new("A", body).unwrap();
        let bytes = CommandCodec::default().encode(&command).dump();
        String::from_utf8(bytes).unwrap().trim_end().to_owned()
    }

    #[test]
    fn negations_reach_single_keys_only() {
        assert_eq!(
            search_line("not (from:alice or from:bob)"),
            "A UID SEARCH NOT FROM alice NOT FROM bob",
        );
        assert_eq!(search_line("not not from:alice"), "A UID SEARCH FROM alice");
        assert_eq!(
            search_line("not (from:alice is:flagged)"),
            "A UID SEARCH OR NOT FROM alice NOT FLAGGED",
        );
        assert_eq!(
            search_line("not with:eve"),
            "A UID SEARCH NOT FROM eve NOT TO eve NOT CC eve NOT BCC eve",
        );
        assert_eq!(
            search_line("with:eve"),
            "A UID SEARCH OR FROM eve OR TO eve OR CC eve BCC eve",
        );
        assert_eq!(
            search_line("from:a not (subject:b or not (to:c cc:d))"),
            "A UID SEARCH FROM a NOT SUBJECT b TO c CC d",
        );
    }

    #[test]
    fn a_message_id_is_a_header_key_rechecked_on_the_root_chain() {
        assert_eq!(
            search_line("mid:<a@b.org> from:c"),
            "A UID SEARCH HEADER Message-ID <a@b.org> FROM c",
        );

        let query: SearchEmailsQuery = "mid:a@b.org (mid:c@d.org or from:e)".parse().unwrap();
        assert_eq!(root_message_ids(query.filter.as_ref()), ["a@b.org"]);
    }

    /// An instant on the root `and` chain is searched by its band, refused
    /// under `or` or `not`, where no check of the fetched dates holds.
    #[test]
    fn an_instant_is_searched_by_its_band_on_the_root_chain_alone() {
        assert_eq!(
            search_line("date:2026-11-14T10:00:01+01:00.. from:c"),
            "A UID SEARCH SENTSINCE \"13-Nov-2026\" FROM c",
        );
        assert_eq!(
            search_line("date:..2026-11-14T09:59:59+01:00"),
            "A UID SEARCH SENTBEFORE \"16-Nov-2026\"",
        );

        for query in [
            "date:2026-11-14T10:00:01Z.. or from:c",
            "not date:..2026-11-14T09:59:59Z",
        ] {
            let query: SearchEmailsQuery = query.parse().unwrap();
            let err = search_keys(query.filter.as_ref()).unwrap_err();
            let code = code_of::<crate::error::ErrorCode>(&err);
            assert_eq!(code, Some(crate::error::ErrorCode::SearchUnsupported));
        }
    }

    /// The messages of an instant's band are kept by their fetched dates:
    /// `after` 10:00 keeps those sent at 11:00 and 12:00, not 09:00.
    #[test]
    fn an_instant_keeps_the_messages_on_its_side() {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            thread,
        };

        use crate::{config::imap::ImapConfig, imap::client::ImapClient};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let envelope = |uid: u32, hour: u32| {
            format!(
                "* {uid} FETCH (UID {uid} FLAGS () RFC822.SIZE 1 ENVELOPE \
                 (\"Sat, 14 Nov 2026 {hour:02}:00:00 +0100\" \"m{uid}\" NIL NIL NIL NIL NIL NIL NIL NIL))\r\n"
            )
        };
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut out = stream;
            let mut commands = Vec::new();
            write!(out, "* PREAUTH [CAPABILITY IMAP4rev1 SORT] ready\r\n").unwrap();

            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 {
                let (tag, command) = line.trim_end().split_once(' ').unwrap();
                let (tag, command) = (tag.to_string(), command.to_string());
                line.clear();
                let data = if command.starts_with("SELECT") {
                    String::from("* 3 EXISTS\r\n")
                } else if command.starts_with("UID SORT") {
                    String::from("* SORT 3 2 1\r\n")
                } else if command.starts_with("UID SEARCH") {
                    String::from("* SEARCH 1 2 3\r\n")
                } else if command.starts_with("UID FETCH") {
                    let set = command.split(' ').nth(2).unwrap();
                    set.split(',')
                        .flat_map(|part| match part.split_once(':') {
                            Some((first, last)) => first.parse().unwrap()..=last.parse().unwrap(),
                            None => part.parse().unwrap()..=part.parse().unwrap(),
                        })
                        .map(|uid: u32| envelope(uid, [9, 11, 12][uid as usize - 1]))
                        .collect()
                } else {
                    String::new()
                };
                commands.push(command);
                write!(out, "{data}{tag} OK done\r\n").unwrap();
            }
            commands
        });

        let config: ImapConfig = toml::from_str(&format!("server = \"imap://{addr}\"")).unwrap();
        let mut secrets = pimalaya_config::secret::SecretResolver::new();
        let mut client = ImapClient::new(config, &mut secrets).unwrap();
        let query: SearchEmailsQuery = "date:2026-11-14T10:00:01+01:00..".parse().unwrap();
        let page = client
            .search_envelopes("INBOX", Some(&query), None, None, false)
            .unwrap();
        drop(client);
        let commands = server.join().unwrap();

        let ids: Vec<&str> = page.value.envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["3", "2"], "{commands:?}");
    }

    #[test]
    fn a_uid_the_server_sorts_twice_is_listed_once() {
        let uids = [1, 1, 7, 3, 1].map(|uid| NonZeroU32::new(uid).unwrap());

        assert_eq!(paginate_uids(&uids, Some(1), Some(2)), (vec![1, 7], true));
        assert_eq!(paginate_uids(&uids, Some(2), Some(2)), (vec![3], false));
        assert_eq!(
            paginate_uids(&uids, Some(1), Some(3)),
            (vec![1, 7, 3], false)
        );
    }

    #[test]
    fn a_registry_keyword_is_searched_by_its_canonical_spelling() {
        assert_eq!(
            search_line("tag:important"),
            "A UID SEARCH KEYWORD $Important"
        );
        assert_eq!(search_line("tag:junk"), "A UID SEARCH KEYWORD $Junk");
        assert_eq!(
            search_line("tag:Forwarded"),
            "A UID SEARCH KEYWORD $Forwarded"
        );
        assert_eq!(search_line("tag:custom"), "A UID SEARCH KEYWORD custom");
    }

    #[test]
    fn an_attachment_clause_is_refused_with_its_code() {
        let err = convert_filter(&SearchEmailsFilterQuery::Attachment).unwrap_err();

        assert_eq!(
            code_of::<ErrorCode>(&err),
            Some(ErrorCode::SearchUnsupported)
        );
        assert!(err.to_string().contains("`has:attachment`"), "{err}");
    }

    #[test]
    fn an_envelope_reads_its_header_strings_as_message_read_does() {
        const LINE: &[u8] = b"* 1 FETCH (UID 7 ENVELOPE (NIL \
            \"=?UTF-8?Q?=F0=9F?= =?UTF-8?Q?=98=82?=\" \
            (({24}\r\n\x93Ann\x94 =?UTF-8?Q?=C3=A9?= NIL \"a\" \"example.org\")) \
            NIL NIL NIL NIL NIL NIL NIL))\r\n";

        let (_, response) = ResponseCodec::new().decode(LINE).unwrap();
        let Response::Data(Data::Fetch { items, .. }) = response else {
            panic!("not a FETCH answer");
        };
        let envelope = envelope_from(1, items.into_inner());

        assert_eq!(envelope.subject.as_deref(), Some("\u{1f602}"));
        assert_eq!(
            envelope.from[0].name.as_deref(),
            Some("\u{201c}Ann\u{201d} \u{e9}")
        );
        assert_eq!(envelope.from[0].email, "a@example.org");
    }

    /// ENVELOPE group markers (RFC 3501 §7.4.2) are no address: a group
    /// lists its members alone, an empty one none, as on Maildir and mbox.
    #[test]
    fn an_envelope_group_lists_its_members_alone() {
        const LINE: &[u8] = b"* 1 FETCH (UID 7 ENVELOPE (NIL NIL \
            ((NIL NIL \"a\" \"x.org\")) NIL NIL \
            ((NIL NIL \"undisclosed-recipients\" NIL)(NIL NIL NIL NIL)\
            (NIL NIL \"Team\" NIL)(\"Bob\" NIL \"b\" \"x.org\")(NIL NIL NIL NIL)) \
            NIL NIL NIL NIL))\r\n";

        let (_, response) = ResponseCodec::new().decode(LINE).unwrap();
        let Response::Data(Data::Fetch { items, .. }) = response else {
            panic!("not a FETCH answer");
        };
        let envelope = envelope_from(1, items.into_inner());

        let to: Vec<(Option<&str>, &str)> = envelope
            .to
            .iter()
            .map(|address| (address.name.as_deref(), address.email.as_str()))
            .collect();
        assert_eq!(to, [(Some("Bob"), "b@x.org")]);
    }

    /// What `hasAttachment` reads off an untagged `FETCH` answer.
    fn has_attachment(body_structure: &str) -> bool {
        let line = format!("* 1 FETCH (BODYSTRUCTURE {body_structure})\r\n");
        let (_, response) = ResponseCodec::new().decode(line.as_bytes()).unwrap();
        let Response::Data(Data::Fetch { items, .. }) = response else {
            panic!("not a FETCH answer: {line}");
        };
        let structure = items
            .as_ref()
            .iter()
            .find_map(|item| match item {
                MessageDataItem::BodyStructure(structure) => Some(structure),
                _ => None,
            })
            .unwrap();
        body_structure_has_attachment(structure)
    }

    /// The `BODYSTRUCTURE` answers of Dovecot 2.4.5 for the unified
    /// attachment rows.
    mod attachment_rows {
        use super::has_attachment;

        const TEXT: &str =
            r#"("text" "plain" ("charset" "utf-8") NIL NIL "7bit" 5 0 NIL NIL NIL NIL)"#;

        #[test]
        fn a_part_disposed_as_attachment_counts() {
            assert!(has_attachment(&format!(
                r#"({TEXT}("application" "pdf" ("name" "a.pdf") NIL NIL "base64" 12 NIL ("attachment" ("filename" "a.pdf")) NIL NIL) "mixed" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn a_single_binary_part_without_disposition_counts() {
            assert!(has_attachment(
                r#"("application" "pdf" ("name" "a.pdf") NIL NIL "base64" 14 NIL NIL NIL NIL)"#
            ));
        }

        #[test]
        fn an_attached_message_without_disposition_counts() {
            assert!(has_attachment(&format!(
                r#"({TEXT}("message" "rfc822" NIL NIL NIL "7bit" 44 (NIL "inner" ((NIL NIL "c" "example.org")) ((NIL NIL "c" "example.org")) ((NIL NIL "c" "example.org")) NIL NIL NIL NIL NIL) ("text" "plain" ("charset" "us-ascii") NIL NIL "7bit" 5 0 NIL NIL NIL NIL) 3 NIL NIL NIL NIL) "mixed" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn a_binary_part_disposed_inline_does_not_count() {
            assert!(!has_attachment(&format!(
                r#"({TEXT}("application" "pdf" ("name" "a.pdf") NIL NIL "base64" 12 NIL ("inline" ("filename" "a.pdf")) NIL NIL) "mixed" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn an_image_disposed_inline_does_not_count() {
            assert!(!has_attachment(
                r#"(("text" "html" ("charset" "utf-8") NIL NIL "7bit" 17 0 NIL NIL NIL NIL)("image" "png" NIL "<i>" NIL "base64" 12 NIL ("inline" NIL) NIL NIL) "related" ("boundary" "b") NIL NIL NIL)"#
            ));
        }

        #[test]
        fn a_text_part_disposed_as_attachment_counts() {
            assert!(has_attachment(&format!(
                r#"({TEXT}("text" "csv" ("name" "a.csv" "charset" "us-ascii") NIL NIL "7bit" 3 0 NIL ("attachment" ("filename" "a.csv")) NIL NIL) "mixed" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn a_text_body_does_not_count() {
            assert!(!has_attachment(
                r#"("text" "plain" ("charset" "utf-8") NIL NIL "7bit" 7 1 NIL NIL NIL NIL)"#
            ));
        }

        #[test]
        fn the_bodies_of_an_alternative_do_not_count() {
            assert!(!has_attachment(&format!(
                r#"({TEXT}("text" "html" ("charset" "utf-8") NIL NIL "7bit" 12 0 NIL NIL NIL NIL) "alternative" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn an_image_named_by_a_content_id_does_not_count() {
            assert!(!has_attachment(
                r#"(("text" "html" ("charset" "utf-8") NIL NIL "7bit" 17 0 NIL NIL NIL NIL)("image" "png" NIL "<i>" NIL "base64" 12 NIL NIL NIL NIL) "related" ("boundary" "b") NIL NIL NIL)"#
            ));
        }

        #[test]
        fn a_bare_image_counts() {
            assert!(has_attachment(&format!(
                r#"({TEXT}("image" "png" ("name" "a.png") NIL NIL "base64" 12 NIL NIL NIL NIL) "mixed" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn a_bare_signature_does_not_count() {
            assert!(!has_attachment(&format!(
                r#"({TEXT}("application" "pgp-signature" ("name" "signature.asc") NIL NIL "7bit" 29 NIL NIL NIL NIL) "signed" ("protocol" "application/pgp-signature" "micalg" "pgp-sha256" "boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn a_signature_disposed_as_attachment_does_not_count() {
            assert!(!has_attachment(&format!(
                r#"({TEXT}("application" "pkcs7-signature" ("name" "smime.p7s") NIL NIL "base64" 12 NIL ("attachment" ("filename" "smime.p7s")) NIL NIL) "signed" ("protocol" "application/pkcs7-signature" "micalg" "sha-256" "boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn an_attachment_under_a_signature_counts() {
            assert!(has_attachment(&format!(
                r#"(({TEXT}("application" "pdf" ("name" "a.pdf") NIL NIL "base64" 12 NIL ("attachment" ("filename" "a.pdf")) NIL NIL) "mixed" ("boundary" "c") NIL NIL NIL)("application" "pgp-signature" ("name" "signature.asc") NIL NIL "7bit" 29 NIL NIL NIL NIL) "signed" ("protocol" "application/pgp-signature" "micalg" "pgp-sha256" "boundary" "b") NIL NIL NIL)"#
            )));
        }

        #[test]
        fn the_parts_of_an_encrypted_message_do_not_count() {
            assert!(!has_attachment(
                r#"(("application" "pgp-encrypted" NIL NIL NIL "7bit" 10 NIL NIL NIL NIL)("application" "octet-stream" NIL NIL NIL "7bit" 27 NIL NIL NIL NIL) "encrypted" ("protocol" "application/pgp-encrypted" "boundary" "b") NIL NIL NIL)"#
            ));
        }

        #[test]
        fn an_unknown_disposition_counts() {
            assert!(has_attachment(&format!(
                r#"({TEXT}("text" "plain" ("charset" "us-ascii") NIL NIL "7bit" 3 0 NIL ("x-foo" NIL) NIL NIL) "mixed" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        /// Dovecot answers an empty type for a `Content-Type` it cannot read.
        #[test]
        fn an_invalid_content_type_reads_as_text() {
            assert!(!has_attachment(&format!(
                r#"({TEXT}("" "" NIL NIL NIL "base64" 12 NIL NIL NIL NIL) "mixed" ("boundary" "b") NIL NIL NIL)"#
            )));
        }

        /// RFC 3501 lets a server omit the extension data; a part then
        /// has no disposition to read.
        #[test]
        fn a_part_without_extension_data_reads_as_having_no_disposition() {
            assert!(has_attachment(
                r#"("application" "pdf" NIL NIL NIL "base64" 14)"#
            ));
            assert!(!has_attachment(r#"("text" "csv" NIL NIL NIL "7bit" 3 0)"#));
        }

        #[test]
        fn a_multipart_sent_as_a_single_part_does_not_count() {
            assert!(!has_attachment(
                r#"("multipart" "mixed" NIL NIL NIL "7bit" 3)"#
            ));
        }

        /// RFC 3501 leaves the case of the media type to the server.
        #[test]
        fn the_case_of_the_media_type_does_not_matter() {
            assert!(!has_attachment(
                r#"("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 7 1 NIL NIL NIL NIL)"#
            ));
            assert!(has_attachment(
                r#"("APPLICATION" "PDF" NIL NIL NIL "BASE64" 14 NIL NIL NIL NIL)"#
            ));
            assert!(!has_attachment(
                r#"("APPLICATION" "PDF" NIL NIL NIL "BASE64" 14 NIL ("INLINE" NIL) NIL NIL)"#
            ));
        }
    }

    /// Write counts and item errors against a scripted IMAP server.
    mod copy_and_move {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            thread::{self, JoinHandle},
        };

        use crate::{
            config::imap::ImapConfig,
            error::ErrorCode,
            imap::client::ImapClient,
            unified::email::{
                flag::{Flag, FlagOp, IanaFlag},
                outcome::Outcome,
            },
        };

        /// The `COPYUID` code a scripted server answers for the source
        /// UIDs that matched, if any.
        type CopyUid = fn(&[u32]) -> Option<String>;

        /// A valid `COPYUID` for the matched UIDs, none when nothing
        /// matched, as RFC 4315 has it.
        fn valid(matched: &[u32]) -> Option<String> {
            (!matched.is_empty()).then(|| copyuid(7, matched))
        }

        /// Mailo's `COPYUID` with the forbidden UIDVALIDITY 0 ([#781]).
        ///
        /// [#781]: https://github.com/pimalaya/himalaya/issues/781
        fn uidvalidity_zero(matched: &[u32]) -> Option<String> {
            (!matched.is_empty()).then(|| copyuid(0, matched))
        }

        /// A `COPYUID` malformed beyond that deviation (a UID 0), which
        /// io-imap leaves unreadable.
        fn garbage(matched: &[u32]) -> Option<String> {
            (!matched.is_empty()).then(|| String::from("COPYUID 0 0 100"))
        }

        /// No `COPYUID` at all, as a server without UIDPLUS answers.
        fn none(_: &[u32]) -> Option<String> {
            None
        }

        fn copyuid(uid_validity: u32, matched: &[u32]) -> String {
            let set = |uids: Vec<u32>| {
                uids.iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let destination = (100..).take(matched.len()).collect();
            format!(
                "COPYUID {uid_validity} {} {}",
                set(matched.to_vec()),
                set(destination)
            )
        }

        /// `FETCH` lines: a whole one, a seven-multipart `BODYSTRUCTURE`
        /// ([#801]), an undecodable `UID` and one that does not scan.
        ///
        /// [#801]: https://github.com/pimalaya/himalaya/issues/801
        fn fetch_lines() -> String {
            let text = r#"("text" "plain" ("charset" "utf-8") NIL NIL "7bit" 2 1)"#;
            let nested = (0..7).fold(String::from(text), |inner, _| {
                format!("({inner} \"mixed\")")
            });

            format!(
                "* 1 FETCH (UID 1 FLAGS (\\Seen) RFC822.SIZE 10 BODYSTRUCTURE {text})\r\n\
                 * 2 FETCH (UID 2 FLAGS () RFC822.SIZE 20 BODYSTRUCTURE {nested})\r\n\
                 * 3 FETCH (UID 3 INTERNALDATE oops)\r\n\
                 * 4 FETCH (UID 0 FLAGS ())\r\n"
            )
        }

        fn parse_set(set: &str) -> Vec<u32> {
            set.split(',')
                .flat_map(|part| match part.split_once(':') {
                    Some((a, b)) => (a.parse().unwrap()..=b.parse().unwrap()).collect(),
                    None => vec![part.parse().unwrap()],
                })
                .collect()
        }

        /// Serves one session whose INBOX holds `existing`, returning the
        /// address and the commands it received, tags stripped.
        fn serve(
            capabilities: &'static str,
            mut existing: Vec<u32>,
            copy_uid: CopyUid,
        ) -> (String, JoinHandle<Vec<String>>) {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut out = stream;
                let mut commands = Vec::new();
                write!(out, "* PREAUTH [CAPABILITY {capabilities}] ready\r\n").unwrap();

                let mut line = String::new();
                while reader.read_line(&mut line).unwrap() > 0 {
                    let (tag, command) = line.trim_end().split_once(' ').unwrap();
                    let (tag, command) = (tag.to_string(), command.to_string());
                    line.clear();
                    commands.push(command.clone());
                    let words: Vec<&str> = command.split(' ').collect();
                    let reply = match words.as_slice() {
                        ["SELECT", ..] => format!(
                            "* {} EXISTS\r\n* OK [UIDVALIDITY 7] ok\r\n\
                             {tag} OK [READ-WRITE] done\r\n",
                            existing.len()
                        ),
                        ["UID", "SEARCH", "UID", set] => {
                            let found: Vec<String> = parse_set(set)
                                .into_iter()
                                .filter(|uid| existing.contains(uid))
                                .map(|uid| uid.to_string())
                                .collect();
                            let found = found.iter().map(|uid| format!(" {uid}"));
                            let found: String = found.collect();
                            format!("* SEARCH{found}\r\n{tag} OK done\r\n")
                        }
                        ["UID", verb @ ("COPY" | "MOVE"), set, _] => {
                            let matched: Vec<u32> = parse_set(set)
                                .into_iter()
                                .filter(|uid| existing.contains(uid))
                                .collect();
                            if *verb == "MOVE" {
                                existing.retain(|uid| !matched.contains(uid));
                            }
                            match copy_uid(&matched) {
                                Some(code) => format!("{tag} OK [{code}] done\r\n"),
                                None => format!("{tag} OK done\r\n"),
                            }
                        }
                        ["LIST", ..] => format!(
                            "* LIST () \"/\" INBOX\r\n* LIST () \"/\" Locked\r\n\
                             * LIST (\\Sent) \"/\" \"Sent Messages\"\r\n\
                             {tag} OK done\r\n"
                        ),
                        ["FETCH", "1:*", ..] => format!("{}{tag} OK done\r\n", fetch_lines()),
                        ["CREATE", "Dup"] => format!("{tag} NO [ALREADYEXISTS] exists\r\n"),
                        ["CREATE", "Locked"] => format!("{tag} NO already there\r\n"),
                        ["CREATE", "Fresh"] => format!("{tag} NO [NOPERM] denied\r\n"),
                        ["STATUS", "Locked", ..] => format!("{tag} NO denied\r\n"),
                        ["STATUS", ..] => {
                            format!("* STATUS INBOX (MESSAGES 1 UNSEEN 0)\r\n{tag} OK done\r\n")
                        }
                        ["UID", "FETCH", "7", ..] => {
                            format!("* 1 FETCH (UID 7 BODY[] \"x)\r\n{tag} OK done\r\n")
                        }
                        ["UID", "EXPUNGE", set] => {
                            let set = parse_set(set);
                            existing.retain(|uid| !set.contains(uid));
                            format!("{tag} OK done\r\n")
                        }
                        ["LOGOUT"] => {
                            write!(out, "* BYE bye\r\n{tag} OK done\r\n").unwrap();
                            break;
                        }
                        _ => format!("{tag} OK done\r\n"),
                    };
                    out.write_all(reply.as_bytes()).unwrap();
                }

                commands
            });

            (format!("imap://{addr}"), handle)
        }

        fn client(server: &str) -> ImapClient {
            let config: ImapConfig = toml::from_str(&format!("server = \"{server}\"")).unwrap();
            ImapClient::new(config, &mut pimalaya_config::secret::SecretResolver::new()).unwrap()
        }

        /// Runs `op` against a scripted server, returning its count, the
        /// ids of its item errors and the commands the server received.
        fn run(
            capabilities: &'static str,
            existing: &[u32],
            copy_uid: CopyUid,
            op: impl FnOnce(&mut ImapClient) -> anyhow::Result<Outcome<usize>>,
        ) -> (usize, Vec<String>) {
            let (count, _, commands) = run_outcome(capabilities, existing, copy_uid, op);
            (count, commands)
        }

        /// Runs `op` as [`run`] does, returning the ids its item errors
        /// name beside its count.
        fn run_outcome(
            capabilities: &'static str,
            existing: &[u32],
            copy_uid: CopyUid,
            op: impl FnOnce(&mut ImapClient) -> anyhow::Result<Outcome<usize>>,
        ) -> (usize, Vec<String>, Vec<String>) {
            let (server, handle) = serve(capabilities, existing.to_vec(), copy_uid);
            let mut client = client(&server);
            let outcome = op(&mut client).unwrap();
            drop(client);

            let missing = outcome
                .errors
                .into_iter()
                .inspect(|error| assert_eq!(error.code, Some(ErrorCode::MessageNotFound)))
                .filter_map(|error| error.id)
                .collect();

            (outcome.value, missing, handle.join().unwrap())
        }

        fn has(commands: &[String], prefix: &str) -> bool {
            commands.iter().any(|command| command.starts_with(prefix))
        }

        const UIDPLUS: &str = "IMAP4rev1 UIDPLUS";
        const PLAIN: &str = "IMAP4rev1";
        const MOVE: &str = "IMAP4rev1 UIDPLUS MOVE";

        #[test]
        fn a_listing_counts_the_mailbox_from_its_select() {
            let (server, handle) = serve(PLAIN, vec![1, 2, 3], none);
            let mut client = client(&server);
            let (_, total) = client
                .list_envelopes_counted("INBOX", Some(1), Some(2), false)
                .unwrap();
            drop(client);
            handle.join().unwrap();

            assert_eq!(total, Some(3));
        }

        /// A `CREATE` refused as existing, or uncoded on a listed mailbox,
        /// is `mailbox-duplicate`; any other refusal keeps its own code.
        #[test]
        fn a_create_of_an_existing_mailbox_is_a_duplicate() {
            let duplicate = Some(ErrorCode::MailboxDuplicate);
            let denied = Some(ErrorCode::PermissionDenied);

            for (name, code) in [("Dup", duplicate), ("Locked", duplicate), ("Fresh", denied)] {
                let (server, handle) = serve(PLAIN, Vec::new(), none);
                let mut client = client(&server);
                let err = client.create_mailbox(name, None).unwrap_err();
                drop(client);
                handle.join().unwrap();

                assert_eq!(crate::error::code(&err), code, "{name}: {err:#}");
            }
        }

        #[test]
        fn a_page_reaching_the_last_message_has_no_next() {
            let (server, handle) = serve(PLAIN, vec![1, 2, 3, 4], none);
            let mut client = client(&server);
            let mut more = |page, size| {
                let (listed, _) = client
                    .list_envelopes_counted("INBOX", Some(page), Some(size), false)
                    .unwrap();
                listed.value.more
            };

            assert!(more(1, 2));
            assert!(!more(2, 2));
            assert!(!more(1, 4));
            assert!(!more(3, 2));
            drop(client);
            handle.join().unwrap();
        }

        #[test]
        fn a_keepalive_sends_noop() {
            let (server, handle) = serve(PLAIN, Vec::new(), none);
            let mut client = client(&server);
            client.keep_alive().unwrap();
            drop(client);

            assert!(has(&handle.join().unwrap(), "NOOP"));
        }

        #[test]
        fn a_keepalive_on_a_closed_connection_fails() {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                write!(stream, "* PREAUTH [CAPABILITY {PLAIN}] ready\r\n").unwrap();
            });

            let mut client = client(&format!("imap://{addr}"));
            handle.join().unwrap();

            assert!(client.keep_alive().is_err());
        }

        fn mv(
            ids: &'static [&'static str],
        ) -> impl FnOnce(&mut ImapClient) -> anyhow::Result<Outcome<usize>> {
            move |client| client.move_messages("INBOX", "Archive", ids)
        }

        #[test]
        fn a_valid_copyuid_answers_the_copy_fallback_move() {
            let (count, missing, commands) = run_outcome(UIDPLUS, &[1], valid, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert_eq!(missing, ["999"]);
            assert!(!has(&commands, "UID SEARCH"), "{commands:?}");
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(has(&commands, "UID EXPUNGE 1"), "{commands:?}");
        }

        #[test]
        fn a_uidvalidity_zero_copyuid_answers_the_copy_fallback_move() {
            let (count, commands) = run(UIDPLUS, &[1], uidvalidity_zero, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(!has(&commands, "UID SEARCH"), "{commands:?}");
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(has(&commands, "UID EXPUNGE 1"), "{commands:?}");
        }

        #[test]
        fn an_unreadable_copyuid_still_completes_the_move() {
            let (count, commands) = run(UIDPLUS, &[1], garbage, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID SEARCH UID 1,999"), "{commands:?}");
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(has(&commands, "UID EXPUNGE 1"), "{commands:?}");
        }

        #[test]
        fn no_copyuid_from_a_uidplus_server_is_a_message_not_found() {
            let (count, commands) = run(UIDPLUS, &[1], valid, mv(&["999"]));
            assert_eq!(count, 0);
            assert!(!has(&commands, "UID STORE"), "{commands:?}");
            assert!(!has(&commands, "UID EXPUNGE"), "{commands:?}");
        }

        #[test]
        fn without_uidplus_the_source_tells_what_was_copied() {
            let (count, commands) = run(PLAIN, &[1], none, mv(&["1"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(!has(&commands, "UID EXPUNGE"), "{commands:?}");

            let (count, commands) = run(PLAIN, &[1], none, mv(&["999"]));
            assert_eq!(count, 0);
            assert!(!has(&commands, "UID STORE"), "{commands:?}");
        }

        #[test]
        fn a_move_without_copyuid_counts_the_messages_present() {
            let (count, missing, commands) = run_outcome(MOVE, &[1], none, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert_eq!(missing, ["999"]);
            assert!(has(&commands, "UID MOVE 1 "), "{commands:?}");

            let (count, commands) = run(MOVE, &[1], valid, mv(&["999"]));
            assert_eq!(count, 0);
            assert!(!has(&commands, "UID MOVE"), "{commands:?}");
        }

        /// A move onto its own mailbox looks the UIDs up and changes
        /// nothing, where the server would give the messages new UIDs.
        #[test]
        fn a_move_onto_its_own_mailbox_changes_nothing() {
            for capabilities in [MOVE, PLAIN] {
                let onto_itself =
                    |client: &mut ImapClient| client.move_messages("INBOX", "inbox", &["1", "999"]);
                let (count, missing, commands) = run_outcome(capabilities, &[1], none, onto_itself);
                assert_eq!(count, 1);
                assert_eq!(missing, ["999"]);
                assert!(has(&commands, "UID SEARCH UID 1,999"), "{commands:?}");
                assert!(!has(&commands, "UID MOVE"), "{commands:?}");
                assert!(!has(&commands, "UID COPY"), "{commands:?}");
                assert!(!has(&commands, "UID STORE"), "{commands:?}");
            }
        }

        #[test]
        fn a_copy_counts_what_was_copied() {
            let copy = |ids: &'static [&'static str]| {
                move |client: &mut ImapClient| client.copy_messages("INBOX", "Archive", ids)
            };
            assert_eq!(run(UIDPLUS, &[1, 2], valid, copy(&["1", "2"])).0, 2);
            let (count, commands) = run(UIDPLUS, &[1], uidvalidity_zero, copy(&["1"]));
            assert_eq!(count, 1);
            assert!(!has(&commands, "UID SEARCH"), "{commands:?}");
            let (count, commands) = run(UIDPLUS, &[1], garbage, copy(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID SEARCH UID 1,999"), "{commands:?}");
            assert_eq!(run(UIDPLUS, &[1], valid, copy(&["999"])).0, 0);
            assert_eq!(run(PLAIN, &[1], none, copy(&["1", "999"])).0, 1);
        }

        #[test]
        fn a_trash_delete_counts_the_messages_present() {
            let delete = |ids: &'static [&'static str]| {
                move |client: &mut ImapClient| {
                    let outcome = client.delete_messages("INBOX", ids)?;
                    Ok(outcome.map(|(count, _)| count))
                }
            };

            let (count, commands) = run(UIDPLUS, &[1], none, delete(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID SEARCH UID 1,999"), "{commands:?}");
            assert!(has(&commands, "UID EXPUNGE 1,999"), "{commands:?}");

            let (count, commands) = run(UIDPLUS, &[1], none, delete(&["999"]));
            assert_eq!(count, 0);
            assert!(!has(&commands, "UID EXPUNGE"), "{commands:?}");

            let (_, missing, _) = run_outcome(UIDPLUS, &[1], none, delete(&["1", "999"]));
            assert_eq!(missing, ["999"]);

            assert_eq!(run(PLAIN, &[1], none, delete(&["999"])).0, 0);
        }

        /// A UIDPLUS trash delete finding no id still says it removes, so it
        /// never reads as flagged and asks for an expunge (#814).
        #[test]
        fn a_trash_delete_finding_nothing_still_says_how_it_deletes() {
            for (capabilities, removes) in [(UIDPLUS, true), (PLAIN, false)] {
                let (server, handle) = serve(capabilities, vec![1], none);
                let mut client = client(&server);
                let outcome = client.delete_messages("INBOX", &["999"]).unwrap();
                drop(client);
                let commands = handle.join().unwrap();

                assert_eq!(outcome.value, (0, removes), "{capabilities}");
                assert!(!has(&commands, "UID EXPUNGE"), "{commands:?}");
            }
        }

        /// A UID neither echoed by the `UID STORE` nor found by the
        /// `UID SEARCH` after it is an item error, the others counted.
        #[test]
        fn a_flag_on_an_unknown_uid_is_an_item_error() {
            let flagged = [Flag::from_iana(IanaFlag::Flagged)];
            let store = move |client: &mut ImapClient| {
                client.store_flags("INBOX", &["1", "999"], &flagged, FlagOp::Add)
            };

            let (count, missing, commands) = run_outcome(UIDPLUS, &[1], none, store);
            assert_eq!(count, 1);
            assert_eq!(missing, ["999"]);
            assert!(has(&commands, "UID SEARCH UID 1,999"), "{commands:?}");
        }

        /// A mailbox whose `STATUS` the server refuses is listed without
        /// counts, beside its error.
        #[test]
        fn a_refused_status_is_an_unreadable_mailbox() {
            let (server, handle) = serve(PLAIN, vec![1], none);
            let mut client = client(&server);
            let outcome = client.list_mailboxes(true).unwrap();
            drop(client);
            handle.join().unwrap();

            let counts: Vec<_> = outcome.value.iter().map(|m| (m.unread, m.total)).collect();
            assert_eq!(
                counts,
                [(Some(0), Some(1)), (None, None), (Some(0), Some(1))]
            );

            let [error] = outcome.errors.as_slice() else {
                panic!("{:?}", outcome.errors);
            };
            assert_eq!(error.id.as_deref(), Some("Locked"));
            assert_eq!(error.code, Some(ErrorCode::MailboxUnreadable));
        }

        /// A line decoded in part lists its message beside an item error;
        /// a line whose UID failed, or that does not scan, lists none.
        #[test]
        fn a_line_decoded_in_part_lists_its_message_beside_its_error() {
            let (server, handle) = serve(PLAIN, vec![1, 2, 3, 4], none);
            let mut client = client(&server);
            let outcome = client.list_envelopes("INBOX", None, None, true).unwrap();
            drop(client);
            handle.join().unwrap();

            let envelopes: Vec<_> = outcome
                .value
                .iter()
                .map(|envelope| (envelope.id.as_str(), envelope.has_attachment))
                .collect();
            assert_eq!(envelopes, [("2", None), ("1", Some(false))]);

            let errors: Vec<_> = outcome
                .errors
                .iter()
                .map(|error| (error.id.as_deref(), error.code, error.error.as_str()))
                .collect();
            let unreadable = Some(ErrorCode::MessageUnreadable);
            assert_eq!(
                errors,
                [
                    (
                        Some("2"),
                        unreadable,
                        "Cannot decode BODYSTRUCTURE of the FETCH response"
                    ),
                    (None, unreadable, "Cannot decode UID of the FETCH response"),
                    (
                        None,
                        unreadable,
                        "Cannot decode a line of the FETCH response"
                    ),
                ]
            );
        }

        /// A read whose `FETCH` answer holds a line that does not decode
        /// and no message is unreadable, not missing.
        #[test]
        fn reading_an_undecodable_answer_is_unreadable() {
            let (server, handle) = serve(PLAIN, vec![1], none);
            let mut client = client(&server);
            let err = client.get_message("INBOX", "7", false).unwrap_err();
            drop(client);
            handle.join().unwrap();

            let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
            assert_eq!(code, Some(ErrorCode::MessageUnreadable), "{err:#}");
        }

        #[test]
        fn reading_an_unknown_id_names_it_and_the_mailbox() {
            let (server, handle) = serve(PLAIN, vec![1], none);
            let mut client = client(&server);
            let err = client.get_message("INBOX", "999", false).unwrap_err();
            drop(client);
            handle.join().unwrap();

            let expected = "Message `999` not found in mailbox `INBOX`";
            assert_eq!(err.to_string(), expected);
            let code = pimalaya_cli::error::code_of::<crate::error::ErrorCode>(&err);
            assert_eq!(code, Some(crate::error::ErrorCode::MessageNotFound));
        }

        /// A read names its mailbox by role as a listing does: `sent`
        /// selects the mailbox whose special-use attribute is `\Sent`.
        #[test]
        fn reading_resolves_a_mailbox_role() {
            let (server, handle) = serve(PLAIN, vec![1], none);
            let mut client = client(&server);
            let _ = client.get_message("sent", "999", false);
            drop(client);
            let commands = handle.join().unwrap();

            let selects: Vec<&str> = commands
                .iter()
                .filter(|command| command.starts_with("SELECT"))
                .map(String::as_str)
                .collect();
            assert_eq!(selects, ["SELECT \"Sent Messages\""], "{commands:?}");
        }

        #[test]
        fn thread_support_follows_the_advertised_algorithms() {
            use io_imap::types::extensions::thread::ThreadingAlgorithm;

            let (server, handle) = serve("IMAP4rev1 THREAD=REFERENCES", Vec::new(), none);
            let client = client(&server);
            let references = client.supports_thread(&ThreadingAlgorithm::References);
            let subject = client.supports_thread(&ThreadingAlgorithm::OrderedSubject);
            drop(client);
            handle.join().unwrap();

            assert!(references);
            assert!(!subject);
        }
    }

    /// A plain `LIST` row carrying a special-use attribute gives its role,
    /// `INBOX` its own, and the other attributes none.
    #[test]
    fn a_special_use_attribute_gives_the_role() {
        use io_imap::types::{flag::FlagNameAttribute, mailbox::Mailbox as ImapMailbox};

        use crate::unified::email::mailbox::MailboxRole;

        let row = |name: &str, attrs: &[&str]| {
            let attrs = attrs
                .iter()
                .map(|attr| FlagNameAttribute::from(Atom::try_from(attr.to_string()).unwrap()))
                .collect();
            (
                ImapMailbox::try_from(name.to_string()).unwrap(),
                None,
                attrs,
            )
        };

        let role = |name, attrs| super::mailbox_from(row(name, attrs)).role;

        assert_eq!(role("INBOX", &[]), Some(MailboxRole::Inbox));
        assert_eq!(
            role("Deleted Messages", &["HasNoChildren", "Trash"]),
            Some(MailboxRole::Trash)
        );
        assert_eq!(role("Sent Messages", &["Sent"]), Some(MailboxRole::Sent));
        assert_eq!(role("Lists", &["HasChildren", "Subscribed"]), None);
    }
}
