//! # pimdir index
//!
//! The query client of the store's search index (pimdir SEARCH), which
//! the `pimdir search`, `count`, `thread` and `tag` commands ask in the
//! SEARCH query language, scoped to the account's mail.
//!
//! The index belongs to the store's owner, whose sync engine builds and
//! refreshes it. Himalaya only reads it, so an answer is the store as of
//! the last refresh, without queued writes, and says how far it lags. An
//! index absent or of another version is a coded error: run the sync.

use std::{collections::BTreeMap, fmt};

use anyhow::{Context, Result, bail};
use ariadne::{Config, Label, Report, ReportKind, Source};
use chrono::{DateTime, FixedOffset, Local};
use io_pimdir::{
    client::search::{
        PimdirIndex, PimdirSearchClock, PimdirSearchCursor, PimdirSearchHit, PimdirSearchMatched,
        PimdirSearchOptions, PimdirSearchThread,
    },
    search::{
        coverage::{PimdirSearchCoverage, PimdirSearchTemporal, PimdirSearchUnread},
        query::{PimdirQuery, PimdirQueryError, PimdirQueryErrorCode, PimdirQueryKind},
        tag::{PimdirSearchTagPlacement, PimdirSearchTagSource},
    },
};
use jiff::tz::TimeZone;
use log::warn;
use pimalaya_cli::{error::CodedError, listing::Listing};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    error::{ErrorCode, ItemError, pimdir_error},
    pimdir::{
        backend::{envelope, envelope_from_item},
        client::PimdirClient,
        mailbox::list::span,
    },
    unified::email::{envelope::Envelope, flag::Flag},
};

impl PimdirClient {
    /// One page of the messages `query` matches in the account, ordered
    /// and paged by `options`.
    pub fn search_mail(
        &self,
        query: PimdirQuery,
        options: &PimdirSearchOptions,
    ) -> Result<PimdirMailPage> {
        let index = self.index()?;
        let answer = index
            .search(&self.scope(query), options)
            .map_err(pimdir_error)
            .context("Search the pimdir index")?;

        let mut errors = Vec::new();
        let hits = self.mail_hits(&index, answer.hits, &mut errors);

        Ok(PimdirMailPage {
            hits,
            next: answer.next,
            coverage: PimdirMailCoverage::from(&answer.coverage),
            errors,
        })
    }

    /// One page of the threads holding the messages `query` matches in
    /// the account (pimdir SEARCH §9), paged by thread.
    pub fn search_mail_threads(
        &self,
        query: PimdirQuery,
        options: &PimdirSearchOptions,
    ) -> Result<PimdirMailThreads> {
        let index = self.index()?;
        let answer = index
            .threads(&self.scope(query), options)
            .map_err(pimdir_error)
            .context("Search the pimdir index by thread")?;

        let mut errors = Vec::new();
        let mut threads = Vec::with_capacity(answer.threads.len());

        for thread in answer.threads {
            threads.push(PimdirMailThread {
                hits: self.mail_hits(&index, thread.hits, &mut errors),
                thread: thread.thread,
                count: thread.count,
                first_date: thread.first,
                last_date: thread.last,
            });
        }

        Ok(PimdirMailThreads {
            threads,
            next: answer.next,
            coverage: PimdirMailCoverage::from(&answer.coverage),
            errors,
        })
    }

    /// How many messages `query` matches in the account, its dates read
    /// by `clock`.
    pub fn count_mail(
        &self,
        query: PimdirQuery,
        clock: &PimdirSearchClock,
    ) -> Result<(u64, PimdirMailCoverage)> {
        let answer = self
            .index()?
            .count(&self.scope(query), clock)
            .map_err(pimdir_error)
            .context("Count in the pimdir index")?;
        Ok((answer.count, PimdirMailCoverage::from(&answer.coverage)))
    }

    /// The thread of the message `id`, its members oldest first, a message
    /// no thread holds being its own.
    pub fn mail_thread(&self, id: &str) -> Result<PimdirMailThread> {
        let Ok(seq) = id.parse::<i64>() else {
            bail!("Message id `{id}` is not a pimdir id: it takes the number a listing prints");
        };
        let index = self.index()?;
        let thread = index
            .thread(self.account.as_deref(), seq)
            .map_err(pimdir_error)
            .with_context(|| format!("Read the thread of message `{id}`"))?;

        match thread {
            Some(thread)
                if thread
                    .hits
                    .iter()
                    .all(|hit| hit.kind == PimdirQueryKind::Mail.as_str()) =>
            {
                self.mail_thread_of(&index, thread)
            }
            Some(_) => bail!("Item `{id}` is not a message"),
            None => bail!(
                "Message `{id}` is not in the search index: it is not in this account, \
                 or the sync engine has not indexed it yet"
            ),
        }
    }

