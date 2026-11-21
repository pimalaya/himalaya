//! # Microsoft Graph backend
//!
//! The Graph adapter of the unified client, glue over the
//! io-msgraph client [`MsgraphClient`] wraps.
//!
//! Graph is folder-based, and a unified flag maps onto a scalar field of
//! the message, `isRead` or the follow-up flag, or onto its categories.
//! There is no `add_message`: a MIME message Graph creates stays a draft.

use std::{
    collections::{BTreeMap, BTreeSet},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, FixedOffset};
use io_msgraph::v1::{
    client::MsgraphClientStdError,
    field::MsgraphField,
    page::MsgraphPage,
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
    error::{
        ErrorCode, ItemError, existing_mailbox, message_not_found, missing_message, unknown_message,
    },
    msgraph::{
        client::{MsgraphClient, names_no_message, refusal_of},
        search,
    },
    unified::email::{
        address::Address,
        envelope::{Envelope, EnvelopePage, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole},
        outcome::Outcome,
        search::{query::SearchEmailsQuery, sort::eval::sort_envelopes},
    },
};

/// OData `$select` for envelope listing: the message fields backing the
/// unified [`Envelope`], returned in one round-trip.
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
/// throttles past four concurrent requests per mailbox.
const BATCH_CONCURRENCY: usize = 4;

/// How many times a throttled sub-request is sent again.
const BATCH_RETRIES: usize = 3;

/// The longest `Retry-After` honoured between two tries, in seconds.
const BATCH_MAX_WAIT: u64 = 10;

impl MsgraphClient {
    /// Lists every Graph mail folder as a mailbox named by its path, counted
    /// through `$batch`; one whose count Graph refuses is listed without.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Outcome<Vec<Mailbox>>> {
        let roles = self.folder_roles()?;
        let mut folders = Vec::new();
        self.list_folders_under(None, "", None, &mut folders)?;

        let requests = match with_counts {
            true => count_requests(&self.user_id, &folders)?,
            false => Vec::new(),
        };
        let responses = match requests.is_empty() {
            true => Vec::new(),
            false => self.batch_all(requests.clone())?,
        };

        let mut mailboxes: Vec<Mailbox> = folders
            .into_iter()
            .map(|(name, folder)| {
                let role = roles
                    .iter()
                    .find(|(id, _)| *id == folder.id)
                    .map(|(_, role)| role.clone());
                mailbox_from(name, folder, role, with_counts)
            })
            .collect();
        let errors = counts_from(&mut mailboxes, &requests, responses);

