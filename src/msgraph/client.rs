//! # Microsoft Graph client
//!
//! The wrapper around io-msgraph's blocking client every Graph-specific
//! subcommand receives, plus the credential helper the shared client and
//! the account checker take.
//!
//! The unified API covers the operations every mail backend names, where
//! the `msgraph` command reaches for the Graph mail surface through this
//! wrapper.

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use anyhow::{Result, anyhow, bail};
use io_msgraph::v1::{
    client::{MsgraphClientStd as Inner, MsgraphClientStdConnectOptions, MsgraphClientStdError},
    send::{MSGRAPH_API_BASE, MsgraphSend, MsgraphSendError, path_segment, user_path},
};
use pimalaya_config::secret::SecretResolver;
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use url::Url;

use crate::{
    account::context::Account,
    config::{
        AccountConfig, Config,
        msgraph::{MsgraphAuthConfig, MsgraphConfig},
        proxy::ProxyConfig,
    },
    email::mailbox::MailboxRole,
    error::{ErrorCode, mailbox_not_found, unknown_mailbox},
    msgraph::backend::well_known_name,
    shared::refusal::Refusal,
};

/// A live Microsoft Graph client and the folder ids it resolved.
pub struct MsgraphClient {
    inner: Inner,
    /// The folder id each value [`Self::resolve_mailbox_id`] took maps
    /// onto, kept until a folder write.
    mailbox_ids: HashMap<String, String>,
}

impl MsgraphClient {
    /// Opens a TLS connection to the Microsoft Graph API
    /// (`https://graph.microsoft.com`) with the configured bearer
    /// credential and user id.
    pub fn new(config: MsgraphConfig) -> Result<Self> {
        let mut resolver = SecretResolver::new();
        let token = msgraph_token(config.auth, &mut resolver)?;
        let options = MsgraphClientStdConnectOptions {
            tls: config.tls.into_tls(config.alpn)?,
            proxy: ProxyConfig::resolve(config.proxy, &mut resolver)?,
            user_id: config.user_id,
        };
        let inner = Inner::connect(token.expose_secret(), options)?;
        Ok(Self {
            inner,
            mailbox_ids: HashMap::new(),
        })
    }