    /// Plans adding `add` and removing `remove` on every message `query`
    /// matches, and stages the plan unless `dry_run`.
    ///
    /// Every action passes the capability gate before the first is
    /// appended (io-pimdir `PimdirSearchTagPlan::stage`), so a refusal
    /// stages nothing.
    pub fn tag_mail(
        &self,
        query: PimdirQuery,
        add: &[String],
        remove: &[String],
        dry_run: bool,
        clock: &PimdirSearchClock,
    ) -> Result<PimdirMailTag> {
        let plan = self
            .index()?
            .tag(&self.scope(query), add, remove, None, clock)
            .map_err(pimdir_error)
            .context("Plan the pimdir tag")?;

        let stageable = |placement: &PimdirSearchTagPlacement| {
            placement.changes()
                && placement.missing.is_empty()
                && matches!(placement.source, PimdirSearchTagSource::Resolved(_))
        };
        let staging = plan
            .hits
            .iter()
            .any(|hit| hit.placements.iter().any(stageable));

        let mut rows = BTreeMap::new();
        if staging && !dry_run {
            let mut producer = self.producer()?;
            for action in plan
                .stage(&mut producer)
                .map_err(pimdir_error)
                .context("Stage the pimdir tag")?
            {
                rows.insert((action.collection, action.seq), action.action);
            }
        }

        let mut tagged = Vec::new();
        for hit in &plan.hits {
            for placement in &hit.placements {
                let row = rows.get(&(placement.collection.clone(), hit.seq)).cloned();
                let status = match (&placement.next, &placement.source, &row) {
                    (_, _, Some(_)) => PimdirTagStatus::Queued,
                    (None, _, _) => PimdirTagStatus::UnknownFlags,
                    (Some(_), _, _) if !placement.changes() => PimdirTagStatus::Unchanged,
                    (_, PimdirSearchTagSource::Ambiguous(_), _) => PimdirTagStatus::AmbiguousSource,
                    (_, PimdirSearchTagSource::Unsynced(_), _) => PimdirTagStatus::Unsynced,
                    _ if !placement.missing.is_empty() => PimdirTagStatus::Unsupported,
                    _ => PimdirTagStatus::Planned,
                };
                let flags = match &placement.next {
                    Some(next) => next.iter().map(Flag::from_raw).collect(),
                    None => Vec::new(),
                };
                let subject = Some(hit.title.clone()).filter(|title| !title.is_empty());
                tagged.push(PimdirTagged {
                    id: hit.seq.to_string(),
                    mailbox: placement.collection.clone(),
                    subject,
                    flags,
                    status,
                    row: row.as_ref().map(|handle| handle.id),
                    handle: row.map(|handle| handle.to_string()),
                });
            }
        }

        Ok(PimdirMailTag {
            tagged,
            coverage: PimdirMailCoverage::from(&plan.coverage),
        })
    }

    /// Opens the store's search index read-only (io-pimdir
    /// `PimdirIndex::open_query`), never building nor refreshing it.
    fn index(&self) -> Result<PimdirIndex> {
        PimdirIndex::open_query(&self.root)
            .map_err(pimdir_error)
            .with_context(|| format!("Open the pimdir search index of `{}`", self.root.display()))
    }

    /// Scopes `query` to `kind:mail` and the named `account:` through
    /// `PimdirQuery::scoped`, where the index reads date range and coverage.
    fn scope(&self, query: PimdirQuery) -> PimdirQuery {
        query.scoped(Some(PimdirQueryKind::Mail), self.account.as_deref(), None)
    }

    /// The hits as Himalaya lists them, each unreadable one pushed to
    /// `errors` (`message-unreadable` unless the store gives a code).
    fn mail_hits(
        &self,
        index: &PimdirIndex,
        hits: Vec<PimdirSearchHit>,
        errors: &mut Vec<ItemError>,
    ) -> Vec<PimdirMailHit> {
        let mut read = Vec::with_capacity(hits.len());

        for hit in hits {
            let id = hit.seq.to_string();

            match self.mail_hit(index, hit) {
                Ok(hit) => read.push(hit),
                Err(err) => {
                    let mut error = ItemError::new(Some(&id), &err);
                    error.code.get_or_insert(ErrorCode::MessageUnreadable);
                    errors.push(error);
                }
            }
        }

        read
    }

    /// A hit as Himalaya lists it: the envelope of its first placement,
    /// read from the store as it stands, and the mailboxes holding it.
    fn mail_hit(&self, index: &PimdirIndex, hit: PimdirSearchHit) -> Result<PimdirMailHit> {
        let Some(first) = hit.placements.first() else {
            bail!("Message `{}` has no mailbox in the search index", hit.seq);
        };

        let item = index
            .reader()
            .get_item(&first.collection, hit.seq)
            .map_err(pimdir_error)
            .with_context(|| format!("Read message `{}`", hit.seq))?;
        let envelope = match item {
            Some(item) => envelope_from_item(&item),
            None => {
                let mut envelope = envelope(hit.seq.to_string(), &first.flags, None);
                envelope.subject = Some(hit.title).filter(|title| !title.is_empty());
                envelope
            }
        };

        Ok(PimdirMailHit {
            envelope,
            mailboxes: hit
                .placements
                .into_iter()
                .map(|placement| placement.collection)
                .collect(),
            matched: hit.matched.map(PimdirMailMatched::from),
            snippet: hit.snippet,
        })
    }

    /// A thread of the index as Himalaya lists it.
    fn mail_thread_of(
        &self,
        index: &PimdirIndex,
        thread: PimdirSearchThread,
    ) -> Result<PimdirMailThread> {
        let mut hits = Vec::with_capacity(thread.hits.len());
        for hit in thread.hits {
            hits.push(self.mail_hit(index, hit)?);
        }
        Ok(PimdirMailThread {
            thread: thread.thread,
            count: thread.count,
            first_date: thread.first,
            last_date: thread.last,
            hits,
        })
    }
}

/// The clock of a query: now, in the user's zone (pimdir SEARCH §8.1),
/// `TZ` then the system's, UTC when it has no IANA name.
pub fn local_clock() -> PimdirSearchClock {
    let zone = TimeZone::try_system()
        .ok()
        .and_then(|zone| zone.iana_name().map(ToOwned::to_owned));
    if zone.is_none() {
        warn!("local time zone has no IANA name, reading query dates in UTC; set TZ to one");
    }

    PimdirSearchClock { now: None, zone }
}

/// Parses the words of a query, joined by spaces, as the pimdir SEARCH
/// query language (§8.3).
///
/// A refused query (§8.4) is `query-refused`, drawn over the query in
/// plain output; in JSON its message names the §8.4 code, the span in
/// Unicode scalar values and what was expected there.
pub fn parse_query(words: &[String], json: bool) -> Result<PimdirQuery> {
    let query = words.join(" ");
    PimdirQuery::parse(&query).map_err(|err| {
        let message = if json {
            refusal_line(&err)
        } else {
            refusal_report(&query, &err)
        };
        CodedError::new(ErrorCode::QueryRefused, message).into()
    })
}

/// A refusal as one line: its code, span and message, then what was
/// expected.
fn refusal_line(err: &PimdirQueryError) -> String {
    let mut line = format!(
        "Query refused, {} at {}..{}: {}",
        err.code.as_str(),
        err.start,
        err.end,
        err.message
    );
    if !err.expected.is_empty() {
        line.push_str(&format!("; expected {}", err.expected.join(", ")));
    }
    line
}

