//! # Microsoft Graph backend
//!
//! The Graph adapter of the unified client, glue over the
//! io-msgraph client [`MsgraphClient`] wraps.
//!
//! Graph is folder-based, and a shared flag maps onto a scalar field of
//! the message, `isRead` or the follow-up flag, or onto its categories.
//! There is no `add_message`: a MIME message Graph creates stays a draft.

use std::{
    collections::{BTreeMap, BTreeSet},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset};
use io_msgraph::v1::{
    client::MsgraphClientStdError,
    field::MsgraphField,
    rest::{
        batch::{MsgraphBatchRequest, MsgraphBatchResponse},
        users::{
            mail_folders::{
                MsgraphMailFolder,
                list::{MsgraphMailFoldersListParams, MsgraphMailFoldersListResponse},
            },
            messages::{
                MsgraphFlagStatus, MsgraphFollowupFlag, MsgraphImportance, MsgraphMessage,
                MsgraphRecipient,
                list::{MsgraphMessagesListParams, MsgraphMessagesListResponse},
            },
        },
    },
    send::{MsgraphSend, MsgraphSendError, path_segment, user_path},
};
use pimalaya_cli::sanitize;
use serde_json::{Value, json};
use url::Url;

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, EnvelopePage, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole},
        outcome::Outcome,
        search::{query::SearchEmailsQuery, sort::eval::sort_envelopes},
    },
    error::{ErrorCode, ItemError, missing_message, unknown_message},
    msgraph::{
        client::{MsgraphClient, names_no_message, refusal_of},
        search,
    },
};

/// OData `$select` for envelope listing: the message fields backing the
/// shared [`Envelope`], returned in one round-trip.
const ENVELOPE_SELECT: &str = "id,subject,from,toRecipients,sentDateTime,receivedDateTime,isRead,isDraft,hasAttachments,internetMessageId,importance,flag,categories";

/// The well-known folder names carrying a special-use role, a stable Graph
/// contract accepted in place of a folder id.
const WELL_KNOWN_FOLDERS: [(&str, MailboxRole); 6] = [
    ("inbox", MailboxRole::Inbox),
    ("sentitems", MailboxRole::Sent),
    ("drafts", MailboxRole::Drafts),
    ("deleteditems", MailboxRole::Trash),
    ("junkemail", MailboxRole::Junk),
    ("archive", MailboxRole::Archive),
];

/// The largest page Graph serves of a message listing (`$top`), asked for
/// when the whole listing is wanted.
const MESSAGES_MAX_TOP: u32 = 1000;

/// The levels of a nested folder's name are joined with it, as on Gmail.
const FOLDER_DELIMITER: char = '/';

/// The sub-requests one `$batch` holds: Graph runs them in parallel and
/// serves at most four concurrent requests per mailbox, throttling the
/// others.
const BATCH_CONCURRENCY: usize = 4;

/// How many times a throttled sub-request is sent again.
const BATCH_RETRIES: usize = 3;

/// The longest `Retry-After` honoured between two tries, in seconds.
const BATCH_MAX_WAIT: u64 = 10;

impl MsgraphClient {
    /// Lists every Graph mail folder as a mailbox, nested ones named by
    /// their path (`Parent/Child`). Graph carries counts inline, kept
    /// only with `with_counts` as the other backends do.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let roles = self.folder_roles()?;
        let mut folders = Vec::new();
        self.list_folders_under(None, "", &mut folders)?;

