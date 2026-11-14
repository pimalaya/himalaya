//! # JMAP backend
//!
//! The JMAP adapter of the unified client, glue over the
//! io-jmap client [`JmapClient`] wraps.
//!
//! The shared mailbox argument is always an opaque JMAP id, the client
//! having mapped human names up front with
//! [`JmapClient::resolve_mailbox_id`], so nothing here resolves a name.

use std::collections::BTreeMap;

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate};
use io_jmap::{
    rfc8620::filter::JmapFilter,
    rfc8621::{
        JMAP_MAIL_CAPABILITY,
        email::{
            JmapEmail, JmapEmailAddress, JmapEmailProperty,
            get::JmapEmailGetOptions,
            import::{JmapEmailImportArgs, JmapEmailImportItemError, JmapEmailImportOutput},
            query::{
                JmapEmailComparator, JmapEmailFilter, JmapEmailQueryOptions, JmapEmailSortProperty,
            },
            set::{JmapEmailPatch, JmapEmailSetArgs, JmapEmailSetItemError, JmapEmailSetOutput},
        },
        email_submission::{
            JMAP_SUBMISSION_CAPABILITY,
            set::{JmapEmailSubmissionCreate, JmapEmailSubmissionSetArgs},
        },
        identity::get::JmapIdentityGetOptions,
        mailbox::{
            JmapMailbox, JmapMailboxRole,
            get::JmapMailboxGetOptions,
            set::{JmapMailboxCreate, JmapMailboxSetArgs, JmapMailboxUpdate},
        },
        vacation_response::{JMAP_VACATION_RESPONSE_CAPABILITY, set::JmapVacationResponseUpdate},
    },
};
use log::warn;
use pimalaya_cli::sanitize;
use url::Url;

use crate::{
    capability,
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole},
        search::{
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        },
        vacation::{Vacation, VacationPatch, format_date, normalize_date},
    },
    jmap::{client::JmapClient, error::format_set_error},
};