/// A refusal drawn over the query with ariadne, without colour, the string
/// landing in a JSON error as often as on a terminal.
fn refusal_report(query: &str, err: &PimdirQueryError) -> String {
    let name = "query";
    let config = Config::default().with_color(false);
    let end = err.end.max(err.start + 1);
    let mut report = Report::build(ReportKind::Error, (name, err.start..end))
        .with_config(config)
        .with_code(err.code.as_str())
        .with_message("Query refused")
        .with_label(Label::new((name, err.start..end)).with_message(&err.message));
    if !err.expected.is_empty() {
        report = report.with_note(format!("expected {}", err.expected.join(", ")));
    }
    if err.code == PimdirQueryErrorCode::UnknownField {
        report = report.with_help(format!("fields: {}", err.fields.join(", ")));
    }

    let mut buf = Vec::new();
    let _ = report.finish().write((name, Source::from(query)), &mut buf);

    String::from_utf8_lossy(&buf).replacen("Error: ", "", 1)
}

/// One page of hits.
pub struct PimdirMailPage {
    /// The hits, in the order asked.
    pub hits: Vec<PimdirMailHit>,
    /// Where the next page starts, `None` on the last one.
    pub next: Option<PimdirSearchCursor>,
    /// What the answer could not see.
    pub coverage: PimdirMailCoverage,
    /// The error of each hit the store could not read.
    pub errors: Vec<ItemError>,
}

/// One page of threads.
pub struct PimdirMailThreads {
    /// The threads, in the order of their first hit.
    pub threads: Vec<PimdirMailThread>,
    /// Where the next page starts, `None` on the last one.
    pub next: Option<PimdirSearchCursor>,
    /// What the answer could not see.
    pub coverage: PimdirMailCoverage,
    /// The error of each hit the store could not read.
    pub errors: Vec<ItemError>,
}

/// What a tag planned or staged, and what it could not see.
pub struct PimdirMailTag {
    /// One entry per placement of each hit, in date order.
    pub tagged: Vec<PimdirTagged>,
    /// What the query could not see: a tag reaches only the hits the
    /// index answers.
    pub coverage: PimdirMailCoverage,
}

/// A message the index matched.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailHit {
    /// The envelope, as the store holds it now; its `id` is what `message
    /// read` takes, with one of the mailboxes.
    #[serde(flatten)]
    pub envelope: Envelope,
    /// The account's mailboxes holding the message.
    pub mailboxes: Vec<String>,
    /// What a text term matched, `null` when only field terms did.
    pub matched: Option<PimdirMailMatched>,
    /// A passage of the body, from the first part naming a query word;
    /// `null` without an indexed body.
    pub snippet: Option<String>,
}

/// What a text term matched (pimdir SEARCH §8.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PimdirMailMatched {
    /// The local body.
    Body,
    /// The headers of a message whose body is not local.
    Summary,
}

impl From<PimdirSearchMatched> for PimdirMailMatched {
    fn from(matched: PimdirSearchMatched) -> Self {
        match matched {
            PimdirSearchMatched::Body => Self::Body,
            PimdirSearchMatched::Summary => Self::Summary,
        }
    }
}

/// A thread of the account (pimdir SEARCH §9).
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailThread {
    /// The thread id, as `thread:` takes it; `null` for a message no
    /// thread holds.
    pub thread: Option<String>,
    /// The messages it holds in the account, matched or not.
    pub count: u64,
    /// The date of its earliest message, `null` without one.
    pub first_date: Option<String>,
    /// The date of its latest message.
    pub last_date: Option<String>,
    /// Its messages the query matched, or every one of a thread read whole.
    pub hits: Vec<PimdirMailHit>,
}

/// One placement a tag reached.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirTagged {
    /// The message id.
    pub id: String,
    /// The mailbox of the placement.
    pub mailbox: String,
    /// The subject, `null` when the message has none.
    pub subject: Option<String>,
    /// The flags the placement carries once the tag applies, empty when
    /// its flags were never read.
    pub flags: Vec<Flag>,
    /// What the tag does there.
    pub status: PimdirTagStatus,
    /// The queue row the change waits in, `null` when none was staged.
    pub row: Option<i64>,
    /// The row's handle, its id under the store's identity (pimdir STORAGE
    /// §4.5), `null` when none was staged.
    pub handle: Option<String>,
}

/// What a tag does to one placement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PimdirTagStatus {
    /// The change would be staged (`--dry-run`).
    Planned,
    /// The change is staged, waiting in the queue.
    Queued,
    /// The placement carries the flags already: nothing is staged.
    Unchanged,
    /// Its flags were never synced, so no set can replace them.
    UnknownFlags,
    /// Several sources sync the mailbox: nothing is staged.
    AmbiguousSource,
    /// No source syncs the mailbox: nothing is staged.
    Unsynced,
    /// The source syncing the mailbox does not declare the change.
    Unsupported,
}

impl fmt::Display for PimdirTagStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Planned => "planned",
            Self::Queued => "queued",
            Self::Unchanged => "unchanged",
            Self::UnknownFlags => "flags-unknown",
            Self::AmbiguousSource => "several-sources",
            Self::Unsynced => "not-synced",
            Self::Unsupported => "unsupported",
        })
    }
}

/// What an answer could not see (pimdir SEARCH §8.2), over the account's
/// mail.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailCoverage {
    /// Whether the answer is the whole answer: every other field at rest.
    pub complete: bool,
    /// Messages in scope whose body is not local, matched on their
    /// headers alone, counted whatever the query reads.
    pub bodiless: u64,
    /// Messages in scope whose body the indexer could not read, indexed
    /// from their headers, counted whatever the query reads.
    pub missing: u64,
    /// Messages in scope whose body no parser accepted, indexed from
    /// their headers, counted whatever the query reads.
    pub unparseable: u64,
    /// Messages in scope whose body is encrypted, indexed by their
    /// headers alone, counted whatever the query reads.
    pub encrypted: u64,
    /// Messages in scope whose last change the index has not folded in.
    pub unindexed: u64,
    /// How many changes the store drew since the index was refreshed,
    /// store-wide: the sync engine refreshes it.
    pub behind: u64,
    /// The instant below which the store is not known to hold every
    /// message of the range asked, `null` when that is not the cause.
    pub since: Option<String>,
    /// The instant from which it is not known to.
    pub until: Option<String>,
    /// Whether a first sync is still listing a mailbox in scope.
    pub filling: bool,
    /// The counts a term of the query reads, which leave the answer
    /// incomplete, as io-pimdir's `unread_counted` answers them.
    #[serde(skip)]
    pub counted: PimdirSearchUnread,
}