        Ok(folders
            .into_iter()
            .map(|(name, folder)| {
                let role = roles
                    .iter()
                    .find(|(id, _)| *id == folder.id)
                    .map(|(_, role)| role.clone());
                let mut mailbox = mailbox_from(name, folder, role);
                if !with_counts {
                    mailbox.total = None;
                    mailbox.unread = None;
                }
                mailbox
            })
            .collect())
    }

    /// Appends the folders under `parent` (the top when `None`) to
    /// `out`, each followed by its own children, every page read.
    fn list_folders_under(
        &mut self,
        parent: Option<&str>,
        prefix: &str,
        out: &mut Vec<(String, MsgraphMailFolder)>,
    ) -> Result<()> {
        let params = MsgraphMailFoldersListParams {
            top: Some(100),
            ..Default::default()
        };
        let mut page = match parent {
            Some(id) => self.mail_child_folders_list(id, &params)?.response,
            None => self.mail_folders_list(&params)?.response,
        };

        loop {
            for folder in page.value {
                let name = format!("{prefix}{}", folder.display_name);
                let id = folder.id.clone();
                let nested = folder.child_folder_count.unwrap_or(0) > 0;
                out.push((name.clone(), folder));

                if nested {
                    let prefix = format!("{name}{FOLDER_DELIMITER}");
                    self.list_folders_under(Some(&id), &prefix, out)?;
                }
            }

            let Some(next) = page.next_link else {
                return Ok(());
            };
            let url = Url::parse(&next).context("Cannot parse the folder paging link")?;
            let coroutine = MsgraphSend::<MsgraphMailFoldersListResponse>::get(&self.auth, url);
            page = self.run(coroutine)?.response;
        }
    }

    /// Resolves the well-known folder names to folder ids through
    /// `$batch`, since Graph v1.0 lists folders without them. A name the
    /// mailbox lacks, such as `archive`, is skipped.
    fn folder_roles(&mut self) -> Result<Vec<(String, MailboxRole)>> {
        let user = user_path(&self.user_id)?;
        let requests: Vec<_> = WELL_KNOWN_FOLDERS
            .iter()
            .map(|(name, _)| MsgraphBatchRequest {
                id: name.to_string(),
                method: "GET".into(),
                url: format!("/{user}/mailFolders/{name}?$select=id"),
                ..Default::default()
            })
            .collect();

        let responses = self.batch_all(requests)?;
        roles_from(responses)
    }

    /// Sends `requests` through `$batch`, at most [`BATCH_CONCURRENCY`] at
    /// a time, sending a throttled (429) or unavailable (5xx) one again
    /// after the delay Graph asks for.
    ///
    /// Returns one response per request, a sub-request still failing
    /// after [`BATCH_RETRIES`] tries keeping its last error status.
    fn batch_all(
        &mut self,
        mut requests: Vec<MsgraphBatchRequest>,
    ) -> Result<Vec<MsgraphBatchResponse>> {
        let mut done = Vec::with_capacity(requests.len());

        for attempt in 0..=BATCH_RETRIES {
            let mut retry = Vec::new();
            let mut wait = 0;

            for chunk in requests.chunks(BATCH_CONCURRENCY) {
                for response in self.batch(chunk)?.response.responses {
                    let Some(request) = chunk.iter().find(|r| r.id == response.id) else {
                        continue;
                    };

                    if attempt < BATCH_RETRIES && is_retryable(response.status) {
                        wait = wait.max(response.retry_after().unwrap_or(1));
                        retry.push(request.clone());
                    } else {
                        done.push(response);
                    }
                }
            }

            if retry.is_empty() {
                break;
            }

            thread::sleep(Duration::from_secs(wait.min(BATCH_MAX_WAIT)));
            requests = retry;
        }

        Ok(done)
    }

    /// Keeps the ids of `ids` naming a message of the folder `mailbox`,
    /// each once, in the given order, beside the error of each other one.
    ///
    /// Graph message ids are account-wide and its message endpoints take
    /// no folder, so without this a command would act on a message of
    /// another folder.
    fn ids_in_folder<'a>(
        &mut self,
        mailbox: &str,
        ids: &[&'a str],
    ) -> Result<Outcome<Vec<&'a str>>> {
        let user = user_path(&self.user_id)?;
        let mut unique: Vec<&str> = Vec::with_capacity(ids.len());
        let mut requests = Vec::with_capacity(ids.len());
        let mut errors = Vec::new();

        for id in ids {
            if unique.contains(id) {
                continue;
            }

            // NOTE: an id that cannot be a path segment names nothing.
            let Ok(segment) = path_segment(id) else {
                errors.push(missing_message(id));
                continue;
            };

            requests.push(MsgraphBatchRequest {
                id: unique.len().to_string(),
                method: "GET".into(),
                url: format!("/{user}/messages/{segment}?$select=id,parentFolderId"),
                ..Default::default()
            });
            unique.push(id);
        }

        if requests.is_empty() {
            return Ok(Outcome {
                value: Vec::new(),
                errors,
            });
        }

        let responses = self.batch_all(requests)?;
        Ok(ids_found(&unique, responses, mailbox).after(errors))
    }

    /// Sends one `$batch` request per id of `ids`, `request` building each
    /// from the id's path segment, returning how many Graph answered with a
    /// success beside the error of each other one.
    fn write_each(
        &mut self,
        ids: &[&str],
        request: impl Fn(String) -> MsgraphBatchRequest,
    ) -> Result<Outcome<usize>> {
        let user = user_path(&self.user_id)?;
        let mut requests = Vec::with_capacity(ids.len());

        for (index, id) in ids.iter().enumerate() {
            let mut sub = request(format!("/{user}/messages/{}", path_segment(id)?));
            sub.id = index.to_string();
            requests.push(sub);
        }

        if requests.is_empty() {
            return Ok(Outcome::from(0));
        }

        let responses = self.batch_all(requests)?;
        Ok(written(ids, responses))
    }

    /// Lists envelopes from the `mailbox` folder (id or well-known name
    /// such as `inbox`). Page 1 is the most recent window; the offset is
    /// `(page - 1) * page_size` via OData `$skip`, and a page follows while
    /// Graph gives a next link. Without a page size, every page is read and
    /// `page` is moot.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<EnvelopePage> {
        let page = page.unwrap_or(1).max(1);
        let (top, skip) = match page_size {
            Some(size) => (size, Some((page - 1) * size)),
            None => (MESSAGES_MAX_TOP, None),
        };

        let params = MsgraphMessagesListParams {
            top: Some(top),
            skip,
            select: Some(ENVELOPE_SELECT),
            ..Default::default()
        };
        let response = self.messages_list(Some(mailbox), &params)?.response;

        let (messages, more) = match page_size {
            Some(_) => (response.value, response.next_link.is_some()),
            None => (self.messages_to_the_end(response)?, false),
        };

        let envelopes = messages.into_iter().map(envelope_from).collect();
        Ok(EnvelopePage { envelopes, more })
    }

    /// Searches the `mailbox` folder with the shared query, its filter
    /// translated to a KQL `$search`, or a lone `message-id` clause to a
    /// `$filter` on `internetMessageId`.
    ///
    /// Graph refuses `$skip` and `$orderby` beside `$search`, so a page is
    /// reached through the paging links and sorted locally, newest first
    /// by default since Graph answers in relevance order. Without a page
    /// size, every page is read.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<EnvelopePage> {
        let sort = query.and_then(|query| query.sort.as_deref());

        let Some(filter) = query.and_then(|query| query.filter.as_ref()) else {
            let mut listed = self.list_envelopes(mailbox, page, page_size, with_attachment)?;
            sort_envelopes(&mut listed.envelopes, sort);
            return Ok(listed);
        };

        let (search, odata_filter) = match search::message_id_filter(filter) {
            Some(odata_filter) => (None, Some(odata_filter)),
            None => (Some(search::filter_to_search(filter)?), None),
        };
        let params = MsgraphMessagesListParams {
            top: Some(page_size.unwrap_or(MESSAGES_MAX_TOP)),
            select: Some(ENVELOPE_SELECT),
            search: search.as_deref(),
            filter: odata_filter.as_deref(),
            ..Default::default()
        };
        let mut response = self.messages_list(Some(mailbox), &params)?.response;

        let (messages, more) = match page_size {
            Some(_) => {
                for _ in 1..page.unwrap_or(1).max(1) {
                    let Some(next) = response.next_link else {
                        return Ok(EnvelopePage::default());
                    };
                    response = self.messages_next_page(&next)?;
                }
                (response.value, response.next_link.is_some())
            }
            None => (self.messages_to_the_end(response)?, false),
        };

        let mut envelopes: Vec<Envelope> = messages.into_iter().map(envelope_from).collect();
        sort_envelopes(&mut envelopes, sort);

        Ok(EnvelopePage { envelopes, more })
    }

    /// The messages of `response` and of every page its next links lead
    /// to.
    fn messages_to_the_end(
        &mut self,
        mut response: MsgraphMessagesListResponse,
    ) -> Result<Vec<MsgraphMessage>> {
        let mut messages = Vec::new();

        loop {
            messages.append(&mut response.value);

            let Some(next) = response.next_link else {
                return Ok(messages);
            };

            response = self.messages_next_page(&next)?;
        }
    }

    /// Follows one message paging link.
    fn messages_next_page(&mut self, link: &str) -> Result<MsgraphMessagesListResponse> {
        let url = Url::parse(link).context("Cannot parse the message paging link")?;
        let coroutine = MsgraphSend::<MsgraphMessagesListResponse>::get(&self.auth, url);
        Ok(self.run(coroutine)?.response)
    }

    /// Adds, sets, or removes `flags` on the messages of `ids` the folder
    /// `mailbox` holds, one `PATCH /messages/{id}` each through `$batch`,
    /// returning how many it patched beside the error of each other id.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        let found = self.ids_in_folder(mailbox, ids)?;
        let patch = flag_patch(flags, op);

        // NOTE: `$Forwarded` has no Graph field, so adding or removing it
        // alone leaves an empty PATCH, a wasted request that could still
        // fail. `\Answered` and `\Draft` never get here: the unified API
        // refuses them up front (`flags.answered`, `flags.draft`).
        if patch == MsgraphMessage::default() {
            return Ok(found.map(|ids| ids.len()));
        }

        let body = serde_json::to_value(&patch).context("Cannot serialize the message patch")?;
        let patched = self.write_each(&found.value, |url| json_request("PATCH", url, &body))?;

        Ok(patched.after(found.errors))
    }

    /// Fetches one message's raw RFC 5322 bytes via
    /// `GET /messages/{id}/$value`. `mailbox` is unused.
    pub fn get_message(&mut self, _mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let raw = match self.message_get_raw(id) {
            Ok(output) => output.response,
            Err(MsgraphClientStdError::Send(err))
                if err
                    .api()
                    .is_some_and(|api| names_no_message(api.status, &api.code)) =>
            {
                return Err(unknown_message(id, err));
            }
            Err(err) => return Err(err.into()),
        };

        // NOTE: fetching `$value` never changes read state, so `--seen`
        // costs a separate PATCH setting `isRead`.
        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.message_update(id, &flag_patch(&[seen], FlagOp::Add))?;
        }

        Ok(raw)
    }

    /// Copies the messages of `ids` the folder `from` holds into `to`
    /// through `$batch`, returning how many it copied beside the error of
    /// each other id.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let found = self.ids_in_folder(from, ids)?;
        let body = json!({ "destinationId": to });

        let copied = self.write_each(&found.value, |url| {
            json_request("POST", format!("{url}/copy"), &body)
        })?;

        Ok(copied.after(found.errors))
    }

    /// Moves the messages of `ids` the folder `from` holds into `to`
    /// through `$batch`, returning how many it moved beside the error of
    /// each other id. Moving into `from` itself leaves them as they are.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let found = self.ids_in_folder(from, ids)?;
        if from == to {
            return Ok(found.map(|ids| ids.len()));
        }

        let body = json!({ "destinationId": to });
        let moved = self.write_each(&found.value, |url| {
            json_request("POST", format!("{url}/move"), &body)
        })?;

        Ok(moved.after(found.errors))
    }

    /// Permanently deletes the messages of `ids` the folder `mailbox`
    /// holds (`DELETE /messages/{id}` through `$batch`; from Deleted Items
    /// this is a hard delete), returning how many it deleted beside the
    /// error of each other id.
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let found = self.ids_in_folder(mailbox, ids)?;

        let deleted = self.write_each(&found.value, |url| MsgraphBatchRequest {
            method: "DELETE".into(),
            url,
            ..Default::default()
        })?;

        Ok(deleted.after(found.errors))
    }

    /// Creates the folder `name` at the top, or under `parent`, returning
    /// its id.
    ///
    /// io-msgraph creates top-level folders only, so a folder under a
    /// parent is created at the top then moved under it, two requests: a
    /// failed move leaves it at the top, which the error says.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        let folder = MsgraphMailFolder {
            display_name: name.to_string(),
            ..Default::default()
        };

        let created = self.mail_folder_create(&folder)?.response;
        self.forget_mailboxes();

        let Some(parent) = parent else {
            return Ok(created.id);
        };

        let moved = self
            .mail_folder_move(&created.id, parent)
            .with_context(|| {
                format!("Folder `{name}` created at the top, not moved under `{parent}`")
            })?
            .response;

        Ok(moved.id)
    }

    /// Renames the folder `mailbox` to `name`, its parent kept (`PATCH
    /// /mailFolders/{id}`), returning its id, which a rename leaves as it
    /// is.
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<String> {
        let folder = MsgraphMailFolder {
            display_name: name.to_string(),
            ..Default::default()
        };

        self.mail_folder_update(mailbox, &folder)?;
        self.forget_mailboxes();
        Ok(mailbox.to_string())
    }

    /// Deletes the folder `mailbox` with the messages and the folders it
    /// holds (`DELETE /mailFolders/{id}`).
    pub fn delete_mailbox(&mut self, mailbox: &str) -> Result<()> {
        self.mail_folder_delete(mailbox)?;
        self.forget_mailboxes();
        Ok(())
    }

    /// Sends a raw RFC 5322 message via `POST /sendMail` (MIME form);
    /// Graph saves it to Sent Items.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        self.mail_send_mime(&raw)?;
        Ok(())
    }
}

