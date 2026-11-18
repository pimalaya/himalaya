//! # Microsoft Graph client
//!
//! The wrapper around io-msgraph's blocking client every Graph-specific
//! subcommand receives, plus the credential helper the shared client and
//! the account checker take.
//!
//! The unified API covers the operations every mail backend names, where
//! the `msgraph` command reaches for the Graph mail surface through this
//! wrapper.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
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
    email::mailbox::{MailboxIndex, MailboxRole},
    error::{ErrorCode, mailbox_not_found, unknown_mailbox},
    msgraph::backend::well_known_role,
    shared::refusal::Refusal,
};

/// A live Microsoft Graph client and the folder index it caches.
pub struct MsgraphClient {
    inner: Inner,
    /// The folders [`Self::resolve_mailbox_id`] maps names and roles
    /// through, fetched once and cached for the client's lifetime.
    folder_index: Option<MailboxIndex>,
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
            folder_index: None,
        })
    }

    /// Maps a folder id, path or role onto its opaque Graph folder id.
    ///
    /// A well-known name (`msgfolderroot`, `outbox`) or a folder id the
    /// listing does not hold (a hidden folder) is asked of Graph; any
    /// other value, or one Graph knows no folder by, is refused. It lives
    /// here so every backend method stays a pure id consumer.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        let index = self.folder_index()?;
        let id = index.resolve(mailbox)?;

        if index.0.iter().any(|m| m.id == id) {
            return Ok(id);
        }

        // NOTE: a well-known name maps onto the folder carrying its role,
        // so it compares equal to that folder's id.
        if let Some(role) = well_known_role(&id)
            && let Some(id) = index.with_role(&role)?
        {
            return Ok(id);
        }

        if !may_name_folder(&id) {
            return Err(mailbox_not_found(format!("Mailbox `{mailbox}` not found")));
        }

        match self.mail_folder_get(&id) {
            Ok(folder) => Ok(folder.response.id),
            Err(MsgraphClientStdError::Send(err)) if names_no_folder(&err) => Err(unknown_mailbox(
                format!("Mailbox `{mailbox}` not found"),
                err,
            )),
            Err(err) => Err(err.into()),
        }
    }

    /// The id of the folder carrying `role`, `None` when none does.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        self.folder_index()?.with_role(role)
    }

    /// Drops the cached folders, so the next lookup sees a folder created,
    /// renamed or deleted since.
    pub fn forget_mailboxes(&mut self) {
        self.folder_index = None;
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

    fn folder_index(&mut self) -> Result<&MailboxIndex> {
        if self.folder_index.is_none() {
            let folders = self.list_mailboxes(false)?.value;
            self.folder_index = Some(MailboxIndex(folders));
        }

        Ok(self.folder_index.get_or_insert_default())
    }
}

#[cfg(test)]
impl MsgraphClient {
    /// A client over `inner`, such as one on a scripted stream, its
    /// folders not fetched yet.
    pub(crate) fn over(inner: Inner) -> Self {
        Self {
            inner,
            folder_index: None,
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

/// Whether `value` may name a Graph folder: a well-known name, or shaped
/// like a folder id (long, base64url).
fn may_name_folder(value: &str) -> bool {
    let id_like = value.len() >= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_=".contains(&byte));

    id_like || WELL_KNOWN_NAMES.contains(&value.to_ascii_lowercase().as_str())
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
    use anyhow::Error;
    use io_msgraph::v1::{
        client::MsgraphClientStdError,
        send::{MsgraphApiError, MsgraphSendError},
    };

    use crate::{
        error::{self, ErrorCode},
        msgraph::client::{may_name_folder, names_no_folder},
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
    fn only_a_well_known_name_or_an_id_shape_is_asked_of_graph() {
        assert!(may_name_folder("msgfolderroot"));
        assert!(may_name_folder("Outbox"));
        assert!(may_name_folder(
            "AAMkADczZWY5NjZhLTY4NzMtNGM4OS1hZGZiLWZmNzdiNGU3YzUxZgAuAAAAAAAwyvki6t3BQokRIq4u7tKhAQC_mCAVB5-SQqiAnuFDrYg6AAAAAAELAAA="
        ));
        assert!(!may_name_folder("No-Such-Folder"));
        assert!(!may_name_folder("Himalaya-Test-MS-A/Sub"));
        assert!(!may_name_folder("a/b?c#d"));
    }

    /// A folder named by no id, path nor role is `mailbox-not-found`,
    /// and so is an id Graph answers it knows no folder by, its answer
    /// kept.
    #[test]
    fn a_missing_folder_is_coded() {
        use io_msgraph::v1::client::MsgraphClientStd;

        use crate::{
            email::mailbox::MailboxIndex,
            msgraph::client::MsgraphClient,
            shared::http_script::{Script, refused},
        };

        let body = r#"{"error":{"code":"ErrorItemNotFound","message":"Not found."}}"#;
        let (script, _) = Script::new(vec![refused(404, body)]);
        let inner = MsgraphClientStd::new(script, "token", Default::default());
        let mut client = MsgraphClient::over(inner);
        client.folder_index = Some(MailboxIndex(Vec::new()));

        for name in ["No-Such-Folder", "AAMkADczZWY5NjZhLTY4NzMtNGM4OS1hZGZiAAA="] {
            let err = client.resolve_mailbox_id(name).unwrap_err();
            let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
            assert_eq!(code, Some(ErrorCode::MailboxNotFound), "{name}: {err}");
        }
    }
}