impl From<&PimdirSearchCoverage> for PimdirMailCoverage {
    fn from(coverage: &PimdirSearchCoverage) -> Self {
        let (since, until, filling) = match &coverage.temporal {
            PimdirSearchTemporal::Full => (None, None, false),
            PimdirSearchTemporal::Bounded {
                since,
                until,
                filling,
            } => (since.clone(), until.clone(), *filling),
        };

        let unread = |count: fn(&PimdirSearchUnread) -> u64| -> u64 {
            coverage.local.kinds.values().map(count).sum()
        };

        Self {
            complete: coverage.is_complete(),
            bodiless: unread(|unread| unread.bodiless),
            missing: unread(|unread| unread.missing),
            unparseable: unread(|unread| unread.unparseable),
            encrypted: unread(|unread| unread.encrypted),
            unindexed: coverage.local.unindexed,
            behind: coverage.local.behind,
            since,
            until,
            filling,
            counted: coverage.unread_counted(),
        }
    }
}

impl PimdirMailCoverage {
    /// What the answer could not see, `None` when it is complete.
    pub fn summary(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.counted.bodiless > 0 {
            parts.push(format!(
                "{} message(s) have no local body and matched on their headers alone",
                self.counted.bodiless
            ));
        }
        if self.counted.missing > 0 {
            parts.push(format!(
                "{} message(s) have a body the index could not read and matched on their \
                 headers alone",
                self.counted.missing
            ));
        }
        if self.counted.unparseable > 0 {
            parts.push(format!(
                "{} message(s) have a body the index could not parse and matched on their \
                 headers alone",
                self.counted.unparseable
            ));
        }
        if self.counted.encrypted > 0 {
            parts.push(format!(
                "{} message(s) are encrypted and matched on their headers alone",
                self.counted.encrypted
            ));
        }
        if let Some(since) = &self.since {
            parts.push(format!("mail {} is not synced", span(None, Some(since))));
        }
        if let Some(until) = &self.until {
            parts.push(format!("mail {} is not synced", span(Some(until), None)));
        }
        if self.filling {
            parts.push(String::from("a first sync is still listing a mailbox"));
        }
        if self.unindexed > 0 {
            parts.push(format!(
                "{} message(s) changed since the index was refreshed",
                self.unindexed
            ));
        }
        if self.behind > 0 {
            parts.push(format!(
                "the index is {} change(s) behind the store, which the sync engine refreshes",
                self.behind
            ));
        }

        (!parts.is_empty()).then(|| parts.join("; "))
    }

    /// Adds the `Incomplete:` field above the items of `listing`, when the
    /// answer is not whole.
    pub fn note(&self, listing: &mut Listing) {
        if let Some(summary) = self.summary() {
            listing.header.push(("Incomplete", summary));
        }
    }
}

/// The line under a count, naming what the answer could not see; nothing
/// when it is complete.
impl fmt::Display for PimdirMailCoverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.summary() {
            Some(summary) => writeln!(f, "Incomplete: {summary}"),
            None => Ok(()),
        }
    }
}

/// How the plain output writes its lists, from the account's settings.
#[derive(Clone, Debug)]
pub struct PimdirListStyle {
    /// The chrono `strftime` format of a date.
    pub datetime_fmt: String,
    /// Whether a date is converted to the local timezone first.
    pub datetime_local_tz: bool,
}

impl PimdirListStyle {
    /// The style `account` configures.
    pub fn new(account: &Account) -> Self {
        Self {
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
        }
    }