        Ok(Outcome {
            value: mailboxes,
            errors,
        })
    }

    /// Appends the folders under `parent` (the top when `None`) to
    /// `out`, each followed by its own children, every page read.
    ///
    /// With `path`, only the folder named by that path is appended, and
    /// only the folders whose path leads to it are walked.
    pub(super) fn list_folders_under(
        &mut self,
        parent: Option<&str>,
        prefix: &str,
        path: Option<&str>,
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
                let prefix = format!("{name}{FOLDER_DELIMITER}");
                let nested = folder.child_folder_count.unwrap_or(0) > 0
                    && path.is_none_or(|path| path.starts_with(&prefix));

                if path.is_none_or(|path| path == name) {
                    out.push((name, folder));
                }

                if nested {
                    self.list_folders_under(Some(&id), &prefix, path, out)?;
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

    /// Resolves the well-known folder names to ids, Graph v1.0 listing
    /// folders without them; a name the mailbox lacks is skipped.
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

    /// Sends `requests` [`BATCH_CONCURRENCY`] at a time, retrying a 429 or
    /// 5xx after the delay Graph asks for.
    ///
    /// One response per request, one still failing after [`BATCH_RETRIES`]
    /// tries keeping its last error status.
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

    /// Sends one `$batch` request per id, returning how many succeeded
    /// beside each other one's error.
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

    /// Lists envelopes from the `mailbox` folder, page 1 the most recent,
    /// through `$skip`; without a page size, every page is read.
    ///
    /// Graph answers a next link after an exactly full last page too, so
    /// one message past the page is asked: a page follows when it exists.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<EnvelopePage> {
        let page = page.unwrap_or(1).max(1);
        let (top, skip) = match page_size {
            Some(size) => (
                size.saturating_add(1).min(MESSAGES_MAX_TOP),
                Some((page - 1) * size),
            ),
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
            Some(size) if top > size => {
                let mut messages = response.value;
                let more = messages.len() > size as usize;
                messages.truncate(size as usize);
                (messages, more)
            }
            Some(_) => (response.value, response.next_link.is_some()),
            None => (self.messages_to_the_end(response)?, false),
        };

        let envelopes = messages.into_iter().map(envelope_from).collect();
        Ok(EnvelopePage { envelopes, more })
    }

    /// Searches the `mailbox` folder through KQL `$search`, a lone
    /// `message-id` clause through `$filter` on `internetMessageId`.
    ///
    /// Graph refuses `$skip` and `$orderby` beside `$search`, so the links
    /// reach one message past the page, sorted locally (newest first, Graph
    /// answering by relevance); without a page size, every page is read.
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

        let window = page_size.map(|size| {
            let start = (page.unwrap_or(1).max(1) as usize - 1).saturating_mul(size as usize);
            (start, start.saturating_add(size as usize))
        });
        let top = match window {
            Some((_, end)) => u32::try_from(end.saturating_add(1))
                .map_or(MESSAGES_MAX_TOP, |top| top.min(MESSAGES_MAX_TOP)),
            None => MESSAGES_MAX_TOP,
        };

        let params = MsgraphMessagesListParams {
            top: Some(top),
            select: Some(ENVELOPE_SELECT),
            search: search.as_deref(),
            filter: odata_filter.as_deref(),
            ..Default::default()
        };
        let mut response = self.messages_list(Some(mailbox), &params)?.response;

        let (messages, more) = match window {
            Some((start, end)) => {
                let mut messages = Vec::new();

                loop {
                    messages.append(&mut response.value);

                    if messages.len() > end {
                        break;
                    }
                    let Some(next) = response.next_link else {
                        break;
                    };
                    response = self.messages_next_page(&next)?;
                }

                let more = messages.len() > end;
                messages.truncate(end);
                (messages.split_off(start.min(messages.len())), more)
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

    /// Applies `flags` to the ids the folder `mailbox` holds, one PATCH
    /// each, returning how many it patched beside each other id's error.
    ///
    /// An empty patch (`$Forwarded` has no Graph field) sends nothing, an
    /// empty PATCH being a wasted request that could still fail.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        let found = self.ids_in_folder(mailbox, ids)?;
        let patch = flag_patch(flags, op);

        if patch == MsgraphMessage::default() {
            return Ok(found.map(|ids| ids.len()));
        }

        let body = serde_json::to_value(&patch).context("Cannot serialize the message patch")?;
        let patched = self.write_each(&found.value, |url| json_request("PATCH", url, &body))?;

        Ok(patched.after(found.errors))
    }

    /// Fetches one message's raw RFC 5322 bytes via `$value`, once a plain
    /// get found it in the folder `mailbox` and read its flags.
    ///
    /// The id names the message account-wide, so a message of another
    /// folder is not found; `$value` carries no `parentFolderId` to check.
    pub fn get_message(
        &mut self,
        mailbox: &str,
        id: &str,
        seen: bool,
    ) -> Result<(Vec<u8>, BTreeSet<Flag>)> {
        let select = "id,parentFolderId,isRead,isDraft,importance,flag,categories";
        let message = self
            .message_get(id, Some(select), None)
            .map_err(|err| read_error(id, err))?
            .response;
        let folder = message.parent_folder_id.as_deref();
        if folder != Some(mailbox) && folder != Some(&*self.resolve_mailbox_id(mailbox)?) {
            return Err(message_not_found(format!(
                "Message `{id}` not found in `{mailbox}`"
            )));
        }

        let raw = self
            .message_get_raw(id)
            .map_err(|err| read_error(id, err))?
            .response;
        let mut flags = flags_from_message(&message);

        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            flags.insert(seen.clone());
            self.message_update(id, &flag_patch(&[seen], FlagOp::Add))?;
        }

        Ok((raw, flags))
    }

    /// Copies the ids the folder `from` holds into `to`, returning how many
    /// it copied beside each other id's error.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let found = self.ids_in_folder(from, ids)?;
        let body = json!({ "destinationId": to });

        let copied = self.write_each(&found.value, |url| {
            json_request("POST", format!("{url}/copy"), &body)
        })?;

        Ok(copied.after(found.errors))
    }

    /// Moves the ids the folder `from` holds into `to`, returning how many
    /// it moved beside each other id's error.
    ///
    /// Moving into `from` itself leaves them as they are.
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

    /// Deletes the ids the folder `mailbox` holds, returning how many it
    /// deleted beside each other id's error.
    ///
    /// From Deleted Items this is a hard delete.
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
    /// A child is created at the top then moved, io-msgraph creating only
    /// top-level folders; a failed move leaves it there, the error says so.
    /// A namesake, even hidden, is `mailbox-duplicate` before any request.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        if let Some(parent) = parent
            && self.holds_child(parent, name)?
        {
            let cause = anyhow!("Microsoft Graph folder `{parent}` holds `{name}`");
            return Err(existing_mailbox(name, cause));
        }

        let folder = MsgraphMailFolder {
            display_name: name.to_string(),
            ..Default::default()
        };

        let created = match self.mail_folder_create(&folder) {
            Ok(created) => created.response,
            Err(err) if parent.is_none() && names_existing_folder(&err) => {
                return Err(existing_mailbox(name, err));
            }
            Err(err) => return Err(err.into()),
        };
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

    /// Whether the folder `parent` holds one named `name`, hidden ones
    /// included, every page read.
    fn holds_child(&mut self, parent: &str, name: &str) -> Result<bool> {
        let params = MsgraphMailFoldersListParams {
            top: Some(100),
            include_hidden_folders: Some(true),
            ..Default::default()
        };
        let mut page = self.mail_child_folders_list(parent, &params)?.response;

        loop {
            if page.value.iter().any(|folder| folder.display_name == name) {
                return Ok(true);
            }

            let Some(next) = page.next_link else {
                return Ok(false);
            };
            page = self.page_get(&next, None)?.response;
        }
    }

    /// Renames the folder `mailbox` to `name`, its parent kept, returning
    /// its unchanged id.
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