impl JmapClient {
    /// Lists every mailbox in the primary mail account. JMAP returns
    /// counts inline, surfaced only when `with_counts` is set.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let output = self.mailbox_get(JmapMailboxGetOptions {
            ids: None,
            properties: None,
        })?;

        Ok(output
            .mailboxes
            .into_iter()
            .map(|mailbox| mailbox_from(mailbox, with_counts))
            .collect())
    }

    /// Lists envelopes from `mailbox` (a JMAP mailbox id), batching
    /// `Email/query` + `Email/get` in one round-trip.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let (position, limit) = compute_position_limit(page, page_size);
        let filter = JmapEmailFilter {
            in_mailbox: Some(mailbox.to_string()),
            ..Default::default()
        };

        let output = self.email_query(JmapEmailQueryOptions {
            filter: Some(JmapFilter::from(filter)),
            position,
            limit,
            properties: Some(envelope_properties()),
            ..Default::default()
        })?;

        Ok(output.emails.into_iter().map(envelope_from).collect())
    }

    /// Searches a mailbox with an `Email/query` chained into an
    /// `Email/get`.
    ///
    /// A date clause over-approximates on the wire, JMAP filtering
    /// `receivedAt` where the shared query means `sentAt`, so it is
    /// rechecked client-side and the page is cut after that trim.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let Converted {
            filter,
            sort,
            post_filters,
        } = build(query, mailbox.to_string());

        let paginate_client_side = !post_filters.is_empty();
        let (position, limit) = if paginate_client_side {
            (None, None)
        } else {
            compute_position_limit(page, page_size)
        };

        let output = self.email_query(JmapEmailQueryOptions {
            filter: Some(filter),
            sort: Some(sort),
            position,
            limit,
            properties: Some(envelope_properties()),
            ..Default::default()
        })?;

        let mut envelopes: Vec<Envelope> = output.emails.into_iter().map(envelope_from).collect();
        if paginate_client_side {
            envelopes.retain(|envelope| post_match(envelope, &post_filters));
            envelopes = paginate(envelopes, page, page_size);
        }

        Ok(envelopes)
    }

    /// Adds, sets, or removes `flags` (JMAP keywords) on the ids of `ids`
    /// that `mailbox` holds, returning how many were updated.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<usize> {
        let found = self.emails_in(mailbox, ids)?;
        let ids: Vec<&str> = found.keys().map(String::as_str).collect();
        self.store_keywords(&ids, flags, op)
    }

    /// Writes `flags` as keywords on `ids`, wherever the emails are.
    fn store_keywords(&mut self, ids: &[&str], flags: &[Flag], op: FlagOp) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }

        let mut args = JmapEmailSetArgs::default();

        for id in ids {
            match op {
                FlagOp::Add => {
                    for flag in flags {
                        args.set_keyword(id.to_string(), keyword_from(flag));
                    }
                }
                FlagOp::Remove => {
                    for flag in flags {
                        args.unset_keyword(id.to_string(), keyword_from(flag));
                    }
                }
                FlagOp::Set => {
                    let map = flags
                        .iter()
                        .map(|flag| (keyword_from(flag), true))
                        .collect();
                    args.replace_keywords(id.to_string(), map);
                }
            }
        }

        count_written(self.email_set(args)?)
    }

    /// The emails of `ids` that `mailbox` holds, each with the mailboxes
    /// holding it (`Email/get` of `mailboxIds`). A JMAP id names an email
    /// account-wide, so this is what scopes a write to the mailbox given.
    fn emails_in(&mut self, mailbox: &str, ids: &[&str]) -> Result<EmailMailboxes> {
        if ids.is_empty() {
            return Ok(EmailMailboxes::new());
        }

        let output = self.email_get(
            ids.iter().map(ToString::to_string).collect(),
            JmapEmailGetOptions {
                properties: Some(vec![JmapEmailProperty::Id, JmapEmailProperty::MailboxIds]),
                ..Default::default()
            },
        )?;

        Ok(held_by(output.emails, mailbox))
    }

    /// Fetches one message's raw RFC 5322 bytes: `Email/get` for the
    /// blob id, then `Blob/download`. The id names the email account-wide,
    /// so `mailbox` is not checked.
    pub fn get_message(&mut self, _mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let output = self.email_get(
            vec![id.to_string()],
            JmapEmailGetOptions {
                properties: Some(vec![JmapEmailProperty::BlobId]),
                ..Default::default()
            },
        )?;

        let email = output
            .emails
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Email/get returned no email for the requested id"))?;
        let blob_id = email
            .blob_id
            .ok_or_else(|| anyhow!("Email/get response did not include a blobId"))?;

        let (template, account_id) = {
            let session = self
                .session()
                .ok_or_else(|| anyhow!("JMAP session is missing"))?;
            (session.download_url.clone(), account_id_of(session))
        };
        let url_str = template
            .replace("{accountId}", &account_id)
            .replace("{blobId}", &blob_id)
            .replace("{type}", "message%2Frfc822")
            .replace("{name}", "message.eml");
        let url = Url::parse(&url_str)
            .map_err(|_| anyhow!("Resolved JMAP download URL is invalid: {url_str}"))?;

        let raw = self.download_blob(&url)?;

        // NOTE: a blob download carries no mutation, so `--seen` costs a
        // separate `Email/set` here.
        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.store_keywords(&[id], &[seen], FlagOp::Add)?;
        }

        Ok(raw)
    }

    /// Uploads `raw` as a blob then imports it into `mailbox` with the
    /// requested keywords. Returns the created email id, when the server
    /// reports one.
    ///
    /// A server forbidding duplicates (Fastmail) refuses an import of
    /// bytes the account holds with `alreadyExists` (RFC 8621 section
    /// 4.8). An email being in several mailboxes at once, the existing one
    /// is then filed into `mailbox` with the keywords, and its id
    /// returned: the message ends up in the mailbox, as asked.
    pub fn add_message(
        &mut self,
        mailbox: &str,
        flags: &[Flag],
        raw: Vec<u8>,
    ) -> Result<Option<String>> {
        let blob_id = self.upload(raw)?;

        let mut mailbox_ids = BTreeMap::new();
        mailbox_ids.insert(mailbox.to_string(), true);
        let keywords = (!flags.is_empty()).then(|| {
            flags
                .iter()
                .map(|flag| (keyword_from(flag), true))
                .collect()
        });

        let mut imports = BTreeMap::new();
        imports.insert(
            CREATED.to_string(),
            JmapEmailImportArgs {
                blob_id,
                mailbox_ids,
                keywords,
                received_at: None,
            },
        );

        let existing_id = match imported(self.email_import(imports)?, CREATED)? {
            Imported::Created(id) => return Ok(id),
            Imported::Existing(id) => id,
        };

        let mut args = JmapEmailSetArgs::default();
        args.add_to_mailbox(existing_id.clone(), mailbox.to_string());
        for flag in flags {
            args.set_keyword(existing_id.clone(), keyword_from(flag));
        }

        if count_written(self.email_set(args)?)? == 0 {
            bail!("JMAP server holds the message as `{existing_id}` but could not file it");
        }

        Ok(Some(existing_id))
    }

    /// Copies the ids of `ids` that `from` holds into `to`, adding `to`
    /// to their mailboxes, and returns how many were copied.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let found = self.emails_in(from, ids)?;
        if found.is_empty() {
            return Ok(0);
        }

        let mut args = JmapEmailSetArgs::default();
        for id in found.keys() {
            args.add_to_mailbox(id.clone(), to.to_string());
        }

        count_written(self.email_set(args)?)
    }

    /// Moves the ids of `ids` that `from` holds to `to` in one
    /// `Email/set`, and returns how many were moved.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let found = self.emails_in(from, ids)?;

        // NOTE: the patch removing `from` would win over the one adding
        // `to`, taking the emails out of the very mailbox they stay in.
        if found.is_empty() || from == to {
            return Ok(found.len());
        }

        let mut args = JmapEmailSetArgs::default();
        for id in found.keys() {
            args.add_to_mailbox(id.clone(), to.to_string());
            args.remove_from_mailbox(id.clone(), from.to_string());
        }

        count_written(self.email_set(args)?)
    }

    /// Removes the ids of `ids` that `mailbox` holds from it, and returns
    /// how many were removed: an email held there alone is destroyed, one
    /// another mailbox holds too only leaves `mailbox`.
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<usize> {
        let found = self.emails_in(mailbox, ids)?;
        if found.is_empty() {
            return Ok(0);
        }

        count_written(self.email_set(removal(mailbox, found))?)
    }

    /// Creates `name` at the top or under `parent` (`Mailbox/set`
    /// create), returning the new mailbox's id.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        let mailbox = JmapMailboxCreate {
            name: Some(name.to_string()),
            parent_id: parent.map(ToString::to_string),
            ..Default::default()
        };

        let args = JmapMailboxSetArgs {
            create: Some(BTreeMap::from([(String::from(CREATED), mailbox)])),
            ..Default::default()
        };

        let mut output = self.mailbox_set(args)?;
        self.forget_mailboxes();

        if let Some(err) = output.not_created.get(CREATED) {
            bail!(
                "Create JMAP mailbox `{name}` error{}",
                format_set_error(err)
            );
        }

        output
            .created
            .remove(CREATED)
            .and_then(|mailbox| mailbox.id)
            .ok_or_else(|| anyhow!("JMAP server created mailbox `{name}` without naming its id"))
    }

    /// Renames `mailbox` to `name`, its parent kept (`Mailbox/set`
    /// update), returning its id, which a rename leaves as it is.
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<String> {
        let patch = JmapMailboxUpdate {
            name: Some(name.to_string()),
            ..Default::default()
        };

        let args = JmapMailboxSetArgs {
            update: Some(BTreeMap::from([(mailbox.to_string(), patch)])),
            ..Default::default()
        };

        let output = self.mailbox_set(args)?;
        self.forget_mailboxes();

        if let Some(err) = output.not_updated.get(mailbox) {
            bail!(
                "Rename JMAP mailbox `{mailbox}` error{}",
                format_set_error(err)
            );
        }

        Ok(mailbox.to_string())
    }

    /// Deletes `mailbox` with the messages it holds (`Mailbox/set` destroy
    /// with `onDestroyRemoveEmails`), a message also in another mailbox
    /// staying there (RFC 8621 section 2.5).
    pub fn delete_mailbox(&mut self, mailbox: &str) -> Result<()> {
        let args = JmapMailboxSetArgs {
            destroy: Some(vec![mailbox.to_string()]),
            on_destroy_remove_emails: Some(true),
            ..Default::default()
        };

        let output = self.mailbox_set(args)?;
        self.forget_mailboxes();

        if let Some(err) = output.not_destroyed.get(mailbox) {
            bail!(
                "Delete JMAP mailbox `{mailbox}` error{}",
                format_set_error(err)
            );
        }

        Ok(())
    }

    /// Reads the auto-reply (`VacationResponse/get`), the default one
    /// when the server holds none.
    ///
    /// A server whose session lacks `urn:ietf:params:jmap:vacationresponse`
    /// (RFC 8621 section 8) is refused under `capability-missing`: the
    /// dynamic tier of `vacation`.
    pub fn get_vacation(&mut self) -> Result<Vacation> {
        self.require_vacation()?;

        let Some(vacation) = self.vacation_response_get()? else {
            return Ok(Vacation::default());
        };

        Ok(Vacation {
            enabled: vacation.is_enabled,
            from: vacation.from_date.map(normalize_date),
            to: vacation.to_date.map(normalize_date),
            subject: vacation.subject,
            text: vacation.text_body,
            html: vacation.html_body,
        })
    }

    /// Changes what `patch` names of the auto-reply
    /// (`VacationResponse/set`), returning it as it then stands.
    pub fn set_vacation(&mut self, patch: VacationPatch) -> Result<Vacation> {
        self.require_vacation()?;

        let update = JmapVacationResponseUpdate {
            is_enabled: patch.enabled,
            from_date: patch.from.map(|date| date.map(format_date)),
            to_date: patch.to.map(|date| date.map(format_date)),
            subject: patch.subject,
            text_body: patch.text,
            html_body: patch.html,
        };

        // NOTE: the server echoes only what changed, so the whole response
        // is read again.
        self.vacation_response_set(update)?;
        self.get_vacation()
    }

    /// Refuses the auto-reply on a server not advertising it.
    fn require_vacation(&mut self) -> Result<()> {
        let served = self.session().is_some_and(|session| {
            session
                .capabilities
                .contains_key(JMAP_VACATION_RESPONSE_CAPABILITY)
        });

        if !served {
            return Err(capability::missing(
                "Cannot `vacation` over jmap: the server does not advertise \
                 `urn:ietf:params:jmap:vacationresponse`",
            ));
        }

        Ok(())
    }

    /// Queues a message for delivery: upload, import as a draft, then
    /// `EmailSubmission/set` under the sending identity.
    ///
    /// The identity and the mailboxes come from the configuration when it
    /// names them, and from the account's default identity and the
    /// `drafts`- and `sent`-role mailboxes otherwise. Once submitted, the
    /// server moves the email to the sent mailbox, unsets `$draft` and sets
    /// `$seen` (RFC 8621 section 7.5 `onSuccessUpdateEmail`), so it does
    /// not stay behind as a draft.
    ///
    /// A server whose session lacks `urn:ietf:params:jmap:submission`
    /// (RFC 8621 section 7) is refused under `capability-missing` before
    /// the upload, so no draft is left behind: the dynamic tier of
    /// `submit`.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let submits = self.session().is_some_and(|session| {
            session
                .capabilities
                .contains_key(JMAP_SUBMISSION_CAPABILITY)
        });

        if !submits {
            return Err(capability::missing(
                "Cannot `submit` over jmap: the server does not advertise \
                 `urn:ietf:params:jmap:submission`; send over SMTP with `--backend smtp`",
            ));
        }

        let identity_id = self.resolve_identity_id()?;
        let (drafts_id, sent_id) = self.resolve_send_mailbox_ids()?;

        let blob_id = self.upload(raw)?;

        let mut mailbox_ids = BTreeMap::new();
        mailbox_ids.insert(drafts_id.clone(), true);
        let mut keywords = BTreeMap::new();
        keywords.insert("$draft".to_string(), true);

        let mut imports = BTreeMap::new();
        imports.insert(
            "outgoing".to_string(),
            JmapEmailImportArgs {
                blob_id,
                mailbox_ids,
                keywords: Some(keywords),
                received_at: None,
            },
        );
        // NOTE: bytes the account holds already (a draft added earlier, a
        // retry) are refused with `alreadyExists` by a server forbidding
        // duplicates; the existing email is then the one submitted.
        let email_id = match imported(self.email_import(imports)?, "outgoing")? {
            Imported::Created(Some(id)) | Imported::Existing(id) => id,
            Imported::Created(None) => bail!("Email/import response did not include an email id"),
        };

        let mut submissions = BTreeMap::new();
        submissions.insert(
            "outgoing".to_string(),
            JmapEmailSubmissionCreate {
                identity_id,
                email_id,
                envelope: None,
            },
        );

        let mut sent = JmapEmailPatch::default()
            .unset_keyword("$draft")
            .set_keyword("$seen");
        if let Some(sent_id) = sent_id {
            sent = sent.remove_from_mailbox(drafts_id).add_to_mailbox(sent_id);
        }

        let output = self.email_submission_set(JmapEmailSubmissionSetArgs {
            create: submissions,
            on_success_update_email: Some(BTreeMap::from([("#outgoing".to_string(), sent)])),
            on_success_destroy_email: None,
        })?;
        if let Some(err) = output.not_created.values().next() {
            bail!("Submit JMAP message error{}", format_set_error(err));
        }

        // NOTE: the message went out whatever the Email/set answers.
        match output.email_set {
            Some(Err(err)) => {
                let err = err.to_string();
                warn!(
                    "message sent, but it stays a draft: {}",
                    sanitize::text(&err)
                );
            }
            Some(Ok(set)) => {
                for err in set.not_updated.values() {
                    let err = format_set_error(err);
                    warn!("message sent, but it stays a draft{err}");
                }
            }
            None => {}
        }

        Ok(())
    }

    /// Resolves the identity to submit under: the configured
    /// `identity_id`, or the first one `Identity/get` reports, servers
    /// listing the primary identity first.
    fn resolve_identity_id(&mut self) -> Result<String> {
        if let Some(id) = self.config.identity_id.clone() {
            return Ok(id);
        }

        let output = self.identity_get(JmapIdentityGetOptions { ids: None })?;
        output
            .identities
            .into_iter()
            .next()
            .map(|identity| identity.id)
            .ok_or_else(|| {
                anyhow!("JMAP send found no identity; set `identity_id` in the account config")
            })
    }

    /// Resolves the mailbox outgoing mail is staged in and the one it is
    /// filed under once sent: the configured `drafts_mailbox_id` and
    /// `sent_mailbox_id`, else the mailboxes carrying the `drafts` and
    /// `sent` roles, read with one `Mailbox/get`.
    ///
    /// Only the drafts mailbox is required, a server without a sent one
    /// leaving the sent message where it was staged.
    fn resolve_send_mailbox_ids(&mut self) -> Result<(String, Option<String>)> {
        let mut drafts = self.config.drafts_mailbox_id.clone();
        let mut sent = self.config.sent_mailbox_id.clone();

        if drafts.is_none() || sent.is_none() {
            let output = self.mailbox_get(JmapMailboxGetOptions {
                ids: None,
                properties: None,
            })?;

            for mailbox in output.mailboxes {
                match mailbox.role {
                    Some(JmapMailboxRole::Drafts) if drafts.is_none() => drafts = mailbox.id,
                    Some(JmapMailboxRole::Sent) if sent.is_none() => sent = mailbox.id,
                    _ => {}
                }
            }
        }

        let drafts = drafts.ok_or_else(|| {
            anyhow!(
                "JMAP send found no `drafts`-role mailbox; \
                 set `drafts_mailbox_id` in the account config"
            )
        })?;

        Ok((drafts, sent))
    }

    /// Uploads `raw` as a `message/rfc822` blob, returning its blob id.
    fn upload(&mut self, raw: Vec<u8>) -> Result<String> {
        let (template, account_id) = {
            let session = self
                .session()
                .ok_or_else(|| anyhow!("JMAP session is missing"))?;
            (session.upload_url.clone(), account_id_of(session))
        };
        let url_str = template.replace("{accountId}", &account_id);
        let url = Url::parse(&url_str)
            .map_err(|_| anyhow!("Resolved JMAP upload URL is invalid: {url_str}"))?;

        Ok(self.blob_upload(&url, "message/rfc822", raw)?.blob_id)
    }
}