    /// A date in the account's format, nothing without one.
    pub fn date(&self, date: Option<DateTime<FixedOffset>>) -> String {
        match date {
            None => String::new(),
            Some(date) if self.datetime_local_tz => date
                .with_timezone(&Local)
                .format(&self.datetime_fmt)
                .to_string(),
            Some(date) => date.format(&self.datetime_fmt).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use io_pimdir::{
        change::PimdirWriteOp,
        client::{PimdirStore, search::PimdirIndexRefreshOptions},
        codec::PimdirAction,
        collection::{PimdirCheckpoint, PimdirCollectionId, PimdirScope},
        placement::{
            PimdirBase, PimdirFlags, PimdirHandle, PimdirLevel, PimdirLinkId, PimdirPlacement,
            PimdirStatus,
        },
        summary::{PimdirSummary, mail::PimdirMailSummary},
    };

    use crate::{
        config::pimdir::PimdirConfig,
        unified::email::flag::{Flag, IanaFlag},
    };

    use super::*;

    const INBOX: &str = "imap/INBOX";

    /// A mail of the store: its Message-ID local part, its headers past
    /// the identity, and its body.
    fn mail(id: &str, headers: &str, body: &str) -> Vec<u8> {
        format!("Message-ID: <{id}@x.org>\r\n{headers}\r\n\r\n{body}\r\n").into_bytes()
    }

    /// An indexed `imap/INBOX` holding `a`, its reply `b`, a flagged `c`
    /// and a bodiless `d`, and the client reading it.
    fn indexed_store() -> (tempfile::TempDir, PimdirClient) {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = PimdirStore::open(dir.path()).unwrap().for_source("imap");
        owner.ensure_collection(INBOX, "message/rfc822").unwrap();
        owner
            .write(vec![PimdirWriteOp::SetCheckpoint {
                collection: PimdirCollectionId(INBOX.into()),
                checkpoint: PimdirCheckpoint(Vec::new()),
            }])
            .unwrap();
        drop(owner);

        let mut client = client(dir.path());
        let mails = [
            (
                mail(
                    "a",
                    "From: Alice <alice@x.org>\r\nTo: bob@y.org\r\n\
                     Date: Thu, 14 May 2026 10:00:00 +0000\r\nSubject: Quarterly report",
                    "The quarterly numbers are attached.",
                ),
                Vec::new(),
            ),
            (
                mail(
                    "b",
                    "In-Reply-To: <a@x.org>\r\nFrom: bob@y.org\r\nTo: alice@x.org\r\n\
                     Date: Fri, 15 May 2026 10:00:00 +0000\r\nSubject: Re: Quarterly report",
                    "Thanks, looks good.",
                ),
                Vec::new(),
            ),
            (
                mail(
                    "c",
                    "From: carol@z.org\r\nTo: bob@y.org\r\n\
                     Date: Sat, 16 May 2026 10:00:00 +0000\r\nSubject: Lunch",
                    "Pizza on friday?",
                ),
                vec![Flag::from_iana(IanaFlag::Flagged)],
            ),
        ];
        for (raw, flags) in mails {
            client.add_message(INBOX, &flags, raw).unwrap();
        }
        let mut owner = PimdirStore::open(dir.path()).unwrap().for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 3);
        let flags = PimdirFlags::Unknown;
        owner
            .write(vec![PimdirWriteOp::UpsertPlacement(PimdirPlacement {
                collection: PimdirCollectionId(INBOX.into()),
                handle: PimdirHandle("d".into()),
                link_id: Some(PimdirLinkId("d@x.org".into())),
                object: None,
                level: PimdirLevel::Meta,
                summary: Some(PimdirSummary::Mail(PimdirMailSummary {
                    message_id: Some("d@x.org".into()),
                    subject: "Invoice".into(),
                    date: Some("2026-05-17T10:00:00Z".into()),
                    ..Default::default()
                })),
                server_object_id: None,
                sort_key: "2026-05-17T10:00:00Z".into(),
                flags: flags.clone(),
                status: PimdirStatus::Clean,
                conflict_revision: None,
                conflict_object: None,
                base: Some(PimdirBase {
                    flags,
                    revision: None,
                    object: None,
                }),
                origin: None,
            })])
            .unwrap();
        drop(owner);
        refresh(dir.path());

        (dir, client)
    }

    fn client(dir: &std::path::Path) -> PimdirClient {
        PimdirClient::new(PimdirConfig {
            root: dir.to_path_buf(),
            account: None,
        })
        .unwrap()
    }

    /// Builds or refreshes the index, as the store's owner does.
    fn refresh(dir: &std::path::Path) {
        PimdirIndex::open(dir)
            .unwrap()
            .refresh(&PimdirIndexRefreshOptions::default())
            .unwrap();
    }

    fn query(text: &str) -> PimdirQuery {
        parse_query(&[text.to_string()], true).unwrap()
    }

    fn page(client: &PimdirClient, text: &str) -> PimdirMailPage {
        let options = PimdirSearchOptions::default();
        client.search_mail(query(text), &options).unwrap()
    }

    /// The subjects of the hits of `text`.
    fn subjects(client: &PimdirClient, text: &str) -> Vec<String> {
        page(client, text)
            .hits
            .into_iter()
            .map(|hit| hit.envelope.subject.unwrap_or_default())
            .collect()
    }

    fn id_of(client: &PimdirClient, subject: &str) -> String {
        page(client, &format!("subject:\"{subject}\""))
            .hits
            .into_iter()
            .find(|hit| hit.envelope.subject.as_deref() == Some(subject))
            .unwrap()
            .envelope
            .id
    }

    #[test]
    fn a_search_answers_from_the_index() {
        let (_dir, client) = indexed_store();

        let numbers = page(&client, "body:numbers");
        assert_eq!(numbers.hits.len(), 1);
        let hit = &numbers.hits[0];
        assert_eq!(hit.envelope.subject.as_deref(), Some("Quarterly report"));
        assert_eq!(hit.mailboxes, [INBOX]);
        assert_eq!(hit.matched, Some(PimdirMailMatched::Body));
        assert!(hit.snippet.as_deref().unwrap().contains("numbers"));
        assert_eq!(hit.envelope.from[0].email, "alice@x.org");

        assert_eq!(subjects(&client, "subject:lunch"), ["Lunch"]);
        assert_eq!(subjects(&client, "from:alice@x.org"), ["Quarterly report"]);
        assert_eq!(
            subjects(&client, "to:alice@x.org"),
            ["Re: Quarterly report"]
        );
        assert_eq!(subjects(&client, "is:flagged"), ["Lunch"]);
        assert_eq!(
            subjects(&client, "date:2026-05-15"),
            ["Re: Quarterly report"]
        );
        assert_eq!(
            subjects(&client, "date:2026-05-15..2026-05-16"),
            ["Lunch", "Re: Quarterly report"]
        );
        assert_eq!(
            subjects(&client, "quarterly -from:bob@y.org"),
            ["Quarterly report"]
        );

        let invoice = page(&client, "invoice");
        assert_eq!(invoice.hits.len(), 1);
        assert_eq!(invoice.hits[0].matched, Some(PimdirMailMatched::Summary));
        assert_eq!(invoice.coverage.bodiless, 1);
        assert!(!invoice.coverage.complete);
    }

    #[test]
    fn a_page_names_where_the_next_starts() {
        let (_dir, client) = indexed_store();
        let options = |after| PimdirSearchOptions {
            limit: 3,
            after,
            ..Default::default()
        };

        let first = client
            .search_mail(query("date:2026-05-01.."), &options(None))
            .unwrap();
        assert_eq!(first.hits.len(), 3);
        assert!(first.next.is_some());
        let last = client
            .search_mail(query("date:2026-05-01.."), &options(first.next))
            .unwrap();
        assert_eq!(last.hits.len(), 1);
        assert!(last.next.is_none());
        assert_eq!(
            client
                .count_mail(query("date:2026-05-01.."), &PimdirSearchClock::default())
                .unwrap()
                .0,
            4
        );
    }

    /// An unreadable hit is a `message-unreadable` item error, the other
    /// hits still listed, by message as by thread.
    #[test]
    fn a_hit_the_store_cannot_read_is_an_item_error() {
        let (dir, client) = indexed_store();
        let lunch = id_of(&client, "Lunch");

        let conn = rusqlite::Connection::open(dir.path().join("pimdir.db")).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = ON")
            .unwrap();
        let seq: i64 = lunch.parse().unwrap();
        let sql = "UPDATE summary SET size = 'x' \
                   WHERE item = (SELECT id FROM items WHERE seq = ?1)";
        assert_eq!(conn.execute(sql, [seq]).unwrap(), 1);
        drop(conn);

        let found = page(&client, "date:2026-05-01..");
        let subjects: Vec<&str> = found
            .hits
            .iter()
            .map(|hit| hit.envelope.subject.as_deref().unwrap_or_default())
            .collect();
        assert_eq!(
            subjects,
            ["Invoice", "Re: Quarterly report", "Quarterly report"]
        );
        let [error] = found.errors.as_slice() else {
            panic!("{:?}", found.errors);
        };
        assert_eq!(error.id.as_deref(), Some(lunch.as_str()));
        assert_eq!(error.code, Some(ErrorCode::MessageUnreadable));

        let threads = client
            .search_mail_threads(query("lunch"), &PimdirSearchOptions::default())
            .unwrap();
        assert_eq!(threads.threads.len(), 1);
        assert!(threads.threads[0].hits.is_empty());
        assert_eq!(threads.errors.len(), 1, "{:?}", threads.errors);
    }

    #[test]
    fn a_day_is_read_in_the_zone_the_clock_states() {
        let (dir, mut client) = indexed_store();
        client
            .add_message(
                INBOX,
                &[],
                mail(
                    "e",
                    "From: eve@x.org\r\nDate: Sat, 07 Nov 2026 00:30:00 +0100\r\nSubject: Offsite",
                    "Agenda inside.",
                ),
            )
            .unwrap();
        let mut owner = PimdirStore::open(dir.path()).unwrap().for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 1);
        drop(owner);
        refresh(dir.path());

        let clock = PimdirSearchClock {
            now: Some(1_794_034_800),
            zone: Some("Europe/Paris".into()),
        };
        let options = PimdirSearchOptions {
            clock: clock.clone(),
            ..Default::default()
        };
        let subjects = |text| {
            client
                .search_mail(query(text), &options)
                .unwrap()
                .hits
                .into_iter()
                .map(|hit| hit.envelope.subject.unwrap_or_default())
                .collect::<Vec<_>>()
        };

        assert_eq!(subjects("date:2026-11-07"), ["Offsite"]);
        assert!(subjects("date:2026-11-06").is_empty());
        assert_eq!(subjects("date:today"), ["Offsite"]);
        let count = |text| client.count_mail(query(text), &clock).unwrap().0;
        assert_eq!(count("date:2026-11-07"), 1);
        assert_eq!(count("date:yesterday"), 0);
    }