/// The role a well-known folder name carries, ignoring case.
pub(super) fn well_known_role(name: &str) -> Option<MailboxRole> {
    WELL_KNOWN_FOLDERS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|(_, role)| role.clone())
}

/// Converts one Graph mail folder named `name` into the shared
/// [`Mailbox`] shape; Graph folders carry their counts inline.
fn mailbox_from(name: String, folder: MsgraphMailFolder, role: Option<MailboxRole>) -> Mailbox {
    Mailbox {
        id: folder.id,
        name,
        role,
        total: folder.total_item_count,
        unread: folder.unread_item_count,
    }
}

/// Reads the well-known folder ids off their `$batch` responses.
///
/// A 404 means the mailbox has no such folder and is skipped; any other
/// failure is an error, a role silently missing sending `-m sent` to
/// Graph as a folder id.
fn roles_from(responses: Vec<MsgraphBatchResponse>) -> Result<Vec<(String, MailboxRole)>> {
    let mut roles = Vec::new();

    for response in responses {
        let Some((name, role)) = WELL_KNOWN_FOLDERS
            .iter()
            .find(|(name, _)| *name == response.id)
        else {
            continue;
        };

        if response.status == 404 {
            continue;
        }

        let folder = response
            .parse::<MsgraphMailFolder>()
            .with_context(|| format!("Cannot resolve the well-known folder `{name}`"))?;
        roles.push((folder.id, role.clone()));
    }

    Ok(roles)
}