/// The well-known folder name carrying `role`, `None` for a role no
/// well-known folder carries.
pub(super) fn well_known_name(role: &MailboxRole) -> Option<&'static str> {
    WELL_KNOWN_FOLDERS
        .iter()
        .find(|(_, known)| known == role)
        .map(|(name, _)| *name)
}

/// The failure of a request reading the message `id`: `message-not-found`
/// when Graph knows no such message, Graph's answer kept in `sources`.
fn read_error(id: &str, err: MsgraphClientStdError) -> anyhow::Error {
    match err {
        MsgraphClientStdError::Send(err)
            if err
                .api()
                .is_some_and(|api| names_no_message(api.status, &api.code)) =>
        {
            unknown_message(id, err)
        }
        err => err.into(),
    }
}

/// Whether Graph refused a folder creation as naming a folder it holds:
/// `409 ErrorFolderExists`.
fn names_existing_folder(err: &MsgraphClientStdError) -> bool {
    matches!(
        err,
        MsgraphClientStdError::Send(err)
            if err.api().is_some_and(|api| api.status == 409 && api.code == "ErrorFolderExists")
    )
}

/// Converts one Graph mail folder named `name` into the unified
/// [`Mailbox`] shape, its total left for [`counts_from`] to fill.
///
/// `totalItemCount` counts contacts and events too (deleted ones land in
/// Deleted Items), so only a 0 is a message count; `unreadItemCount`
/// matched the unread messages live, so it is read as is.
fn mailbox_from(
    name: String,
    folder: MsgraphMailFolder,
    role: Option<MailboxRole>,
    with_counts: bool,
) -> Mailbox {
    let total = (with_counts && folder.total_item_count == Some(0)).then_some(0);
    let unread = folder.unread_item_count.filter(|_| with_counts);

    Mailbox {
        id: folder.id,
        name,
        role,
        total,
        unread,
    }
}

/// The `$batch` requests counting the messages of `folders` by `$count`,
/// each under the folder's index; a folder holding no item is not asked.
fn count_requests(
    user_id: &str,
    folders: &[(String, MsgraphMailFolder)],
) -> Result<Vec<MsgraphBatchRequest>> {
    let user = user_path(user_id)?;
    let mut requests = Vec::new();

    for (index, (_, folder)) in folders.iter().enumerate() {
        if folder.total_item_count == Some(0) {
            continue;
        }

        requests.push(MsgraphBatchRequest {
            id: index.to_string(),
            method: "GET".into(),
            url: format!(
                "/{user}/mailFolders/{}/messages?$select=id&$top=1&$count=true",
                path_segment(&folder.id)?,
            ),
            ..Default::default()
        });
    }

    Ok(requests)
}