/// What an `Email/import` of one message gave.
#[derive(Debug, PartialEq)]
enum Imported {
    /// The email it created, with its id when the server named it.
    Created(Option<String>),
    /// The email holding the same bytes already, which a server forbidding
    /// duplicates names on `alreadyExists` (RFC 8621 section 4.8).
    Existing(String),
}

/// Reads the outcome of the import keyed `key`, failing on any refusal
/// but `alreadyExists`.
fn imported(mut output: JmapEmailImportOutput, key: &str) -> Result<Imported> {
    match output.not_created.remove(key) {
        None => output
            .created
            .remove(key)
            .map(|email| Imported::Created(email.id))
            .ok_or_else(|| anyhow!("Email/import did not create the imported email")),
        Some(JmapEmailImportItemError::AlreadyExists { existing_id, .. }) => {
            Ok(Imported::Existing(existing_id))
        }
        Some(err) => bail!("Import JMAP message error{}", format_set_error(&err)),
    }
}

/// The mailboxes holding each email, keyed by email id.
type EmailMailboxes = BTreeMap<String, BTreeMap<String, bool>>;

/// Keeps the emails `mailbox` holds, each with the mailboxes holding it.
fn held_by(emails: Vec<JmapEmail>, mailbox: &str) -> EmailMailboxes {
    emails
        .into_iter()
        .filter_map(|email| {
            let mailboxes = email.mailbox_ids.unwrap_or_default();
            if !mailboxes.get(mailbox).copied().unwrap_or(false) {
                return None;
            }
            Some((email.id?, mailboxes))
        })
        .collect()
}