/// Keeps the ids of `ids` whose `$batch` response, answered under the id's
/// index, names a message of the folder `mailbox`, beside the error of
/// each other id: one of another folder is not found, as is one Graph
/// answers so ([`part_error`]).
fn ids_found<'a>(
    ids: &[&'a str],
    responses: Vec<MsgraphBatchResponse>,
    mailbox: &str,
) -> Outcome<Vec<&'a str>> {
    let mut outcome = Outcome::from(Vec::with_capacity(ids.len()));

    for (id, response) in ids.iter().zip(answered(ids, responses)) {
        let Some(response) = response else {
            outcome.errors.push(unanswered(id));
            continue;
        };

        if !(200..300).contains(&response.status) {
            outcome.errors.push(part_error(id, response));
            continue;
        }

        match response.parse::<MsgraphMessage>() {
            Ok(message) if message.parent_folder_id.as_deref() == Some(mailbox) => {
                outcome.value.push(*id);
            }
            Ok(_) => outcome.errors.push(missing_message(id)),
            Err(err) => outcome.errors.push(ItemError {
                id: Some(id.to_string()),
                code: Some(ErrorCode::MessageUnreadable),
                error: String::from("Cannot parse the Graph message"),
                sources: vec![err.to_string()],
            }),
        }
    }

    outcome
}