/// Fills the totals of `mailboxes` from the [`count_requests`] answers; a
/// failed count is `mailbox-unreadable`, both its counts then `null`.
fn counts_from(
    mailboxes: &mut [Mailbox],
    requests: &[MsgraphBatchRequest],
    mut responses: Vec<MsgraphBatchResponse>,
) -> Vec<ItemError> {
    let mut errors = Vec::new();

    for request in requests {
        let Some(index) = request.id.parse::<usize>().ok() else {
            continue;
        };
        let Some(mailbox) = mailboxes.get_mut(index) else {
            continue;
        };

        let response = responses
            .iter()
            .position(|response| response.id == request.id)
            .map(|position| responses.swap_remove(position));

        let count = match response {
            Some(response) => response
                .parse::<MsgraphPage<Value>>()
                .map_err(anyhow::Error::from)
                .and_then(|page| page.count.context("Graph answered no `@odata.count`")),
            None => Err(anyhow!(
                "Graph batch answer holds no response for this count"
            )),
        };

        match count {
            Ok(count) => mailbox.total = Some(count),
            Err(err) => {
                let err = err.context("Cannot count the messages of the folder");
                let mut error = ItemError::new(Some(&mailbox.id), &err);
                error.code = Some(ErrorCode::MailboxUnreadable);
                errors.push(error);
                mailbox.total = None;
                mailbox.unread = None;
            }
        }
    }

    errors
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

/// Keeps the ids whose response names a message of the folder `mailbox`,
/// beside each other id's error, another folder's being not found.
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

/// Folds a Graph message into the unified [`Envelope`] shape. `size` is
/// `None`, Graph's message resource carrying no raw size.
///
/// `in_reply_to` stays empty: Graph keeps In-Reply-To in
/// `internetMessageHeaders` alone, which a listing does not select.
pub(crate) fn envelope_from(message: MsgraphMessage) -> Envelope {
    let flags = flags_from_message(&message);
    let message_id = message
        .internet_message_id
        .as_deref()
        .and_then(normalize_message_id);
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
        in_reply_to: Vec::new(),
        flags,
        subject: message.subject,
        from,
        to,
        date,
        size: None,
        has_attachment: message.has_attachments,
    }
}

/// Builds the PATCH body applying `flags` under `op`: `Set` drives every
/// flag field and replaces `categories`, `Add`/`Remove` touch those named.
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

/// Derives the unified flag set from a Graph message's scalar fields and
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

/// Converts a Graph recipient into a unified [`Address`].
fn address_from(recipient: MsgraphRecipient) -> Address {
    Address {
        name: recipient.email_address.name,
        email: recipient.email_address.address.unwrap_or_default(),
    }
}

/// Maps the unified `\Flagged` onto a Graph follow-up flag.
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

/// Maps the unified `$Important` onto a Graph importance.
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
        error::{ErrorCode, ItemError},
        msgraph::backend::{
            envelope_from, flag_patch, ids_found, roles_from, well_known_name, written,
        },
        unified::email::{
            flag::{Flag, FlagOp, IanaFlag},
            mailbox::MailboxRole,
        },
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

    /// Graph gives no size of the raw message, so an envelope prints it
    /// `null`, where it printed `0`, an empty message's size.
    #[test]
    fn an_envelope_size_graph_does_not_give_is_null() {
        let message = serde_json::from_value(json!({"id": "m1"})).unwrap();
        let envelope = serde_json::to_value(envelope_from(message)).unwrap();
        assert_eq!(envelope["size"], serde_json::Value::Null, "{envelope}");
    }

    #[test]
    fn a_role_reads_as_its_well_known_name() {
        assert_eq!(well_known_name(&MailboxRole::Trash), Some("deleteditems"));
        assert_eq!(well_known_name(&MailboxRole::Sent), Some("sentitems"));
        assert_eq!(well_known_name(&MailboxRole::All), None);
    }

    fn codes(errors: &[ItemError]) -> Vec<(Option<&str>, Option<ErrorCode>)> {
        errors
            .iter()
            .map(|error| (error.id.as_deref(), error.code))
            .collect()
    }

    /// Another folder's message, a 404 or a 400 `ErrorInvalidIdMalformed` is
    /// not found; another 400 is refused, a missing part coded `null`.
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

#[cfg(test)]
mod count_tests {
    use std::sync::{Arc, Mutex};