/// The `Email/set` taking `found` out of `mailbox`: an email no other
/// mailbox holds is destroyed, the others only lose `mailbox`.
fn removal(mailbox: &str, found: EmailMailboxes) -> JmapEmailSetArgs {
    let mut args = JmapEmailSetArgs::default();

    for (id, mailboxes) in found {
        let elsewhere = mailboxes
            .iter()
            .any(|(other, held)| *held && other != mailbox);

        if elsewhere {
            args.remove_from_mailbox(id, mailbox.to_string());
        } else {
            args.destroy(id);
        }
    }

    args
}

/// Counts the emails an `Email/set` updated or destroyed. One gone since
/// it was looked up (`notFound`) counts as not found; any other refusal
/// fails, saying how many landed beside it.
fn count_written(output: JmapEmailSetOutput) -> Result<usize> {
    let count = output.updated.len() + output.destroyed.len();

    let refused: Vec<String> = output
        .not_updated
        .iter()
        .chain(&output.not_destroyed)
        .filter(|(_, err)| !matches!(err, JmapEmailSetItemError::NotFound { .. }))
        .map(|(id, err)| format!("`{id}`{}", format_set_error(err)))
        .collect();

    if !refused.is_empty() {
        bail!(
            "Write JMAP messages error ({count} written): {}",
            refused.join("; ")
        );
    }

    Ok(count)
}