/// Counts the ids of `ids` whose `$batch` response, answered under the
/// id's index, is a success, beside the error of each other id.
fn written(ids: &[&str], responses: Vec<MsgraphBatchResponse>) -> Outcome<usize> {
    let mut outcome = Outcome::from(0);

    for (id, response) in ids.iter().zip(answered(ids, responses)) {
        match response {
            None => outcome.errors.push(unanswered(id)),
            Some(response) if (200..300).contains(&response.status) => outcome.value += 1,
            Some(response) => outcome.errors.push(part_error(id, response)),
        }
    }

    outcome
}

/// The `$batch` responses in the order of `ids`, each answered under its
/// id's index, `None` for one the answer left out.
fn answered(
    ids: &[&str],
    responses: Vec<MsgraphBatchResponse>,
) -> Vec<Option<MsgraphBatchResponse>> {
    let mut answered: Vec<_> = ids.iter().map(|_| None).collect();

    for response in responses {
        if let Some(slot) = response
            .id
            .parse::<usize>()
            .ok()
            .and_then(|i| answered.get_mut(i))
        {
            *slot = Some(response);
        }
    }

    answered
}

/// The item error of the message `id` a `$batch` answer left out.
fn unanswered(id: &str) -> ItemError {
    ItemError {
        id: Some(id.to_owned()),
        code: None,
        error: String::from("Graph batch answer holds no response for this message"),
        sources: Vec::new(),
    }
}