    use io_msgraph::v1::client::MsgraphClientStd;
    use serde_json::{Value, json};

    use crate::{
        error::ErrorCode,
        http_script::{Script, ok},
        msgraph::client::MsgraphClient,
    };

    /// A client over a Graph server answering `bodies` in order, each a
    /// 200 carrying JSON, and the requests it reads.
    pub(super) fn graph(bodies: &[Value]) -> (MsgraphClient, Arc<Mutex<Vec<String>>>) {
        let responses = bodies
            .iter()
            .map(|body| ok("application/json", &body.to_string()));
        let (script, requests) = Script::new(responses.collect());
        let inner = MsgraphClientStd::new(script, "token", Default::default());

        (MsgraphClient::over(inner), requests)
    }

    /// The `$batch` answer of the well-known folder lookups, two batches
    /// of four requests at most, the trash alone found.
    fn roles() -> [Value; 2] {
        let missing = |id: &str| {
            let body = json!({"error": {"code": "ErrorItemNotFound"}});
            json!({"id": id, "status": 404, "body": body})
        };
        let trash = json!({"id": "deleteditems", "status": 200, "body": {"id": "D"}});
        let first = [
            missing("inbox"),
            missing("sentitems"),
            missing("drafts"),
            trash,
        ];

        [
            json!({ "responses": first }),
            json!({"responses": [missing("junkemail"), missing("archive")]}),
        ]
    }

    /// `totalItemCount` counts contacts and events too (a test Deleted Items
    /// said 835 for 26 messages), so the total is counted, unread kept.
    #[test]
    fn a_folder_counts_its_messages_not_its_items() {
        let [inbox, junk] = roles();
        let folders = json!({"value": [{
            "id": "D",
            "displayName": "Deleted Items",
            "childFolderCount": 0,
            "unreadItemCount": 5,
            "totalItemCount": 835,
        }]});
        let counts = json!({"responses": [{
            "id": "0",
            "status": 200,
            "body": {"@odata.count": 26, "value": [{"id": "m1"}]},
        }]});
        let (mut client, requests) = graph(&[inbox, junk, folders, counts]);

        let outcome = client.list_mailboxes(true).unwrap();
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        let [trash] = outcome.value.as_slice() else {
            panic!("{:?}", outcome.value);
        };
        assert_eq!((trash.total, trash.unread), (Some(26), Some(5)));

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4, "{requests:#?}");
        let batch = &requests[3];
        let url = "/me/mailFolders/D/messages?$select=id&$top=1&$count=true";
        assert!(batch.contains(&format!(r#""url":"{url}""#)), "{batch}");
        assert!(!batch.contains("isRead"), "{batch}");
    }

    /// A folder whose count Graph refuses is listed without counts beside
    /// its error, and no request counts the unread messages.
    #[test]
    fn a_refused_count_leaves_the_folder_uncounted() {
        let [inbox, junk] = roles();
        let folder = |id: &str, total: u64, unread: u64| {
            json!({
                "id": id,
                "displayName": id,
                "childFolderCount": 0,
                "unreadItemCount": unread,
                "totalItemCount": total,
            })
        };
        let folders = json!({"value": [folder("A", 12, 3), folder("B", 4, 0)]});
        let count = |id: &str, count: u64| {
            let body = json!({"@odata.count": count, "value": []});
            json!({"id": id, "status": 200, "body": body})
        };
        let denied = json!({
            "id": "1",
            "status": 403,
            "body": {"error": {"code": "ErrorAccessDenied", "message": "Denied"}},
        });
        let counts = json!({"responses": [count("0", 10), denied]});
        let (mut client, requests) = graph(&[inbox, junk, folders, counts]);

        let outcome = client.list_mailboxes(true).unwrap();
        let [a, b] = outcome.value.as_slice() else {
            panic!("{:?}", outcome.value);
        };
        assert_eq!((a.total, a.unread), (Some(10), Some(3)));
        assert_eq!((b.total, b.unread), (None, None));

        let [error] = outcome.errors.as_slice() else {
            panic!("{:?}", outcome.errors);
        };
        assert_eq!(error.id.as_deref(), Some("B"));
        assert_eq!(error.code, Some(ErrorCode::MailboxUnreadable));

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4, "{requests:#?}");
        assert!(!requests[3].contains("isRead"), "{}", requests[3]);
    }

    /// Without `--counts`, nothing is counted and nothing asked.
    #[test]
    fn an_uncounted_listing_asks_no_count() {
        let [inbox, junk] = roles();
        let folders = json!({"value": [{"id": "D", "displayName": "Deleted Items"}]});
        let (mut client, requests) = graph(&[inbox, junk, folders]);

        let outcome = client.list_mailboxes(false).unwrap();
        assert_eq!(outcome.value[0].total, None);
        assert_eq!(requests.lock().unwrap().len(), 3);
    }
}

#[cfg(test)]
mod page_tests {
    use serde_json::{Value, json};