    /// Maps a folder id, path or role onto its opaque Graph folder id.
    ///
    /// A value shaped like a folder id is asked of Graph first, which
    /// reaches a hidden folder; then a path, only the folders leading to
    /// it listed; then a role or a well-known name (`msgfolderroot`,
    /// `outbox`), asked of Graph. Any other value, or one Graph knows no
    /// folder by, is refused. It lives here so every backend method stays
    /// a pure id consumer.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        if let Some(id) = self.mailbox_ids.get(mailbox) {
            return Ok(id.clone());
        }

        let id = self.lookup_mailbox_id(mailbox)?;
        self.mailbox_ids.insert(mailbox.to_string(), id.clone());
        Ok(id)
    }

    /// The id of the folder carrying `role`, `None` when none does.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        let Some(name) = well_known_name(role) else {
            return Ok(None);
        };

        Ok(self.folder_id(name)?.ok())
    }

    /// Drops the resolved folder ids, so the next lookup sees a folder
    /// created, renamed or deleted since.
    pub fn forget_mailboxes(&mut self) {
        self.mailbox_ids.clear();
    }

    /// The resource at `segments` under the mailbox owner (`me` or
    /// `users/<id>`), each segment percent-encoded, none naming the owner
    /// itself, as Graph sent it.
    pub fn raw_get(&mut self, segments: &[&str]) -> Result<Value> {
        let mut path = user_path(&self.user_id)?;
        for segment in segments {
            path.push('/');
            path.push_str(&path_segment(segment)?);
        }

        let url = Url::parse(MSGRAPH_API_BASE)?.join(&path)?;
        let auth = self.auth.clone();
        Ok(self.run(MsgraphSend::<Value>::get(&auth, url))?.response)
    }

    fn lookup_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        let mut unknown = None;

        if is_folder_id(mailbox) {
            match self.folder_id(mailbox)? {
                Ok(id) => return Ok(id),
                Err(err) => unknown = Some(err),
            }
        }

        let mut named = Vec::new();
        self.list_folders_under(None, "", Some(mailbox), &mut named)?;

        if let [(_, first), others @ ..] = named.as_slice() {
            if others.is_empty() {
                return Ok(first.id.clone());
            }

            let others: Vec<&str> = others.iter().map(|(_, f)| f.id.as_str()).collect();
            bail!(
                "Several mailboxes are named `{mailbox}` ({}, {}): pass the mailbox id",
                first.id,
                others.join(", ")
            );
        }

        let well_known = match MailboxRole::known(mailbox) {
            Some(role) => well_known_name(&role),
            None => WELL_KNOWN_NAMES
                .into_iter()
                .find(|name| name.eq_ignore_ascii_case(mailbox)),
        };

        if let Some(name) = well_known {
            match self.folder_id(name)? {
                Ok(id) => return Ok(id),
                Err(err) => unknown = Some(err),
            }
        }

        let message = format!("Mailbox `{mailbox}` not found");
        match unknown {
            Some(err) => Err(unknown_mailbox(message, err)),
            None => Err(mailbox_not_found(message)),
        }
    }

    /// The id of the folder Graph knows by `value`, or Graph's answer when
    /// it knows none.
    fn folder_id(&mut self, value: &str) -> Result<Result<String, MsgraphSendError>> {
        match self.mail_folder_get(value) {
            Ok(folder) => Ok(Ok(folder.response.id)),
            Err(MsgraphClientStdError::Send(err)) if names_no_folder(&err) => Ok(Err(err)),
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(test)]
impl MsgraphClient {
    /// A client over `inner`, such as one on a scripted stream, its
    /// folders not fetched yet.
    pub(crate) fn over(inner: Inner) -> Self {
        Self {
            inner,
            mailbox_ids: HashMap::new(),
        }
    }

    /// A client whose requests are answered by the JSON `bodies`, in
    /// order, beside the requests it sends.
    pub(crate) fn scripted(
        bodies: &[Value],
    ) -> (Self, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        let bodies = bodies
            .iter()
            .map(|body| crate::shared::http_script::ok("application/json", &body.to_string()));
        let (script, requests) = crate::shared::http_script::Script::new(bodies.collect());
        let inner = Inner::new(script, "token", Default::default());
        (Self::over(inner), requests)
    }
}

impl Deref for MsgraphClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MsgraphClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the Microsoft Graph client for an already-resolved account:
/// takes the `[msgraph]` block out of `account_config` and builds the
/// merged [`Account`]. Bails when the account has no `[msgraph]` block.
pub fn build_msgraph_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, MsgraphClient)> {
    let msgraph_config = account_config
        .msgraph
        .take()
        .ok_or_else(|| anyhow!("Microsoft Graph config is missing for account `{name}`"))?;
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
    let client = MsgraphClient::new(msgraph_config)?;
    Ok((account, client))
}

/// The well-known folder names Graph v1.0 accepts in place of an id.
const WELL_KNOWN_NAMES: [&str; 17] = [
    "archive",
    "clutter",
    "conflicts",
    "conversationhistory",
    "deleteditems",
    "drafts",
    "inbox",
    "junkemail",
    "localfailures",
    "msgfolderroot",
    "outbox",
    "recoverableitemsdeletions",
    "scheduled",
    "searchfolders",
    "sentitems",
    "serverfailures",
    "syncissues",
];

/// Whether `value` is shaped like a Graph folder id: long, base64url.
fn is_folder_id(value: &str) -> bool {
    value.len() >= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_=".contains(&byte))
}

/// Whether Graph's answer to a folder lookup means no such folder: an
/// unknown id (404), a malformed one (400 `ErrorInvalidIdMalformed`), or a
/// value no path segment can carry.
fn names_no_folder(err: &MsgraphSendError) -> bool {
    let malformed = err
        .api()
        .is_some_and(|err| err.status == 400 && err.code == "ErrorInvalidIdMalformed");

    malformed || err.status() == Some(404) || matches!(err, MsgraphSendError::InvalidRequest(_))
}

/// Whether Graph's answer to a message request means no such message: an
/// unknown id (404) or a malformed one (400 `ErrorInvalidIdMalformed`).
pub(crate) fn names_no_message(status: u16, code: &str) -> bool {
    status == 404 || (status == 400 && code == "ErrorInvalidIdMalformed")
}