/// Primary mail account id; empty when the session has none.
fn account_id_of(session: &io_jmap::rfc8620::session::JmapSession) -> String {
    session
        .primary_accounts
        .get(JMAP_MAIL_CAPABILITY)
        .cloned()
        .unwrap_or_default()
}

/// Converts a JMAP mailbox into the shared [`Mailbox`] shape.
fn mailbox_from(mailbox: JmapMailbox, with_counts: bool) -> Mailbox {
    Mailbox {
        id: mailbox.id.unwrap_or_default(),
        name: mailbox.name.unwrap_or_default(),
        role: mailbox
            .role
            .map(|role| MailboxRole::parse(&role.to_string())),
        total: with_counts.then_some(u64::from(mailbox.total_emails)),
        unread: with_counts.then_some(u64::from(mailbox.unread_emails)),
    }
}

/// A shared [`Flag`] as JMAP writes it, spelled as its keyword.
pub(crate) fn written_flag(flag: &Flag) -> Flag {
    Flag::from_raw(keyword_from(flag))
}

/// Maps a shared [`Flag`] to its JMAP keyword (RFC 8621 §4.1.1).
fn keyword_from(flag: &Flag) -> String {
    match flag.iana() {
        Some(IanaFlag::Seen) => "$seen".into(),
        Some(IanaFlag::Answered) => "$answered".into(),
        Some(IanaFlag::Flagged) => "$flagged".into(),
        Some(IanaFlag::Draft) => "$draft".into(),
        Some(IanaFlag::Deleted) => "$deleted".into(),
        Some(IanaFlag::Forwarded) => "$forwarded".into(),
        Some(IanaFlag::Junk) => "$junk".into(),
        Some(IanaFlag::NotJunk) => "$notjunk".into(),
        Some(IanaFlag::Phishing) => "$phishing".into(),
        Some(IanaFlag::Important) => "$important".into(),
        Some(IanaFlag::MdnSent) => "$mdnsent".into(),
        None => flag.raw().to_string(),
    }
}