    use crate::{
        msgraph::backend::count_tests::graph, unified::email::search::query::SearchEmailsQuery,
    };

    /// A page of `ids`, with the next link Graph answers after an exactly
    /// full last page too.
    fn page(ids: &[&str]) -> Value {
        let value: Vec<Value> = ids.iter().map(|id| json!({"id": id})).collect();
        let link = "https://graph.microsoft.com/v1.0/me/messages?$skiptoken=x";
        json!({"value": value, "@odata.nextLink": link})
    }

    /// Five messages five a page: Graph's next link leads to an empty page,
    /// so none follows, the extra message asked telling so.
    #[test]
    fn an_exactly_full_last_page_has_no_next() {
        let (mut client, requests) = graph(&[page(&["1", "2", "3", "4", "5"])]);
        let listed = client.list_envelopes("A", Some(1), Some(5), false).unwrap();
        assert_eq!(listed.envelopes.len(), 5);
        assert!(!listed.more);

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "{requests:#?}");
        assert!(requests[0].contains("%24top=6"), "{}", requests[0]);

        let (mut client, _) = graph(&[page(&["1", "2", "3"])]);
        let listed = client.list_envelopes("A", Some(1), Some(2), false).unwrap();
        let ids: Vec<&str> = listed.envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["1", "2"]);
        assert!(listed.more);
    }

    /// A search reads one message past its page through the links, so an
    /// exactly full last page has no next one either.
    #[test]
    fn a_search_page_reads_one_message_past_it() {
        let query: SearchEmailsQuery = "subject:report".parse().unwrap();

        let (mut client, requests) = graph(&[page(&["1", "2"]), json!({"value": []})]);
        let found = client
            .search_envelopes("A", Some(&query), Some(1), Some(2), false)
            .unwrap();
        assert_eq!(found.envelopes.len(), 2);
        assert!(!found.more);
        assert!(requests.lock().unwrap()[0].contains("%24top=3"));

        let last = json!({"value": [{"id": "1"}, {"id": "2"}, {"id": "3"}, {"id": "4"}]});
        let (mut client, requests) = graph(&[last]);
        let found = client
            .search_envelopes("A", Some(&query), Some(2), Some(2), false)
            .unwrap();
        let ids: Vec<&str> = found.envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["3", "4"]);
        assert!(!found.more);
        assert!(requests.lock().unwrap()[0].contains("%24top=5"));

        let (mut client, _) = graph(&[page(&["1", "2", "3", "4", "5"])]);
        let found = client
            .search_envelopes("A", Some(&query), Some(2), Some(2), false)
            .unwrap();
        let ids: Vec<&str> = found.envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["3", "4"]);
        assert!(found.more);
    }
}

#[cfg(test)]
mod read_tests {
    use std::sync::{Arc, Mutex};

    use io_msgraph::v1::client::MsgraphClientStd;
    use serde_json::{Value, json};

    use crate::{
        error::ErrorCode,
        http_script::{Script, ok},
        msgraph::client::MsgraphClient,
    };

    /// The raw message `m1`.
    const RAW: &str = "Subject: hi\r\n\r\nbody\r\n";

    /// A client over a Graph server answering `responses` in order, and
    /// the requests it reads.
    fn graph(responses: Vec<Vec<u8>>) -> (MsgraphClient, Arc<Mutex<Vec<String>>>) {
        let (script, requests) = Script::new(responses);
        let inner = MsgraphClientStd::new(script, "token", Default::default());
        (MsgraphClient::over(inner), requests)
    }

    /// A 200 carrying `body` as JSON.
    fn reply(body: Value) -> Vec<u8> {
        ok("application/json", &body.to_string())
    }

    /// The answers resolving `sentitems`: the top folders, none named so,
    /// then the well-known folder, the Sent Items `S`.
    fn folders() -> [Vec<u8>; 2] {
        let folders = json!({"value": [
            {"id": "I", "displayName": "Inbox"},
            {"id": "S", "displayName": "Sent Items"},
        ]});
        let sent = json!({"id": "S", "displayName": "Sent Items"});

        [reply(folders), reply(sent)]
    }

