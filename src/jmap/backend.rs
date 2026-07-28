//! JMAP adapter for the shared cross-protocol client.
//!
//! Thin glue over [`JmapClient`], which wraps io_jmap's high-level
//! client (`mailbox_get`, `email_query`, `email_get`, `email_set`,
//! `email_import`, `email_submission_set`, `blob_upload`,
//! `blob_download`). The shared `mailbox` argument is always a JMAP
//! mailbox id: the shared client maps human names to ids up front via
//! [`JmapClient::resolve_mailbox_id`], so these adapters never resolve
//! names themselves. The conversion is lifted from the retired io-email
//! JMAP drivers.

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
            import::JmapEmailImportArgs,
            query::{
                JmapEmailComparator, JmapEmailFilter, JmapEmailQueryOptions, JmapEmailSortProperty,
            },
            set::JmapEmailSetArgs,
        },
        email_submission::set::JmapEmailSubmissionCreate,
        identity::get::JmapIdentityGetOptions,
        mailbox::{JmapMailbox, JmapMailboxRole, get::JmapMailboxGetOptions},
    },
};
use url::Url;

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
        search::{
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        },
    },
    jmap::client::JmapClient,
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

    /// Searches envelopes in `mailbox` (a JMAP mailbox id) via
    /// `Email/query` + `Email/get`. Date clauses over-approximate on the
    /// wire (JMAP filters `receivedAt` while the shared DSL targets
    /// `sentAt`), so they are re-checked client-side and pagination then
    /// happens after the trim.
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
        })?;

        let mut envelopes: Vec<Envelope> = output.emails.into_iter().map(envelope_from).collect();
        if paginate_client_side {
            envelopes.retain(|envelope| post_match(envelope, &post_filters));
            envelopes = paginate(envelopes, page, page_size);
        }

        Ok(envelopes)
    }

    /// Adds, sets, or removes `flags` (JMAP keywords) on an email id
    /// set. `mailbox` is unused: JMAP keywords are global per email.
    pub fn store_flags(
        &mut self,
        _mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
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

        let output = self.email_set(args)?;
        bail_on_not_updated(output.not_updated)
    }

    /// Fetches one message's raw RFC 5322 bytes: `Email/get` for the
    /// blob id, then `Blob/download`.
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
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

        // JMAP has no side-effecting read (the blob download cannot carry a
        // mutation), so `--seen` is a separate `Email/set`.
        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.store_flags(mailbox, &[id], &[seen], FlagOp::Add)?;
        }

        Ok(raw)
    }

    /// Uploads `raw` as a blob then imports it into `mailbox` with the
    /// requested keywords. Returns the created email id.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
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
            "new".to_string(),
            JmapEmailImportArgs {
                blob_id,
                mailbox_ids,
                keywords,
                received_at: None,
            },
        );

        let output = self.email_import(imports)?;
        let email = output
            .created
            .get("new")
            .ok_or_else(|| anyhow!("Email/import did not create the imported email"))?;

        Ok(email.id.clone().unwrap_or_default())
    }

    /// Copies an email id set into `to` by adding `to`'s mailbox id.
    /// `from` is unused (existing `mailboxIds` carry the source).
    /// Returns the number of emails updated (any failure bails).
    pub fn copy_messages(&mut self, _from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let mut args = JmapEmailSetArgs::default();
        for id in ids {
            args.add_to_mailbox(id.to_string(), to.to_string());
        }

        let output = self.email_set(args)?;
        let count = output.updated.len();
        bail_on_not_updated(output.not_updated)?;
        Ok(count)
    }

    /// Moves an email id set from `from` to `to` in one `Email/set`.
    /// Returns the number of emails updated (any failure bails).
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let mut args = JmapEmailSetArgs::default();
        for id in ids {
            args.add_to_mailbox(id.to_string(), to.to_string());
            args.remove_from_mailbox(id.to_string(), from.to_string());
        }

        let output = self.email_set(args)?;
        let count = output.updated.len();
        bail_on_not_updated(output.not_updated)?;
        Ok(count)
    }

    /// Queues `raw` for delivery: upload, import into the drafts mailbox
    /// as `$draft`, then `EmailSubmission/set` under the sending
    /// identity. The identity and drafts mailbox come from the
    /// `identity_id` / `drafts_mailbox_id` config when set, otherwise
    /// they are discovered from the account (default identity and the
    /// `drafts`-role mailbox).
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let identity_id = self.resolve_identity_id()?;
        let drafts_id = self.resolve_drafts_mailbox_id()?;

        let blob_id = self.upload(raw)?;

        let mut mailbox_ids = BTreeMap::new();
        mailbox_ids.insert(drafts_id, true);
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
        let import = self.email_import(imports)?;
        let email = import
            .created
            .get("outgoing")
            .ok_or_else(|| anyhow!("Email/import did not stage the outgoing email"))?;
        let email_id = email
            .id
            .clone()
            .ok_or_else(|| anyhow!("Email/import response did not include an email id"))?;

        let mut submissions = BTreeMap::new();
        submissions.insert(
            "outgoing".to_string(),
            JmapEmailSubmissionCreate {
                identity_id,
                email_id,
                envelope: None,
            },
        );
        let output = self.email_submission_set(submissions)?;
        if !output.not_created.is_empty() {
            bail!("EmailSubmission/set did not submit the email");
        }

        Ok(())
    }

    /// Resolves the identity to submit under: the configured
    /// `identity_id`, else the account's default — the first identity
    /// `Identity/get` reports (servers list the primary first).
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

    /// Resolves the drafts mailbox to stage outgoing mail in: the
    /// configured `drafts_mailbox_id`, else the mailbox carrying the
    /// `drafts` role (`Mailbox/get`).
    fn resolve_drafts_mailbox_id(&mut self) -> Result<String> {
        if let Some(id) = self.config.drafts_mailbox_id.clone() {
            return Ok(id);
        }

        let output = self.mailbox_get(JmapMailboxGetOptions {
            ids: None,
            properties: None,
        })?;
        output
            .mailboxes
            .into_iter()
            .find(|mailbox| matches!(mailbox.role, Some(JmapMailboxRole::Drafts)))
            .and_then(|mailbox| mailbox.id)
            .ok_or_else(|| {
                anyhow!(
                    "JMAP send found no `drafts`-role mailbox; \
                     set `drafts_mailbox_id` in the account config"
                )
            })
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

/// Fails with the offending ids when an `Email/set` left some emails
/// un-updated.
fn bail_on_not_updated<E>(not_updated: BTreeMap<String, E>) -> Result<()> {
    if not_updated.is_empty() {
        return Ok(());
    }

    let ids: Vec<&str> = not_updated.keys().map(String::as_str).collect();
    bail!("JMAP Email/set failed for: {}", ids.join(", "))
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
        total: with_counts.then_some(u64::from(mailbox.total_emails)),
        unread: with_counts.then_some(u64::from(mailbox.unread_emails)),
    }
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
fn envelope_from(email: JmapEmail) -> Envelope {
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
    // NOTE: JMAP returns messageId as a list (RFC 5322 allows multiple
    // header instances); the first non-empty entry is canonical.
    let message_id = email
        .message_id
        .and_then(|ids| ids.into_iter().find_map(|s| normalize_message_id(&s)));

    Envelope {
        id,
        message_id,
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

fn parse_rfc3339(raw: &str) -> Option<DateTime<FixedOffset>> {
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
        .map(|filter| convert_filter(filter, &mut post_filters));

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

/// Recursively translates `node` into a JMAP filter tree; date leaves
/// push a [`PostFilter`] so the strict sentAt rule can be re-applied.
fn convert_filter(
    node: &SearchEmailsFilterQuery,
    post_filters: &mut Vec<PostFilter>,
) -> JmapFilter<JmapEmailFilter> {
    use SearchEmailsFilterQuery as Q;

    match node {
        Q::And(left, right) => JmapFilter::and(vec![
            convert_filter(left, post_filters),
            convert_filter(right, post_filters),
        ]),
        Q::Or(left, right) => JmapFilter::or(vec![
            convert_filter(left, post_filters),
            convert_filter(right, post_filters),
        ]),
        Q::Not(inner) => JmapFilter::not(vec![convert_filter(inner, post_filters)]),

        Q::From(pattern) => JmapFilter::Condition(JmapEmailFilter {
            from: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::To(pattern) => JmapFilter::Condition(JmapEmailFilter {
            to: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Subject(pattern) => JmapFilter::Condition(JmapEmailFilter {
            subject: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Body(pattern) => JmapFilter::Condition(JmapEmailFilter {
            body: Some(pattern.clone()),
            ..Default::default()
        }),
        Q::Flag(flag) => JmapFilter::Condition(JmapEmailFilter {
            has_keyword: Some(keyword_from(flag)),
            ..Default::default()
        }),

        // Over-approximate via after = start-of-day(D); the exact
        // sent-at rule is re-checked client-side.
        Q::Date(target) => {
            post_filters.push(PostFilter::Date(*target));
            JmapFilter::Condition(JmapEmailFilter {
                after: Some(utc_midnight(*target)),
                ..Default::default()
            })
        }
        // Over-approximate via after = start-of-day(D+1); the strict
        // sent-at rule is re-checked client-side.
        Q::AfterDate(target) => {
            post_filters.push(PostFilter::AfterDate(*target));
            let bumped = target.succ_opt().unwrap_or(*target);
            JmapFilter::Condition(JmapEmailFilter {
                after: Some(utc_midnight(bumped)),
                ..Default::default()
            })
        }
    }
}

/// True when `envelope` matches every residual predicate.
fn post_match(envelope: &Envelope, post_filters: &[PostFilter]) -> bool {
    post_filters.iter().all(|post_filter| match post_filter {
        PostFilter::Date(target) => envelope
            .date
            .map(|d| d.date_naive() == *target)
            .unwrap_or(false),
        PostFilter::AfterDate(target) => envelope
            .date
            .map(|d| d.date_naive() > *target)
            .unwrap_or(false),
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