/// The item error of the message `id` a `$batch` part failed on, coded by
/// its status and Graph's `error.code`, both kept in `sources`.
///
/// A 404, or a 400 Graph codes `ErrorInvalidIdMalformed`, is a message not
/// found; any other 400 is a request Graph refused, no missing message.
/// `ErrorQuotaExceeded` or a 507 is a quota refusal, whatever the status.
fn part_error(id: &str, response: MsgraphBatchResponse) -> ItemError {
    let status = response.status;
    let (code, message) = match response.parse::<Value>() {
        Err(MsgraphSendError::Api(err)) => (err.code, err.message),
        _ => (String::new(), String::new()),
    };

    let item_code = match names_no_message(status, &code) {
        true => Some(ErrorCode::MessageNotFound),
        false => refusal_of(status, &code),
    };

    let mut sources = vec![format!("HTTP {status}")];
    if !code.is_empty() {
        sources.push(sanitize::text(&code).into_owned());
    }

    let error = match message.is_empty() {
        true => format!("Graph refused the message with status {status}"),
        false => format!("Graph refused the message: {}", sanitize::text(&message)),
    };

    ItemError {
        id: Some(id.to_owned()),
        code: item_code,
        error,
        sources,
    }
}

/// A `$batch` request with a JSON body.
fn json_request(method: &str, url: String, body: &Value) -> MsgraphBatchRequest {
    MsgraphBatchRequest {
        method: method.into(),
        url,
        headers: BTreeMap::from([("Content-Type".into(), "application/json".into())]),
        body: Some(body.clone()),
        ..Default::default()
    }
}