    /// Another folder's message is `message-not-found` though Graph ids are
    /// account-wide, its bytes not fetched, as on IMAP and JMAP.
    #[test]
    fn a_read_of_a_message_another_folder_holds_is_not_found() {
        let message = || reply(json!({"id": "m1", "parentFolderId": "I"}));

        let [listing, sent] = folders();
        let (mut client, requests) = graph(vec![message(), listing, sent]);
        let err = client.get_message("sentitems", "m1", false).unwrap_err();
        let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
        assert_eq!(code, Some(ErrorCode::MessageNotFound), "{err}");
        assert!(err.to_string().contains("`sentitems`"), "{err}");

        let requests = requests.lock().unwrap();
        let select = "/me/messages/m1?%24select=id%2CparentFolderId";
        assert!(requests[0].contains(select), "{}", requests[0]);
        assert!(
            !requests.iter().any(|r| r.contains("$value")),
            "{requests:#?}"
        );

        let (mut client, requests) = graph(vec![message(), ok("text/plain", RAW)]);
        let (raw, _) = client.get_message("I", "m1", false).unwrap();
        assert_eq!(raw, RAW.as_bytes());
        assert_eq!(requests.lock().unwrap().len(), 2);
    }

    /// An id Graph knows no message by is `message-not-found` from the
    /// folder lookup already, nothing more asked.
    #[test]
    fn a_read_of_an_unknown_id_is_not_found() {
        let body = json!({"error": {"code": "ErrorItemNotFound", "message": "Not found"}});
        let body = body.to_string();
        let len = body.len();
        let head = "HTTP/1.1 404 Not Found\r\nContent-Type: application/json";
        let missing = format!("{head}\r\nContent-Length: {len}\r\n\r\n{body}");

        let (mut client, requests) = graph(vec![missing.into_bytes()]);
        let err = client.get_message("I", "m9", false).unwrap_err();
        let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
        assert_eq!(code, Some(ErrorCode::MessageNotFound), "{err}");
        assert_eq!(requests.lock().unwrap().len(), 1);
    }

    /// Graph's answer to a folder creation naming one it holds.
    fn folder_exists() -> Vec<u8> {
        let message = "A folder with the specified name already exists.";
        let body = json!({"error": {"code": "ErrorFolderExists", "message": message}});
        let body = body.to_string();
        let len = body.len();
        let head = "HTTP/1.1 409 Conflict\r\nContent-Type: application/json";
        format!("{head}\r\nContent-Length: {len}\r\n\r\n{body}").into_bytes()
    }

    /// A namesake at the top or under the parent is `mailbox-duplicate`, one
    /// at the top of a folder asked under a parent is not.
    #[test]
    fn a_create_of_an_existing_folder_is_a_duplicate() {
        let code = |err: &anyhow::Error| pimalaya_cli::error::code_of::<ErrorCode>(err);

        let (mut client, _) = graph(vec![folder_exists()]);
        let err = client.create_mailbox("Kid", None).unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::MailboxDuplicate), "{err:#}");
        assert!(format!("{err:#}").contains("ErrorFolderExists"), "{err:#}");

        let children = reply(json!({"value": [{"id": "K", "displayName": "Kid"}]}));
        let (mut client, requests) = graph(vec![children]);
        let err = client.create_mailbox("Kid", Some("I")).unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::MailboxDuplicate), "{err:#}");
        assert_eq!(
            err.to_string(),
            "Cannot create mailbox `Kid`: it exists already"
        );
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "{requests:#?}");
        assert!(requests[0].starts_with("GET "), "{}", requests[0]);
        assert!(requests[0].contains("/mailFolders/I/childFolders"));

        let none = || reply(json!({"value": []}));
        let (mut client, _) = graph(vec![none(), folder_exists()]);
        let err = client.create_mailbox("Kid", Some("I")).unwrap_err();
        assert_eq!(code(&err), None, "{err:#}");

        let created = reply(json!({"id": "N", "displayName": "Kid"}));
        let moved = reply(json!({"id": "M", "displayName": "Kid"}));
        let (mut client, _) = graph(vec![none(), created, moved]);
        assert_eq!(client.create_mailbox("Kid", Some("I")).unwrap(), "M");
    }
}