/// The code of a Graph refusal in an error chain, as [`refusal_of`] reads
/// it.
pub(crate) fn refusal_code(cause: &(dyn std::error::Error + 'static)) -> Option<ErrorCode> {
    let err = match cause.downcast_ref::<MsgraphClientStdError>() {
        Some(MsgraphClientStdError::Send(err)) => err,
        Some(_) => return None,
        None => cause.downcast_ref::<MsgraphSendError>()?,
    };
    let err = err.api()?;

    refusal_of(err.status, &err.code)
}

/// The code of what Graph refused, by its status and its `error.code`:
/// `ErrorQuotaExceeded` or a 507 is `quota-exceeded` whatever the
/// status, then 429 `rate-limited` and 403 `permission-denied`.
pub(crate) fn refusal_of(status: u16, code: &str) -> Option<ErrorCode> {
    match (status, code) {
        (_, "ErrorQuotaExceeded") | (507, _) => Some(ErrorCode::QuotaExceeded),
        (429, _) => Some(ErrorCode::RateLimited),
        (403, _) => Some(ErrorCode::PermissionDenied),
        _ => None,
    }
}

/// The refusal of a Graph error in a chain, for the `msgraph` commands:
/// the status, `error.code` and `error.message` of what Graph answered.
pub(crate) fn refusal(cause: &(dyn std::error::Error + 'static)) -> Option<Refusal> {
    let err = match cause.downcast_ref::<MsgraphClientStdError>() {
        Some(MsgraphClientStdError::Send(err)) => err,
        Some(_) => return None,
        None => cause.downcast_ref::<MsgraphSendError>()?,
    };
    let err = err.api()?;

    Some(Refusal {
        status: Some(err.status),
        reason: Some(err.code.clone()).filter(|code| !code.is_empty()),
        message: err.message.clone(),
        code: None,
        retry_after: err.retry_after.map(|delay| delay.as_secs()),
    })
}

/// Resolves a [`MsgraphAuthConfig`] into the bare OAuth 2.0 bearer token;
/// the Microsoft Graph client adds the `Bearer ` prefix itself.
///
/// The token goes through `resolver`, so an account naming one command
/// here and in another block spawns it once.
pub fn msgraph_token(
    config: MsgraphAuthConfig,
    resolver: &mut SecretResolver,
) -> Result<SecretString> {
    Ok(resolver.resolve(config.token)?)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use anyhow::Error;
    use io_msgraph::v1::{
        client::{MsgraphClientStd, MsgraphClientStdError},
        send::{MsgraphApiError, MsgraphSendError},
    };
    use serde_json::{Value, json};

    use crate::{
        email::mailbox::MailboxRole,
        error::{self, ErrorCode},
        msgraph::client::{MsgraphClient, is_folder_id, names_no_folder},
        shared::http_script::{Script, ok, refused as refused_with},
    };

    fn refused(status: u16, code: &str) -> MsgraphSendError {
        MsgraphApiError {
            status,
            code: code.into(),
            ..Default::default()
        }
        .into()
    }

    /// A 400 names no folder only when Graph codes the id malformed.
    #[test]
    fn only_a_malformed_id_or_a_404_names_no_folder() {
        assert!(names_no_folder(&refused(404, "ErrorItemNotFound")));
        assert!(names_no_folder(&refused(400, "ErrorInvalidIdMalformed")));
        assert!(!names_no_folder(&refused(400, "BadRequest")));
        assert!(!names_no_folder(&refused(403, "ErrorAccessDenied")));
    }

    /// A whole command Graph refused is coded by what it answered, through
    /// the context a command adds: the 429 of a mailbox throttling its
    /// concurrent requests `rate-limited`.
    #[test]
    fn a_whole_refusal_is_coded() {
        let code = |status, code| {
            let err = MsgraphClientStdError::Send(refused(status, code));
            error::code(&Error::from(err).context("Cannot list Graph messages"))
        };

        assert_eq!(
            code(429, "MailboxConcurrency"),
            Some(ErrorCode::RateLimited)
        );
        assert_eq!(
            code(403, "ErrorAccessDenied"),
            Some(ErrorCode::PermissionDenied)
        );
        assert_eq!(
            code(403, "ErrorQuotaExceeded"),
            Some(ErrorCode::QuotaExceeded)
        );
        assert_eq!(
            code(507, "ErrorInternalServerError"),
            Some(ErrorCode::QuotaExceeded)
        );
        assert_eq!(code(500, "ErrorInternalServerError"), None);
    }

    #[test]
    fn only_an_id_shape_is_asked_of_graph_as_an_id() {
        assert!(is_folder_id(
            "AAMkADczZWY5NjZhLTY4NzMtNGM4OS1hZGZiLWZmNzdiNGU3YzUxZgAuAAAAAAAwyvki6t3BQokRIq4u7tKhAQC_mCAVB5-SQqiAnuFDrYg6AAAAAAELAAA="
        ));
        assert!(!is_folder_id("msgfolderroot"));
        assert!(!is_folder_id("No-Such-Folder"));
        assert!(!is_folder_id("Himalaya-Test-MS-A/Sub"));
        assert!(!is_folder_id("a/b?c#d"));
    }

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

    /// Graph's answer to a folder it knows none by.
    fn missing() -> Vec<u8> {
        let body = r#"{"error":{"code":"ErrorItemNotFound","message":"Not found."}}"#;
        refused_with(404, body)
    }

    /// A folder named by no id, path nor role is `mailbox-not-found`,
    /// and so is an id Graph answers it knows no folder by, its answer
    /// kept.
    #[test]
    fn a_missing_folder_is_coded() {
        let none = || reply(json!({"value": []}));
        let (mut client, _) = graph(vec![none(), missing(), none()]);

        for name in ["No-Such-Folder", "AAMkADczZWY5NjZhLTY4NzMtNGM4OS1hZGZiAAA="] {
            let err = client.resolve_mailbox_id(name).unwrap_err();
            let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
            assert_eq!(code, Some(ErrorCode::MailboxNotFound), "{name}: {err}");
        }
    }

    /// A path lists the folders leading to it alone: the top, then the
    /// children of `Projets`, never those of a folder off the path, nor
    /// the well-known folders, nor the folders again once resolved.
    #[test]
    fn a_path_walks_only_the_folders_leading_to_it() {
        let top = json!({"value": [
            {"id": "O", "displayName": "Other", "childFolderCount": 20},
            {"id": "P", "displayName": "Projets", "childFolderCount": 1},
        ]});
        let children = json!({"value": [{"id": "A", "displayName": "Alpha"}]});
        let (mut client, requests) = graph(vec![reply(top), reply(children)]);

        assert_eq!(client.resolve_mailbox_id("Projets/Alpha").unwrap(), "A");
        assert_eq!(client.resolve_mailbox_id("Projets/Alpha").unwrap(), "A");

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2, "{requests:#?}");
        assert!(requests[1].contains("/mailFolders/P/childFolders"));
        assert!(!requests.iter().any(|r| r.contains("$batch")));
    }

    /// Two folders one path names, a top folder whose name holds the
    /// delimiter and a nested one, are refused, naming both.
    #[test]
    fn a_path_two_folders_carry_is_refused() {
        let top = json!({"value": [
            {"id": "X", "displayName": "a/b"},
            {"id": "Y", "displayName": "a", "childFolderCount": 1},
        ]});
        let children = json!({"value": [{"id": "Z", "displayName": "b"}]});
        let (mut client, _) = graph(vec![reply(top), reply(children)]);

        let err = client.resolve_mailbox_id("a/b").unwrap_err().to_string();
        assert!(err.contains("X") && err.contains("Z"), "{err}");
    }

    /// A role no folder is named after is asked of its well-known folder,
    /// one request, and a role the mailbox lacks names none.
    #[test]
    fn a_role_is_asked_of_its_well_known_folder() {
        let top = json!({"value": [{"id": "I", "displayName": "Inbox"}]});
        let sent = json!({"id": "S", "displayName": "Sent Items"});
        let trash = json!({"id": "D", "displayName": "Deleted Items"});
        let responses = vec![reply(top), reply(sent), reply(trash), missing()];
        let (mut client, requests) = graph(responses);

        assert_eq!(client.resolve_mailbox_id("sent").unwrap(), "S");
        let trash = client.role_mailbox_id(&MailboxRole::Trash).unwrap();
        assert_eq!(trash.as_deref(), Some("D"));
        let archive = client.role_mailbox_id(&MailboxRole::Archive).unwrap();
        assert_eq!(archive, None);

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4, "{requests:#?}");
        assert!(requests[1].contains("/mailFolders/sentitems"));
        assert!(requests[2].contains("/mailFolders/deleteditems"));
        assert!(requests[3].contains("/mailFolders/archive"));
    }
}