/// Whether a `$batch` sub-response is worth sending again: throttled, or
/// a transient server failure.
fn is_retryable(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Folds a Graph message into the shared [`Envelope`] shape. `size` is
/// left `0`: the listing selection does not expose a raw size.
pub(crate) fn envelope_from(message: MsgraphMessage) -> Envelope {
    let flags = flags_from_message(&message);
    let message_id = message
        .internet_message_id
        .as_deref()
        .and_then(normalize_message_id);
    // NOTE: the shared date is the `Date:` header, which Graph keeps as
    // `sentDateTime`; a message without one falls back on its arrival.
    let date = message
        .sent_date_time
        .as_deref()
        .and_then(parse_rfc3339)
        .or_else(|| {
            message
                .received_date_time
                .as_deref()
                .and_then(parse_rfc3339)
        });
    let from = message.from.map(address_from).into_iter().collect();
    let to = message
        .to_recipients
        .into_option()
        .into_iter()
        .flatten()
        .map(address_from)
        .collect();

    Envelope {
        id: message.id,
        message_id,
        // NOTE: Graph carries In-Reply-To inside `internetMessageHeaders`
        // alone, which a listing does not select, so the field stays
        // empty rather than cost one request per row.
        in_reply_to: Vec::new(),
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
            let categories = flags
                .iter()
                .filter(|flag| flag.iana().is_none())
                .map(|flag| flag.raw().to_string())
                .collect();
            patch.categories = MsgraphField::Set(categories);
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
    for category in message.categories.as_option().into_iter().flatten() {
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

/// Maps the shared `\Flagged` onto a Graph follow-up flag.
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

/// Maps the shared `$Important` onto a Graph importance.
fn importance(important: bool) -> MsgraphImportance {
    if important {
        MsgraphImportance::High
    } else {
        MsgraphImportance::Normal
    }
}

/// Parses the RFC 3339 timestamp Graph reports a date with.
fn parse_rfc3339(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(trimmed).ok()
}

#[cfg(test)]
mod tests {
    use io_msgraph::v1::rest::batch::{MsgraphBatchResponse, MsgraphBatchResponses};
    use serde_json::json;

    use crate::{
        email::flag::{Flag, FlagOp, IanaFlag},
        email::mailbox::MailboxRole,
        error::{ErrorCode, ItemError},
        msgraph::backend::{flag_patch, ids_found, roles_from, well_known_role, written},
    };

    fn response(id: &str, status: u16, body: serde_json::Value) -> MsgraphBatchResponse {
        MsgraphBatchResponse {
            id: id.to_string(),
            status,
            body: Some(body),
            ..Default::default()
        }
    }

    #[test]
    fn a_throttled_role_lookup_fails_rather_than_dropping_the_role() {
        let throttled = vec![
            response("inbox", 200, json!({"id": "f1"})),
            response(
                "archive",
                404,
                json!({"error": {"code": "ErrorItemNotFound"}}),
            ),
            response(
                "sentitems",
                429,
                json!({"error": {"code": "ApplicationThrottled"}}),
            ),
        ];
        let err = roles_from(throttled).unwrap_err();
        assert!(format!("{err:#}").contains("sentitems"), "got: {err:#}");

        let missing_archive = vec![
            response("inbox", 200, json!({"id": "f1"})),
            response(
                "archive",
                404,
                json!({"error": {"code": "ErrorItemNotFound"}}),
            ),
        ];
        let roles = roles_from(missing_archive).unwrap();
        assert_eq!(roles, vec![(String::from("f1"), MailboxRole::Inbox)]);
    }

    #[test]
    fn a_flag_set_clears_the_categories_it_does_not_name() {
        let seen = Flag::from_iana(IanaFlag::Seen);
        let patch = serde_json::to_value(flag_patch(&[seen], FlagOp::Set)).unwrap();
        assert_eq!(patch["categories"], json!([]));
    }

    #[test]
    fn a_well_known_name_reads_as_its_role() {
        assert_eq!(well_known_role("DeletedItems"), Some(MailboxRole::Trash));
        assert_eq!(well_known_role("sentitems"), Some(MailboxRole::Sent));
        assert_eq!(well_known_role("outbox"), None);
    }

    fn codes(errors: &[ItemError]) -> Vec<(Option<&str>, Option<ErrorCode>)> {
        errors
            .iter()
            .map(|error| (error.id.as_deref(), error.code))
            .collect()
    }

    /// A message of another folder, one Graph answers 404 for or 400
    /// `ErrorInvalidIdMalformed`, is not found; another 400 is a refused
    /// request, and a part the answer left out an error coded `null`.
    #[test]
    fn a_lookup_keeps_the_messages_of_the_folder() {
        let ids = ["in", "elsewhere", "gone", "malformed", "refused", "lost"];
        let responses = vec![
            response("0", 200, json!({"id": "in", "parentFolderId": "trash"})),
            response(
                "1",
                200,
                json!({"id": "elsewhere", "parentFolderId": "inbox"}),
            ),
            response("2", 404, json!({"error": {"code": "ErrorItemNotFound"}})),
            response(
                "3",
                400,
                json!({"error": {"code": "ErrorInvalidIdMalformed"}}),
            ),
            response("4", 400, json!({"error": {"code": "BadRequest"}})),
        ];

        let outcome = ids_found(&ids, responses, "trash");

        assert_eq!(outcome.value, ["in"]);
        assert_eq!(
            codes(&outcome.errors),
            [
                (Some("elsewhere"), Some(ErrorCode::MessageNotFound)),
                (Some("gone"), Some(ErrorCode::MessageNotFound)),
                (Some("malformed"), Some(ErrorCode::MessageNotFound)),
                (Some("refused"), None),
                (Some("lost"), None),
            ]
        );
        assert_eq!(outcome.errors[3].sources, ["HTTP 400", "BadRequest"]);
    }

    /// Each part of a write is counted or coded on its own, the write
    /// carrying on past a refused one.
    #[test]
    fn a_write_counts_each_part_on_its_own() {
        let ids = ["a", "b", "c", "d", "e"];
        let responses = vec![
            response("0", 200, json!({"id": "a"})),
            response("1", 403, json!({"error": {"code": "ErrorAccessDenied"}})),
            response("2", 429, json!({"error": {"code": "ApplicationThrottled"}})),
            response("3", 507, json!({"error": {"code": "ErrorQuotaExceeded"}})),
        ];

        let outcome = written(&ids, responses);

        assert_eq!(outcome.value, 1);
        assert_eq!(
            codes(&outcome.errors),
            [
                (Some("b"), Some(ErrorCode::PermissionDenied)),
                (Some("c"), Some(ErrorCode::RateLimited)),
                (Some("d"), Some(ErrorCode::QuotaExceeded)),
                (Some("e"), None),
            ]
        );
    }

    /// A quota refusal is read before the status: `ErrorQuotaExceeded` sent
    /// as 403, and a 507 with any other code.
    #[test]
    fn a_quota_refusal_is_coded_whatever_its_status() {
        let ids = ["a", "b", "c"];
        let answer = json!({"responses": [
            {
                "id": "0",
                "status": 403,
                "headers": {"Content-Type": "application/json"},
                "body": {"error": {"code": "ErrorQuotaExceeded", "message": "Full"}},
            },
            {
                "id": "1",
                "status": 507,
                "body": {"error": {"code": "ErrorStorageFull", "message": "Full"}},
            },
            {
                "id": "2",
                "status": 403,
                "body": {"error": {"code": "ErrorAccessDenied", "message": "No"}},
            },
        ]});
        let answer: MsgraphBatchResponses = serde_json::from_value(answer).unwrap();

        let outcome = written(&ids, answer.responses);

        assert_eq!(
            codes(&outcome.errors),
            [
                (Some("a"), Some(ErrorCode::QuotaExceeded)),
                (Some("b"), Some(ErrorCode::QuotaExceeded)),
                (Some("c"), Some(ErrorCode::PermissionDenied)),
            ]
        );
    }
}