    #[test]
    fn a_queued_message_is_not_found_until_indexed() {
        let (dir, mut client) = indexed_store();
        client
            .add_message(
                INBOX,
                &[],
                mail("e", "From: eve@x.org\r\nSubject: Offsite", "Agenda inside."),
            )
            .unwrap();

        let agenda = page(&client, "agenda");
        assert!(agenda.hits.is_empty(), "a queued add is listed");
        assert_eq!(agenda.coverage.behind, 0);

        let mut owner = PimdirStore::open(dir.path()).unwrap().for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 1);
        drop(owner);
        let agenda = page(&client, "agenda");
        assert!(agenda.hits.is_empty(), "an unindexed add is listed");
        assert!(agenda.coverage.behind > 0);
        assert!(!agenda.coverage.complete);
        assert!(
            agenda.coverage.to_string().contains("behind the store"),
            "{}",
            agenda.coverage
        );

        refresh(dir.path());
        assert_eq!(subjects(&client, "agenda"), ["Offsite"]);
    }

    #[test]
    fn a_reply_joins_its_parents_thread() {
        let (_dir, client) = indexed_store();

        let threads = client
            .search_mail_threads(query("quarterly"), &PimdirSearchOptions::default())
            .unwrap();
        assert_eq!(threads.threads.len(), 1);
        assert_eq!(threads.threads[0].count, 2);

        let reply = id_of(&client, "Re: Quarterly report");
        let thread = client.mail_thread(&reply).unwrap();
        let subjects: Vec<&str> = thread
            .hits
            .iter()
            .map(|hit| hit.envelope.subject.as_deref().unwrap_or_default())
            .collect();
        assert_eq!(subjects, ["Quarterly report", "Re: Quarterly report"]);

        let lunch = id_of(&client, "Lunch");
        assert_eq!(client.mail_thread(&lunch).unwrap().hits.len(), 1);
        assert!(client.mail_thread("999").is_err());
    }

    #[test]
    fn a_tag_queues_one_flag_change_per_message_it_changes() {
        let (_dir, client) = indexed_store();
        let flagged = vec![String::from("flagged")];

        let planned = client
            .tag_mail(
                query("quarterly or lunch"),
                &flagged,
                &[],
                true,
                &PimdirSearchClock::default(),
            )
            .unwrap();
        assert!(
            planned
                .tagged
                .iter()
                .all(|tagged| tagged.row.is_none() && tagged.handle.is_none())
        );
        assert!(client.store.pending_actions(INBOX).unwrap().is_empty());

        let tag = client
            .tag_mail(
                query("quarterly or lunch"),
                &flagged,
                &[],
                false,
                &PimdirSearchClock::default(),
            )
            .unwrap();
        let statuses: BTreeMap<&str, PimdirTagStatus> = tag
            .tagged
            .iter()
            .map(|tagged| (tagged.subject.as_deref().unwrap_or_default(), tagged.status))
            .collect();
        assert_eq!(statuses["Lunch"], PimdirTagStatus::Unchanged);
        assert_eq!(statuses["Quarterly report"], PimdirTagStatus::Queued);
        assert_eq!(statuses["Re: Quarterly report"], PimdirTagStatus::Queued);

        let pending = client.store.pending_actions(INBOX).unwrap();
        assert_eq!(pending.len(), 2);
        let queued: Vec<&str> = tag
            .tagged
            .iter()
            .filter_map(|t| t.handle.as_deref())
            .collect();
        for action in pending {
            assert!(
                queued
                    .iter()
                    .any(|row| row.ends_with(&format!(".{}", action.id)))
            );
            let PimdirAction::SetFlags { flags, .. } = action.action else {
                panic!("expected set-flags, got {:?}", action.action);
            };
            assert_eq!(
                flags.known().unwrap().iter().collect::<Vec<_>>(),
                ["\\Flagged"]
            );
        }
    }

    #[test]
    fn a_tag_on_flags_never_synced_stages_nothing_for_them() {
        let (_dir, client) = indexed_store();

        let tag = client
            .tag_mail(
                query("invoice"),
                &[String::from("flagged")],
                &[],
                false,
                &PimdirSearchClock::default(),
            )
            .unwrap();

        assert_eq!(tag.tagged.len(), 1);
        assert_eq!(tag.tagged[0].status, PimdirTagStatus::UnknownFlags);
        assert_eq!(tag.tagged[0].handle, None);
        assert!(client.store.pending_actions(INBOX).unwrap().is_empty());
    }

    #[test]
    fn a_tag_naming_no_flag_is_refused() {
        let (_dir, client) = indexed_store();

        let err = client
            .tag_mail(
                query("lunch"),
                &[String::from("unread")],
                &[],
                false,
                &PimdirSearchClock::default(),
            )
            .err()
            .unwrap();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::TagRefused),
            "{err:#}"
        );
    }

    /// A query past SEARCH §8.3 bounds (1,024 characters, 32 open groups,
    /// 128 terms) is refused whole as `limit-exceeded`, plain and JSON.
    #[test]
    fn a_query_past_its_bounds_is_refused_as_limit_exceeded() {
        let long = "a".repeat(1025);
        let deep = format!("{}a{}", "(".repeat(33), ")".repeat(33));
        let wide = vec!["a"; 129].join(" ");

        for (text, span) in [(long, "1024..1025"), (deep, "32..33"), (wide, "256..257")] {
            for json in [true, false] {
                let err = parse_query(std::slice::from_ref(&text), json)
                    .err()
                    .unwrap();
                assert_eq!(
                    pimalaya_cli::error::code_of::<ErrorCode>(&err),
                    Some(ErrorCode::QueryRefused),
                    "{err}"
                );
                let err = err.to_string();
                assert!(err.contains("limit-exceeded"), "{err}");
                if json {
                    assert!(err.contains(&format!("limit-exceeded at {span}")), "{err}");
                }
            }
        }
    }

    #[test]
    fn a_refused_query_names_its_code_and_span() {
        let cases = [
            ("()", "empty-query"),
            ("frm:alice", "unknown-field"),
            ("is:bogus", "invalid-value"),
            ("date:2026-13-45", "invalid-date"),
            ("\"hello", "unterminated-phrase"),
            ("(hello", "unbalanced-parenthesis"),
            ("hello and", "dangling-operator"),
            ("he*llo", "invalid-wildcard"),
        ];

        for (text, code) in cases {
            for json in [true, false] {
                let err = parse_query(&[text.to_string()], json).err().unwrap();
                assert_eq!(
                    pimalaya_cli::error::code_of::<ErrorCode>(&err),
                    Some(ErrorCode::QueryRefused),
                    "{text}"
                );
                assert!(err.to_string().contains(code), "{text}: {err}");
            }
        }

        let err = parse_query(&[String::from("frm:alice")], true)
            .err()
            .unwrap();
        assert!(
            err.to_string()
                .starts_with("Query refused, unknown-field at 0..4"),
            "{err}"
        );
    }

    #[test]
    fn a_missing_or_outdated_index_says_to_run_the_sync_engine() {
        let (dir, client) = indexed_store();
        let assert_coded = |code| {
            let err = client
                .search_mail(query("lunch"), &PimdirSearchOptions::default())
                .err()
                .unwrap();
            assert_eq!(
                pimalaya_cli::error::code_of::<ErrorCode>(&err),
                Some(code),
                "{err:#}"
            );
            let err = format!("{err:#}");
            assert!(err.contains("sync engine"), "{err}");
            assert!(!err.contains("delete"), "{err}");
        };

        let conn = rusqlite::Connection::open(dir.path().join("index.db")).unwrap();
        conn.execute("UPDATE index_meta SET tokenizer = 'porter'", [])
            .unwrap();
        drop(conn);
        assert_coded(ErrorCode::IndexOutdated);

        for name in ["index.db", "index.db-wal", "index.db-shm"] {
            let _ = std::fs::remove_file(dir.path().join(name));
        }
        assert_coded(ErrorCode::IndexMissing);
        assert!(!dir.path().join("index.db").exists(), "the index was built");
    }

    /// An index of another SEARCH draft is refused as `index-version`,
    /// pointing at the sync engine, and left as it is.
    #[test]
    fn an_index_of_another_draft_is_refused_as_index_version() {
        let (dir, client) = indexed_store();
        let conn = rusqlite::Connection::open(dir.path().join("index.db")).unwrap();
        conn.execute(
            "UPDATE index_meta SET format_version = 'search-draft-11'",
            [],
        )
        .unwrap();

        let err = client
            .search_mail(query("lunch"), &PimdirSearchOptions::default())
            .err()
            .unwrap();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::IndexVersion),
            "{err:#}"
        );
        let err = format!("{err:#}");
        assert!(err.contains("search-draft-11"), "{err}");
        assert!(err.contains("sync engine"), "{err}");
        assert!(!err.contains("delete"), "{err}");
        let version: String = conn
            .query_row("SELECT format_version FROM index_meta", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, "search-draft-11", "the index was rebuilt");
    }

    #[test]
    fn a_range_below_the_coverage_is_not_complete() {
        let (dir, client) = indexed_store();
        let mut owner = PimdirStore::open(dir.path()).unwrap().for_source("imap");
        let inbox = PimdirCollectionId(INBOX.into());
        owner
            .write(vec![
                PimdirWriteOp::OpenRound {
                    collection: inbox.clone(),
                    scope: PimdirScope::since("2026-05-15T00:00:00Z"),
                    band: false,
                },
                PimdirWriteOp::CloseRound {
                    collection: inbox,
                    coverage: PimdirScope::since("2026-05-15T00:00:00Z"),
                    checkpoint: None,
                },
            ])
            .unwrap();
        drop(owner);
        refresh(dir.path());

        let below = page(&client, "date:2026-05-01..2026-05-10").coverage;
        assert_eq!(below.since.as_deref(), Some("2026-05-15T00:00:00Z"));
        assert!(!below.complete);
        assert!(
            below
                .to_string()
                .contains("mail before 2026-05-15 is not synced"),
            "{below}"
        );

        let within = page(&client, "lunch date:2026-05-16..").coverage;
        assert_eq!(within.since, None);
    }

    #[test]
    fn a_complete_answer_prints_no_coverage_line() {
        let coverage = PimdirMailCoverage::from(&PimdirSearchCoverage::default());

        assert!(coverage.complete);
        assert_eq!(coverage.to_string(), "");
    }

    #[test]
    fn coverage_renders_as_camel_case_json() {
        let coverage = PimdirMailCoverage {
            complete: false,
            bodiless: 2,
            missing: 4,
            unparseable: 5,
            encrypted: 6,
            unindexed: 1,
            behind: 3,
            since: Some("2026-05-15T00:00:00Z".into()),
            until: None,
            filling: true,
            counted: PimdirSearchUnread::default(),
        };

        assert_eq!(
            serde_json::to_value(&coverage).unwrap(),
            serde_json::json!({
                "complete": false,
                "bodiless": 2,
                "missing": 4,
                "unparseable": 5,
                "encrypted": 6,
                "unindexed": 1,
                "behind": 3,
                "since": "2026-05-15T00:00:00Z",
                "until": null,
                "filling": true,
            })
        );
        let line = coverage.to_string();
        for part in [
            "mail before 2026-05-15 is not synced",
            "a first sync is still listing a mailbox",
            "1 message(s) changed since the index was refreshed",
            "the index is 3 change(s) behind the store",
        ] {
            assert!(line.contains(part), "{line}");
        }
        for part in [
            "no local body",
            "could not read",
            "could not parse",
            "encrypted",
        ] {
            assert!(!line.contains(part), "{line}");
        }
    }

    /// A bodiless message is counted, and leaves the answer incomplete only
    /// for body terms, `thread:` and `has:attachment` (SEARCH §8.2).
    #[test]
    fn a_bodiless_message_is_incomplete_only_while_a_term_reads_it() {
        let (dir, client) = indexed_store();
        let mut owner = PimdirStore::open(dir.path()).unwrap().for_source("imap");
        let inbox = PimdirCollectionId(INBOX.into());
        owner
            .write(vec![
                PimdirWriteOp::OpenRound {
                    collection: inbox.clone(),
                    scope: PimdirScope::unbounded(),
                    band: false,
                },
                PimdirWriteOp::CloseRound {
                    collection: inbox,
                    coverage: PimdirScope::unbounded(),
                    checkpoint: None,
                },
            ])
            .unwrap();
        drop(owner);
        refresh(dir.path());

        for text in ["is:flagged", "date:2026-05-01..", "from:alice@x.org"] {
            let coverage = page(&client, text).coverage;
            assert_eq!(coverage.bodiless, 1, "{text}");
            assert!(coverage.complete, "{text}");
            assert_eq!(coverage.summary(), None, "{text}");
        }

        for text in [
            "subject:lunch",
            "thread:a@x.org",
            "has:attachment",
            "(is:flagged) -has:attachment",
        ] {
            let coverage = page(&client, text).coverage;
            assert_eq!(coverage.bodiless, 1, "{text}");
            assert!(!coverage.complete, "{text}");
            let line = coverage.to_string();
            assert!(line.contains("1 message(s) have no local body"), "{text}");
        }
    }

    /// A body the index could not read, parse or decrypt is counted, and
    /// leaves only a body query incomplete (pimdir SEARCH §8.2).
    #[test]
    fn an_unread_body_is_incomplete_only_while_a_term_reads_bodies() {
        use io_pimdir::search::coverage::PimdirSearchReads;

        let mut coverage = PimdirSearchCoverage::default();
        coverage.local.kinds.insert(
            "message/rfc822".into(),
            PimdirSearchUnread {
                bodiless: 0,
                missing: 1,
                unparseable: 2,
                encrypted: 3,
            },
        );

        let headers = PimdirMailCoverage::from(&coverage);
        assert!(headers.complete);
        assert_eq!(
            (headers.missing, headers.unparseable, headers.encrypted),
            (1, 2, 3)
        );
        assert_eq!(headers.summary(), None);

        coverage.local.reads = PimdirSearchReads::text();
        let structure = PimdirMailCoverage::from(&coverage);
        assert!(!structure.complete);
        let line = structure.to_string();
        assert!(line.contains("1 message(s) have a body the index could not read"));
        assert!(line.contains("2 message(s) have a body the index could not parse"));
        assert!(!line.contains("encrypted"), "{line}");

        coverage.local.reads.content = true;
        let body = PimdirMailCoverage::from(&coverage);
        assert!(!body.complete);
        assert!(
            body.to_string().contains("3 message(s) are encrypted"),
            "{body}"
        );
    }

    #[test]
    fn a_search_reads_the_configured_account_alone() {
        let dir = tempfile::tempdir().unwrap();
        for account in ["work", "home"] {
            PimdirStore::open(dir.path())
                .unwrap()
                .for_account(account)
                .ensure_collection(format!("{account}/INBOX"), "message/rfc822")
                .unwrap();
        }
        let client = |account: &str| {
            PimdirClient::new(PimdirConfig {
                root: dir.path().to_path_buf(),
                account: Some(account.into()),
            })
            .unwrap()
        };
        let (mut work, mut home) = (client("work"), client("home"));
        let lunch = |id| mail(id, "From: carol@z.org\r\nSubject: Lunch", "Pizza?");
        work.add_message("work/INBOX", &[], lunch("w")).unwrap();
        home.add_message("home/INBOX", &[], lunch("h")).unwrap();
        let mut owner = PimdirStore::open(dir.path()).unwrap().for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 2);
        drop(owner);
        refresh(dir.path());

        for (client, mailbox) in [(&work, "work/INBOX"), (&home, "home/INBOX")] {
            let hits = page(client, "pizza").hits;
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].mailboxes, [mailbox]);
        }
    }
}