/// `Email/get` properties for an [`Envelope`]; uses `sentAt`
/// (author-claimed Date:) for cross-backend consistency.
fn envelope_properties() -> Vec<JmapEmailProperty> {
    vec![
        JmapEmailProperty::Id,
        JmapEmailProperty::Keywords,
        JmapEmailProperty::Subject,
        JmapEmailProperty::From,
        JmapEmailProperty::To,
        JmapEmailProperty::SentAt,
        JmapEmailProperty::Size,
        JmapEmailProperty::HasAttachment,
        JmapEmailProperty::MessageId,
        JmapEmailProperty::InReplyTo,
    ]
}

/// Translates 1-indexed `(page, page_size)` to JMAP `(position, limit)`.
fn compute_position_limit(page: Option<u32>, page_size: Option<u32>) -> (Option<u64>, Option<u64>) {
    let Some(size) = page_size else {
        return (None, None);
    };
    let page = page.unwrap_or(1).max(1);
    let position = u64::from(page - 1).saturating_mul(u64::from(size));
    (Some(position), Some(u64::from(size)))
}

/// Folds a JMAP email object into the shared [`Envelope`] shape.
pub(crate) fn envelope_from(email: JmapEmail) -> Envelope {
    let id = email.id.unwrap_or_default();
    let flags = email
        .keywords
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(keyword, set)| set.then(|| Flag::from_raw(keyword)))
        .collect();
    let subject = email.subject.unwrap_or_default();
    let from = email
        .from
        .unwrap_or_default()
        .into_iter()
        .map(address_from)
        .collect();
    let to = email
        .to
        .unwrap_or_default()
        .into_iter()
        .map(address_from)
        .collect();
    let date = email.sent_at.as_deref().and_then(parse_rfc3339);
    let size = email.size.unwrap_or(0);
    let has_attachment = email.has_attachment;
    // NOTE: JMAP returns messageId as a list, RFC 5322 allowing several
    // header instances, and the first non-empty entry is canonical.
    let message_id = email
        .message_id
        .and_then(|ids| ids.into_iter().find_map(|s| normalize_message_id(&s)));
    // NOTE: JMAP hands the ids over already stripped of their brackets,
    // but normalising anyway keeps them comparable with a server that
    // does not.
    let in_reply_to = email
        .in_reply_to
        .unwrap_or_default()
        .iter()
        .filter_map(|id| normalize_message_id(id))
        .collect();

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

fn address_from(addr: JmapEmailAddress) -> Address {
    Address {
        name: addr.name,
        email: addr.email,
    }
}

pub(crate) fn parse_rfc3339(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(trimmed).ok()
}

/// Residual client-side predicate left after the JMAP filter ran.
enum PostFilter {
    Date(NaiveDate),
    AfterDate(NaiveDate),
    BeforeDate(NaiveDate),
}

/// JMAP-side filter + sort, plus the residual client-side predicates.
struct Converted {
    filter: JmapFilter<JmapEmailFilter>,
    sort: Vec<JmapEmailComparator>,
    post_filters: Vec<PostFilter>,
}

/// Converts `query` into JMAP primitives, AND-scoped to `mailbox_id`.
fn build(query: Option<&SearchEmailsQuery>, mailbox_id: String) -> Converted {
    let mailbox_scope = JmapFilter::Condition(JmapEmailFilter {
        in_mailbox: Some(mailbox_id),
        ..Default::default()
    });

    let mut post_filters = Vec::new();
    let user_filter = query
        .and_then(|q| q.filter.as_ref())
        .map(|filter| convert_filter(filter, Some(&mut post_filters)));

    let filter = match user_filter {
        Some(user_filter) => JmapFilter::and(vec![mailbox_scope, user_filter]),
        None => mailbox_scope,
    };

    let sort = query
        .and_then(|q| q.sort.as_deref())
        .filter(|chain| !chain.is_empty())
        .map(|chain| chain.iter().map(convert_sorter).collect())
        .unwrap_or_else(|| vec![sent_at_desc()]);

    Converted {
        filter,
        sort,
        post_filters,
    }
}

