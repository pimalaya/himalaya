//! # IMAP backend
//!
//! The IMAP adapter of the unified client, glue over the
//! io-imap session [`ImapClient`] already wraps.
//!
//! Each method takes and returns the CLI's own [`crate::email`] types, so
//! the work here is converting between those and io-imap's wire types.

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
        select::ImapMailboxSelectOptions,
        store::ImapMessageStoreOptions,
    },
    rfc5256::sort::ImapMessageSortOptions,
    rfc6851::r#move::ImapMessageMoveOptions,
    send::ImapOutput,
    types::{
        body::{BodyStructure, SpecificFields},
        core::{AString, Atom, Vec1},
        datetime::NaiveDate as ImapNaiveDate,
        envelope::Address as ImapAddress,
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
    capability,
    email::{
        address::Address,
        envelope::{
            Envelope, EnvelopePage, normalize_message_id, page_follows, parse_message_ids,
            parse_rfc2822_date,
        },
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxIndex, MailboxRole, child_path, renamed_path, validate_level},
        outcome::Outcome,
        search::{
            error::unsupported,
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        },
    },
    error::{
        ErrorCode, ItemError, existing_mailbox, mailbox_not_found, message_not_found,
        missing_message,
    },
    imap::{client::ImapClient, utils::output_data},
    shared::message::{header, part},
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
                        let id = Some(mailbox.id.as_str());
                        let err = anyhow!(err).context("Cannot count the messages");
                        let mut error = ItemError::new(id, &err);
                        error.code = Some(ErrorCode::MailboxUnreadable);
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

    /// Maps a role (`sent`, `trash`...) onto the mailbox whose special-use
    /// attribute names it, a mailbox literally named so winning; any other
    /// name passes through without a round trip.
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

    /// Lists envelopes as [`Self::list_envelopes`] does, beside how many
    /// messages the mailbox holds: the `EXISTS` of the `SELECT` the
    /// listing sends anyway.
    pub fn list_envelopes_counted(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<(Outcome<EnvelopePage>, Option<u64>)> {
        let mbox = parse_mailbox(mailbox)?;
        let select = self.select(mbox, ImapMailboxSelectOptions::default())?;
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

    /// Searches a mailbox with the shared query.
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
        let mbox = parse_mailbox(mailbox)?;
        let select = self.select(mbox, ImapMailboxSelectOptions::default())?;
        if output_data("SELECT", select).exists.unwrap_or(0) == 0 {
            return Ok(Outcome::from(EnvelopePage::default()));
        }

        let search_criteria = search_keys(query.and_then(|q| q.filter.as_ref()))?;
        let sort_criteria = sort_criteria(query.and_then(|q| q.sort.as_deref()));
        let fallback = self.sort_fallback();

        let uids = self.sort(
            sort_criteria,
            search_criteria,
            ImapMessageSortOptions {
                uid: true,
                fallback,
            },
        )?;
        let uids = output_data("SORT", uids);
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

        // NOTE: HEADER matches a substring, so a `message-id` clause on the
        // chain of `and` from the root is checked whole once fetched.
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

    /// Adds, sets or removes flags on a UID set, returning how many of
    /// the UIDs exist in the mailbox.
    ///
    /// `UID STORE` answers `OK` for UIDs that match nothing (RFC 3501
    /// section 6.4.8), so the untagged `FETCH` echoes are the first
    /// count. A server is free to skip the echo of a message whose flags
    /// did not change, so fewer echoes than UIDs is settled by a
    /// `UID SEARCH` on the same set. A UID found by neither is an item
    /// error.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(ids)?;
        let imap_flags: Vec<ImapFlag<'static>> = flags.iter().map(flag_from).collect();
        let kind = match op {
            FlagOp::Add => StoreType::Add,
            FlagOp::Set => StoreType::Replace,
            FlagOp::Remove => StoreType::Remove,
        };

        let selected = self.select(mbox, ImapMailboxSelectOptions::default())?;
        let selected = output_data("SELECT", selected);

        // NOTE: a server sending no PERMANENTFLAGS lets every flag be
        // changed permanently (RFC 3501 section 7.1).
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

    /// Fetches one message's raw RFC 5322 bytes, marking it seen only
    /// when asked.
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(&[id])?;

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        // NOTE: `BODY[]` sets `\Seen` server-side as a side effect, so
        // `--seen` costs no extra round trip where `BODY.PEEK[]` does not.
        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: !seen,
            }]);
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

        let body = data
            .into_values()
            .flat_map(|items| items.into_inner().into_iter())
            .find_map(|item| match item {
                MessageDataItem::BodyExt { data, .. } => data.0.map(|d| d.as_ref().to_vec()),
                _ => None,
            });

        match body {
            Some(body) => Ok(body),
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

        let (_, appenduid) = self.append(
            mbox.clone(),
            &raw,
            ImapMessageAppendOptions {
                flags: imap_flags,
                date: None,
                non_sync: false,
            },
        )?;

        if let Some((_, uid)) = appenduid {
            return Ok(Some(uid.to_string()));
        }

        // NOTE: without `APPENDUID`, which a server may omit even with
        // UIDPLUS, the UID is recovered by searching the message's own
        // `Message-ID`, which it therefore has to carry.
        let message_id = header::parse_headers(&raw)
            .root
            .header::<MESSAGE_ID>()
            .and_then(|ids| ids.into_iter().next())
            .filter(|id| !id.is_empty());
        let Some(message_id) = message_id else {
            debug!("no APPENDUID and no Message-ID to search, appended UID unknown");
            return Ok(None);
        };

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        let field =
            AString::try_from("Message-ID").map_err(|_| anyhow!("Invalid IMAP search header"))?;
        let value = AString::try_from(message_id.clone())
            .map_err(|_| anyhow!("Invalid IMAP search Message-ID value"))?;
        let criteria = Vec1::from(SearchKey::Header(field, value));
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
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;
        let copy_uid = self.copy(
            sequence_set.clone(),
            target,
            ImapMessageCopyOptions { uid: true },
        )?;

        let copied = self.copied_uids(copy_uid, sequence_set, ids.len())?;
        Ok(found_uids(ids, &copied))
    }

    /// Moves a UID set between two mailboxes.
    ///
    /// With RFC 6851 MOVE this is a single `UID MOVE`. Without it, it is
    /// the sequence RFC 6851 §1 describes for such servers: `UID COPY`,
    /// then `\Deleted` and, when RFC 4315 UIDPLUS is advertised, a
    /// `UID EXPUNGE` of the source UIDs really copied (see
    /// `Self::copied_uids`). Without UIDPLUS the messages stay flagged
    /// in the source, as `delete_messages` already does in the trash. A
    /// plain `EXPUNGE` would also remove every other `\Deleted` message.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;

        // NOTE: a move onto its own mailbox changes nothing, as on every
        // backend, where a server would give the messages new UIDs.
        if same_mailbox(from, to) {
            let present = self.present_uids(sequence_set)?;
            return Ok(found_uids(ids, &present));
        }

        if self.supports_move() {
            // NOTE: `UID MOVE` takes the messages out of the source, so
            // without a readable `COPYUID` nothing is left there to tell
            // a moved message from one that never existed. The UIDs
            // present are searched beforehand, and only they are moved.
            let present = self.present_uids(sequence_set)?;
            if present.is_empty() {
                return Ok(found_uids(ids, &present));
            }
            let copy_uid = self.r#move(
                uid_sequence_set(present.clone())?,
                target,
                ImapMessageMoveOptions { uid: true },
            )?;
            // NOTE: a COPYUID counting more messages than were sent is
            // not believed.
            let mut outcome = found_uids(ids, &present);
            if let Some(copy_uid) = copy_uid
                && copy_uid.count() < present.len() as u64
            {
                outcome.value = copy_uid.count() as usize;
            }
            return Ok(outcome);
        }

        let copy_uid = self.copy(
            sequence_set.clone(),
            target,
            ImapMessageCopyOptions { uid: true },
        )?;
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

    /// Creates `name` at the top, or under `parent` joined by the
    /// server's hierarchy delimiter, returning the new mailbox's id, its
    /// full name. A NAME holding the delimiter is refused, NAME being one
    /// level.
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
                // NOTE: the parent is joined as the server spells it, so
                // `Inbox` nests under the reserved `INBOX`.
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

    /// The failure of a `CREATE` of `path`, under `mailbox-duplicate` when
    /// the server names the mailbox as existing (`ALREADYEXISTS`, RFC 5530
    /// section 3) or, refusing it with no code, lists it.
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

    /// Renames the last level of `mailbox` to `name`, its parent kept,
    /// returning the mailbox's new id, its full name. A NAME holding the
    /// delimiter is refused, NAME being one level.
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<String> {
        let path = match self.delimiter()? {
            Some(delimiter) => {
                validate_level(name, delimiter)?;
                renamed_path(mailbox, delimiter, name)
            }
            None => name.to_string(),
        };

        if let Err(err) = self.rename(parse_mailbox(mailbox)?, parse_mailbox(&path)?) {
            return Err(self.explain(mailbox, err.into(), None));
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
        Err(self.explain(mailbox, err.into(), delimiter))
    }

    /// Replaces the server's refusal of a write on `mailbox` by what the
    /// mailboxes it lists explain: `mailbox` missing, or holding others
    /// when `delimiter` is given. The server's error stays otherwise.
    fn explain(
        &mut self,
        mailbox: &str,
        err: anyhow::Error,
        delimiter: Option<char>,
    ) -> anyhow::Error {
        let Ok(names) = self.mailbox_names() else {
            return err;
        };

        if !names.iter().any(|known| same_name(known, mailbox)) {
            return mailbox_not_found(format!("IMAP mailbox `{mailbox}` not found"));
        }

        let Some(delimiter) = delimiter else {
            return err;
        };

        let prefix = format!("{mailbox}{delimiter}");
        let children: Vec<String> = names
            .into_iter()
            .filter(|name| name.starts_with(&prefix))
            .collect();

        if children.is_empty() {
            return err;
        }

        let children = children.join("`, `");
        anyhow!("Cannot delete IMAP mailbox `{mailbox}`: it holds `{children}`; delete them first")
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

    /// Permanently deletes a UID set from the trash, returning how many
    /// of the UIDs the mailbox held and whether the mailbox removes them
    /// rather than flag them, whatever it held.
    ///
    /// They are flagged `\Deleted`, then `UID EXPUNGE`d one by one when
    /// the server advertises RFC 4315 UIDPLUS, which leaves every other
    /// `\Deleted` message alone. Without it they are only flagged. The
    /// count comes from the STORE echoes, else from a `UID SEARCH UID`
    /// run before the expunge.
    pub fn delete_messages(
        &mut self,
        mailbox: &str,
        ids: &[&str],
    ) -> Result<Outcome<(usize, bool)>> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(ids)?;

        self.select(mbox, ImapMailboxSelectOptions::default())?;
        let echoes = self.store(
            sequence_set.clone(),
            StoreType::Add,
            vec![ImapFlag::Deleted],
            ImapMessageStoreOptions { uid: true },
        )?;

        let present = self.stored_uids(echoes, sequence_set.clone(), ids.len())?;
        let outcome = found_uids(ids, &present);

        // NOTE: what removes says how the mailbox deletes, so a delete
        // that found nothing never reads as flagged and asks for an
        // expunge; only the expunge waits for a found UID.
        let removes = self.supports_uidplus();
        if removes && outcome.value > 0 {
            self.uid_expunge(sequence_set)?;
        }

        Ok(outcome.map(|count| (count, removes)))
    }

    /// The UIDs of `requested` a `UID STORE` found, from its `echoes` when
    /// they number `asked`, else from a `UID SEARCH` of the set, a server
    /// being free to skip the echo of a message whose flags did not change.
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
    /// A RFC 4315 `COPYUID` lists exactly those, so it answers alone,
    /// including Mailo's, whose forbidden UIDVALIDITY 0 io-imap still
    /// reads ([#781]). Without one the tagged `OK` says nothing of what
    /// matched: a server without UIDPLUS never sends one, a UIDPLUS
    /// server sends none when nothing matched (an `OK` all the same,
    /// RFC 9051 §6.4.9), and one malformed beyond that deviation is
    /// reported as missing. `COPY` leaves the source untouched, so a
    /// `UID SEARCH` of the requested UIDs still there finds the ones
    /// copied. A `COPYUID` counting more than the `limit` UIDs asked is
    /// read as missing too, a server being free to announce billions.
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

/// Converts one `LIST` row into the shared [`Mailbox`] shape.
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

/// Folds one `FETCH` row into a shared [`Envelope`].
pub(crate) fn envelope_from(seq: u32, items: Vec<MessageDataItem<'static>>) -> Envelope {
    let mut id = String::new();
    let mut message_id: Option<String> = None;
    let mut in_reply_to = Vec::new();
    let mut flags = BTreeSet::new();
    let mut subject = String::new();
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
                    subject = header::decode_bytes(s.as_ref());
                }
                if let Some(d) = env.date.into_option() {
                    date = parse_rfc2822_date(&bytes_to_string(d.as_ref()));
                }
                if let Some(m) = env.message_id.into_option() {
                    message_id = normalize_message_id(&bytes_to_string(m.as_ref()));
                }
                // NOTE: the ninth ENVELOPE element (RFC 3501 section
                // 7.4.2), so a reply's parent costs no extra round trip.
                if let Some(m) = env.in_reply_to.into_option() {
                    in_reply_to = parse_message_ids(&bytes_to_string(m.as_ref()));
                }
                from = env.from.iter().map(address_from).collect();
                to = env.to.iter().map(address_from).collect();
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

/// Converts a fetched flag into a shared [`Flag`], dropping the
/// non-flag fetch items.
fn flag_from_fetch(fetch: FlagFetch<'_>) -> Option<Flag> {
    let FlagFetch::Flag(flag) = fetch else {
        return None;
    };
    Some(Flag::from_raw(flag.to_string()))
}

/// Converts an ENVELOPE address into a shared [`Address`].
fn address_from(addr: &ImapAddress<'_>) -> Address {
    let name = addr
        .name
        .0
        .as_ref()
        .map(|s| header::decode_bytes(s.as_ref()))
        .filter(|s| !s.is_empty());

    let mailbox = addr
        .mailbox
        .0
        .as_ref()
        .map(|s| bytes_to_string(s.as_ref()))
        .unwrap_or_default();
    let host = addr
        .host
        .0
        .as_ref()
        .map(|s| bytes_to_string(s.as_ref()))
        .unwrap_or_default();

    let email = if mailbox.is_empty() {
        host
    } else if host.is_empty() {
        mailbox
    } else {
        format!("{mailbox}@{host}")
    };

    Address { name, email }
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

/// Maps a shared [`Flag`] onto its IMAP wire counterpart.
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

/// Refuses a keyword the mailbox would keep for the session only, its
/// `PERMANENTFLAGS` listing neither it nor `\*` (RFC 3501 section 7.1), so
/// a flag never seems set and then vanishes. The system flags are left
/// to the server.
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

/// Parses a shared mailbox name into an IMAP Mailbox token.
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

/// The `SEARCH` keys of a shared filter, `ALL` when there is none, a
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

/// The `SORT` criteria of a shared sort, `REVERSE DATE` when there is
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

/// Translates a shared filter into an IMAP search key, its negations
/// pushed down to the single keys.
///
/// `NOT` then only ever applies to one non-composite key: iCloud reads
/// `NOT OR a b` as matching everything and `NOT NOT a` as `NOT a`, and
/// refuses `NOT (a b)`. The rewrite (De Morgan, double negations
/// cancelled) means the same on every server.
fn convert_filter(filter: &SearchEmailsFilterQuery) -> Result<SearchKey<'static>> {
    convert_filter_with(filter, false)
}

/// Translates `filter`, or its negation when `negated` holds.
fn convert_filter_with(
    filter: &SearchEmailsFilterQuery,
    negated: bool,
) -> Result<SearchKey<'static>> {
    use SearchEmailsFilterQuery as Q;

    let key = match filter {
        Q::Not(inner) => return convert_filter_with(inner, !negated),
        Q::And(left, right) if !negated => {
            return Ok(and_key(
                convert_filter_with(left, false)?,
                convert_filter_with(right, false)?,
            ));
        }
        Q::And(left, right) => {
            return Ok(SearchKey::Or(
                Box::new(convert_filter_with(left, true)?),
                Box::new(convert_filter_with(right, true)?),
            ));
        }
        Q::Or(left, right) if !negated => {
            return Ok(SearchKey::Or(
                Box::new(convert_filter_with(left, false)?),
                Box::new(convert_filter_with(right, false)?),
            ));
        }
        Q::Or(left, right) => {
            return Ok(and_key(
                convert_filter_with(left, true)?,
                convert_filter_with(right, true)?,
            ));
        }
        Q::Address(pattern) => return address_key(pattern, negated),

        Q::Date(date) => SearchKey::SentOn(imap_date(*date)?),

        // NOTE: the shared clause is strictly after, where SENTSINCE is on
        // or after, so the day is bumped.
        Q::AfterDate(date) => {
            let bumped = date.succ_opt().unwrap_or(*date);
            SearchKey::SentSince(imap_date(bumped)?)
        }

        // NOTE: SENTBEFORE is strictly earlier than the day, as the shared
        // clause is.
        Q::BeforeDate(date) => SearchKey::SentBefore(imap_date(*date)?),

        Q::From(pattern) => SearchKey::From(astring(pattern)?),
        Q::To(pattern) => SearchKey::To(astring(pattern)?),
        Q::Cc(pattern) => SearchKey::Cc(astring(pattern)?),
        Q::Bcc(pattern) => SearchKey::Bcc(astring(pattern)?),
        Q::Subject(pattern) => SearchKey::Subject(astring(pattern)?),
        Q::Body(pattern) => SearchKey::Body(astring(pattern)?),
        Q::MessageId(id) => SearchKey::Header(astring("Message-ID")?, astring(id)?),
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

/// Translates one shared sorter into an IMAP sort criterion.
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
        email::{
            flag::{Flag, IanaFlag},
            search::{filter::query::SearchEmailsFilterQuery, query::SearchEmailsQuery},
        },
        error::ErrorCode,
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
            search_line("not (from alice or from bob)"),
            "A UID SEARCH NOT FROM alice NOT FROM bob",
        );
        assert_eq!(search_line("not not from alice"), "A UID SEARCH FROM alice");
        assert_eq!(
            search_line("not (from alice and flag flagged)"),
            "A UID SEARCH OR NOT FROM alice NOT FLAGGED",
        );
        assert_eq!(
            search_line("not address eve"),
            "A UID SEARCH NOT FROM eve NOT TO eve NOT CC eve NOT BCC eve",
        );
        assert_eq!(
            search_line("address eve"),
            "A UID SEARCH OR FROM eve OR TO eve OR CC eve BCC eve",
        );
        assert_eq!(
            search_line("from a and not (subject b or not (to c and cc d))"),
            "A UID SEARCH FROM a NOT SUBJECT b TO c CC d",
        );
    }

    #[test]
    fn a_message_id_is_a_header_key_rechecked_on_the_root_chain() {
        assert_eq!(
            search_line("message-id <a@b.org> and from c"),
            "A UID SEARCH HEADER Message-ID a@b.org FROM c",
        );

        let query: SearchEmailsQuery = "message-id a@b.org and (message-id c@d.org or from e)"
            .parse()
            .unwrap();
        assert_eq!(root_message_ids(query.filter.as_ref()), ["a@b.org"]);
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
            search_line("flag important"),
            "A UID SEARCH KEYWORD $Important"
        );
        assert_eq!(search_line("flag junk"), "A UID SEARCH KEYWORD $Junk");
        assert_eq!(
            search_line("flag Forwarded"),
            "A UID SEARCH KEYWORD $Forwarded"
        );
        assert_eq!(search_line("flag custom"), "A UID SEARCH KEYWORD custom");
    }

    #[test]
    fn an_attachment_clause_is_refused_with_its_code() {
        let err = convert_filter(&SearchEmailsFilterQuery::Attachment).unwrap_err();

        assert_eq!(
            code_of::<ErrorCode>(&err),
            Some(ErrorCode::SearchUnsupported)
        );
        assert!(err.to_string().contains("`attachment`"), "{err}");
    }

    #[test]
    fn an_envelope_reads_its_header_strings_as_message_read_does() {
        // NOTE: an emoji split across two encoded words, and a name in
        // raw Windows-1252 bytes beside an encoded word.
        const LINE: &[u8] = b"* 1 FETCH (UID 7 ENVELOPE (NIL \
            \"=?UTF-8?Q?=F0=9F?= =?UTF-8?Q?=98=82?=\" \
            (({24}\r\n\x93Ann\x94 =?UTF-8?Q?=C3=A9?= NIL \"a\" \"example.org\")) \
            NIL NIL NIL NIL NIL NIL NIL))\r\n";

        let (_, response) = ResponseCodec::new().decode(LINE).unwrap();
        let Response::Data(Data::Fetch { items, .. }) = response else {
            panic!("not a FETCH answer");
        };
        let envelope = envelope_from(1, items.into_inner());

        assert_eq!(envelope.subject, "\u{1f602}");
        assert_eq!(
            envelope.from[0].name.as_deref(),
            Some("\u{201c}Ann\u{201d} \u{e9}")
        );
        assert_eq!(envelope.from[0].email, "a@example.org");
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

    /// The `BODYSTRUCTURE` answers of Dovecot 2.4.5 for the shared
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
            email::{
                flag::{Flag, FlagOp, IanaFlag},
                outcome::Outcome,
            },
            error::ErrorCode,
            imap::client::ImapClient,
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

        /// A `FETCH` answer holding a whole line, one whose
        /// `BODYSTRUCTURE` nests seven multiparts ([#801]), one whose
        /// `UID` does not decode and one that does not scan.
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

        /// A `CREATE` the server refuses naming the mailbox as existing,
        /// or refuses without a code while listing the mailbox, is
        /// `mailbox-duplicate`; any other refusal keeps its own code.
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

        /// Whether a trash delete removes says how the mailbox deletes,
        /// whatever it found: a UIDPLUS trash removes even when no id
        /// was found, so it never reads as flagged and asks for an
        /// expunge (#814).
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
            assert_eq!(counts, [(Some(0), Some(1)), (None, None)]);

            let [error] = outcome.errors.as_slice() else {
                panic!("{:?}", outcome.errors);
            };
            assert_eq!(error.id.as_deref(), Some("Locked"));
            assert_eq!(error.code, Some(ErrorCode::MailboxUnreadable));
        }

        /// A listing keeps the message of a line decoded in part, beside
        /// an item error naming the items that did not decode; a line
        /// whose UID failed, or that does not scan, names no message.
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

        use crate::email::mailbox::MailboxRole;

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