/// Recursively translates `node` into a JMAP filter tree.
///
/// JMAP dates read the received-at, the shared clauses the sent-at. On the
/// chain of `and` from the root, `post_filters` is given: a date leaf
/// over-approximates on the wire and pushes a [`PostFilter`] re-checking
/// the sent-at exactly, every residual having to hold. Under `or` or `not`
/// a residual cannot be ANDed onto the whole result, so a date leaf there
/// reads the received-at day alone.
fn convert_filter(
    node: &SearchEmailsFilterQuery,
    mut post_filters: Option<&mut Vec<PostFilter>>,
) -> JmapFilter<JmapEmailFilter> {
    use SearchEmailsFilterQuery as Q;

    match node {
        Q::And(left, right) => JmapFilter::and(vec![
            convert_filter(left, post_filters.as_deref_mut()),
            convert_filter(right, post_filters),
        ]),
        Q::Or(left, right) => JmapFilter::or(vec![
            convert_filter(left, None),
            convert_filter(right, None),
        ]),
        Q::Not(inner) => JmapFilter::not(vec![convert_filter(inner, None)]),

        Q::From(pattern) => condition(JmapEmailFilter {
            from: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::To(pattern) => condition(JmapEmailFilter {
            to: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Cc(pattern) => condition(JmapEmailFilter {
            cc: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Bcc(pattern) => condition(JmapEmailFilter {
            bcc: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Address(pattern) => JmapFilter::or(vec![
            condition(JmapEmailFilter {
                from: Some(pattern.clone()),
                ..Default::default()
            }),
            condition(JmapEmailFilter {
                to: Some(pattern.clone()),
                ..Default::default()
            }),
            condition(JmapEmailFilter {
                cc: Some(pattern.clone()),
                ..Default::default()
            }),
            condition(JmapEmailFilter {
                bcc: Some(pattern.clone()),
                ..Default::default()
            }),
        ]),
        Q::Subject(pattern) => condition(JmapEmailFilter {
            subject: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Body(pattern) => condition(JmapEmailFilter {
            body: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Flag(flag) => condition(JmapEmailFilter {
            has_keyword: Some(keyword_from(flag)),
            ..Default::default()
        }),
        Q::Attachment => condition(JmapEmailFilter {
            has_attachment: Some(true),
            ..Default::default()
        }),
        // NOTE: JMAP declares `message-id` `none`, refused before connecting,
        // so this never runs; were it reached, it matches nothing.
        Q::MessageId(_) => JmapFilter::not(vec![condition(JmapEmailFilter::default())]),

        // NOTE: a message is received no earlier than it is sent, so the
        // start of the day bounds the received-at from below.
        Q::Date(target) => match post_filters {
            Some(post_filters) => {
                post_filters.push(PostFilter::Date(*target));
                condition(JmapEmailFilter {
                    after: Some(utc_midnight(*target)),
                    ..Default::default()
                })
            }
            None => condition(JmapEmailFilter {
                after: Some(utc_midnight(*target)),
                before: Some(utc_midnight(next_day(*target))),
                ..Default::default()
            }),
        },
        Q::AfterDate(target) => {
            if let Some(post_filters) = post_filters {
                post_filters.push(PostFilter::AfterDate(*target));
            }
            condition(JmapEmailFilter {
                after: Some(utc_midnight(next_day(*target))),
                ..Default::default()
            })
        }
        // NOTE: a message sent before the day may be received any time
        // after, so nothing bounds the received-at and the sent-at is
        // checked client-side alone.
        Q::BeforeDate(target) => match post_filters {
            Some(post_filters) => {
                post_filters.push(PostFilter::BeforeDate(*target));
                condition(JmapEmailFilter::default())
            }
            None => condition(JmapEmailFilter {
                before: Some(utc_midnight(*target)),
                ..Default::default()
            }),
        },
    }
}

/// Wraps one filter condition.
fn condition(filter: JmapEmailFilter) -> JmapFilter<JmapEmailFilter> {
    JmapFilter::Condition(filter)
}

/// The day after `date`, `date` itself at the end of the calendar.
fn next_day(date: NaiveDate) -> NaiveDate {
    date.succ_opt().unwrap_or(date)
}

/// True when `envelope` matches every residual predicate.
fn post_match(envelope: &Envelope, post_filters: &[PostFilter]) -> bool {
    post_filters.iter().all(|post_filter| {
        let Some(sent) = envelope.date.map(|date| date.date_naive()) else {
            return false;
        };

        match post_filter {
            PostFilter::Date(target) => sent == *target,
            PostFilter::AfterDate(target) => sent > *target,
            PostFilter::BeforeDate(target) => sent < *target,
        }
    })
}

fn sent_at_desc() -> JmapEmailComparator {
    JmapEmailComparator {
        property: JmapEmailSortProperty::SentAt,
        is_ascending: Some(false),
        collation: None,
        keyword: None,
    }
}

fn convert_sorter(sorter: &SearchEmailsSorter) -> JmapEmailComparator {
    let SearchEmailsSorter(kind, order) = sorter;

    let property = match kind {
        SearchEmailsSorterKind::Date => JmapEmailSortProperty::SentAt,
        SearchEmailsSorterKind::From => JmapEmailSortProperty::From,
        SearchEmailsSorterKind::To => JmapEmailSortProperty::To,
        SearchEmailsSorterKind::Subject => JmapEmailSortProperty::Subject,
    };

    JmapEmailComparator {
        property,
        is_ascending: Some(!matches!(order, SearchEmailsSorterOrder::Descending)),
        collation: None,
        keyword: None,
    }
}

fn paginate(envelopes: Vec<Envelope>, page: Option<u32>, page_size: Option<u32>) -> Vec<Envelope> {
    let total = envelopes.len();
    let size = page_size.map(|n| n as usize);
    let start = ((page.unwrap_or(1).max(1) - 1) as usize).saturating_mul(size.unwrap_or(0));

    if start >= total {
        return Vec::new();
    }

    let end = match size {
        Some(n) => start.saturating_add(n).min(total),
        None => total,
    };

    envelopes[start..end].to_vec()
}

fn utc_midnight(date: NaiveDate) -> String {
    format!(
        "{:04}-{:02}-{:02}T00:00:00Z",
        date.year(),
        date.month(),
        date.day()
    )
}

/// The creation id of a mailbox `Mailbox/set` creates, which its answer
/// is keyed by.
const CREATED: &str = "new";

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::{
        email::{envelope::Envelope, search::query::SearchEmailsQuery},
        jmap::backend::*,
    };

    fn sent_on(date: &str) -> Envelope {
        Envelope {
            id: String::from("1"),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: Default::default(),
            subject: String::new(),
            from: Vec::new(),
            to: Vec::new(),
            date: DateTime::parse_from_rfc3339(date).ok(),
            size: 0,
            has_attachment: None,
        }
    }

    fn rechecked(query: &str, envelope: &Envelope) -> bool {
        let query: SearchEmailsQuery = query.parse().unwrap();
        let converted = build(Some(&query), String::from("inbox"));
        post_match(envelope, &converted.post_filters)
    }

    #[test]
    fn a_negated_date_clause_is_not_rechecked_as_a_positive_one() {
        let december = sent_on("2025-12-01T10:00:00+00:00");

        assert!(rechecked("not after 2026-01-01", &december));
        assert!(rechecked("date 2026-01-01 or subject x", &december));
    }

    fn email(id: &str, mailboxes: &[&str]) -> JmapEmail {
        let mailbox_ids: BTreeMap<&str, bool> = mailboxes.iter().map(|m| (*m, true)).collect();
        serde_json::from_value(serde_json::json!({ "id": id, "mailboxIds": mailbox_ids })).unwrap()
    }

    fn set_output(updated: &[&str], not_updated: &[(&str, &str)]) -> JmapEmailSetOutput {
        JmapEmailSetOutput {
            new_state: String::new(),
            created: BTreeMap::new(),
            updated: updated.iter().map(|id| (id.to_string(), None)).collect(),
            destroyed: Vec::new(),
            not_created: BTreeMap::new(),
            not_updated: not_updated
                .iter()
                .map(|(id, kind)| {
                    let err = serde_json::from_value(serde_json::json!({ "type": kind })).unwrap();
                    (id.to_string(), err)
                })
                .collect(),
            not_destroyed: BTreeMap::new(),
            keep_alive: true,
        }
    }

    fn import_output(not_created: serde_json::Value) -> JmapEmailImportOutput {
        JmapEmailImportOutput {
            new_state: String::new(),
            created: BTreeMap::new(),
            not_created: BTreeMap::from([(
                CREATED.to_string(),
                serde_json::from_value(not_created).unwrap(),
            )]),
            keep_alive: true,
        }
    }

    #[test]
    fn an_id_another_mailbox_holds_is_not_in_the_mailbox() {
        let emails = vec![
            email("E1", &["A"]),
            email("E2", &["B"]),
            email("E3", &["A", "B"]),
        ];

        let found = held_by(emails, "A");

        assert_eq!(found.keys().collect::<Vec<_>>(), ["E1", "E3"]);
    }

    #[test]
    fn a_trash_removal_destroys_only_what_the_trash_alone_holds() {
        let found = held_by(vec![email("E1", &["T"]), email("E2", &["T", "B"])], "T");

        let args = serde_json::to_value(removal("T", found)).unwrap();

        assert_eq!(args["destroy"], serde_json::json!(["E1"]));
        assert_eq!(
            args["update"],
            serde_json::json!({ "E2": { "mailboxIds/T": null } })
        );
    }

    #[test]
    fn an_id_gone_beside_an_updated_one_counts_as_not_found() {
        let output = set_output(&["E1"], &[("99999999", "notFound")]);

        assert_eq!(count_written(output).unwrap(), 1);
    }

    #[test]
    fn a_refusal_beside_an_update_fails_saying_what_landed() {
        let output = set_output(&["E1"], &[("E2", "tooManyKeywords")]);

        let err = count_written(output).unwrap_err().to_string();

        assert_eq!(
            err,
            "Write JMAP messages error (1 written): `E2`: tooManyKeywords"
        );
    }

    #[test]
    fn a_duplicate_import_names_the_existing_email() {
        let output = import_output(serde_json::json!({
            "type": "alreadyExists",
            "existingId": "E1",
        }));

        assert_eq!(
            imported(output, CREATED).unwrap(),
            Imported::Existing(String::from("E1"))
        );
    }

    #[test]
    fn an_import_refusal_names_its_type() {
        let output = import_output(serde_json::json!({
            "type": "overQuota",
            "description": "Mailbox full.",
        }));

        let err = imported(output, CREATED).unwrap_err().to_string();

        assert_eq!(err, "Import JMAP message error: overQuota: Mailbox full");
    }

    #[test]
    fn a_receipt_names_the_keyword_written() {
        let flag = written_flag(&Flag::from_iana(IanaFlag::Flagged));

        assert_eq!(flag.raw(), "$flagged");
        assert_eq!(flag.iana(), Some(IanaFlag::Flagged));
    }
}
