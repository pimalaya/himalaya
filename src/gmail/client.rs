//! # Gmail client
//!
//! The wrapper around io-gmail's blocking client every Gmail-specific
//! subcommand receives, plus the credential helper the shared client and
//! the account checker take.
//!
//! The unified API covers the operations every mail backend names, where
//! the `gmail` command reaches for the full REST surface through this
//! wrapper.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_gmail::v1::{
    client::{GmailClientStd as Inner, GmailClientStdConnectOptions, GmailClientStdError},
    query::to_query_pairs,
    send::{GmailApiError, GmailSend, GmailSendError, gmail_api_url},
};
use pimalaya_config::secret::SecretResolver;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    config::{
        AccountConfig, Config,
        gmail::{GmailAuthConfig, GmailConfig},
        proxy::ProxyConfig,
    },
    email::mailbox::{MailboxIndex, MailboxRole},
    error::{ErrorCode, mailbox_not_found},
    shared::refusal::Refusal,
};

/// One page of a Gmail listing as Gmail sent it, its cursor lifted out.
pub struct GmailRawPage {
    /// The items of the page, each as Gmail sent it.
    pub items: Vec<Value>,
    /// The other keys of the page, such as `resultSizeEstimate` or
    /// `historyId`.
    pub rest: Map<String, Value>,
    /// Gmail's `nextPageToken`, `None` on the last page.
    pub next_page: Option<String>,
}

/// A live Gmail client and the label index it caches.
pub struct GmailClient {
    inner: Inner,
    /// The labels [`Self::resolve_mailbox_id`] maps names and roles
    /// through, fetched once and cached for the client's lifetime.
    label_index: Option<MailboxIndex>,
}

impl GmailClient {
    /// Opens a TLS connection to the Gmail REST API with the configured
    /// bearer credential and user id.
    pub fn new(config: GmailConfig) -> Result<Self> {
        let mut resolver = SecretResolver::new();
        let token = gmail_token(config.auth, &mut resolver)?;
        let options = GmailClientStdConnectOptions {
            tls: config.tls.into_tls(config.alpn)?,
            proxy: ProxyConfig::resolve(config.proxy, &mut resolver)?,
            user_id: config.user_id,
        };
        let inner = Inner::connect(token.expose_secret(), options)?;
        Ok(Self {
            inner,
            label_index: None,
        })
    }

    /// Maps a label id, name or role onto its opaque Gmail label id.
    ///
    /// It lives here so every backend method stays a pure id consumer.
    /// A value naming no label is refused, Gmail taking no label by any
    /// other name.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        let index = self.label_index()?;
        let id = index.resolve(mailbox)?;

        if !index.0.iter().any(|label| label.id == id) {
            return Err(mailbox_not_found(format!(
                "Gmail has no label `{mailbox}`, by id, name or role"
            )));
        }

        Ok(id)
    }

    /// The id of the label carrying `role`, `None` when none does.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        self.label_index()?.with_role(role)
    }

    /// Drops the cached labels, so the next lookup sees a label created,
    /// renamed or deleted since.
    pub fn forget_mailboxes(&mut self) {
        self.label_index = None;
    }

    /// One page of the listing at `users/<user>/<collection>`, queried by
    /// `params`, as Gmail sent it: the items under `collection` (`[]` when
    /// Gmail leaves the key out of an empty page), the rest of the page
    /// beside them and the `nextPageToken` taken out as the cursor.
    pub fn raw_list<P: Serialize>(&mut self, collection: &str, params: &P) -> Result<GmailRawPage> {
        let pairs = to_query_pairs(params).map_err(GmailSendError::from)?;
        self.raw_list_at(&[collection], collection, pairs)
    }

    /// The listing at `users/<user>/<path>`, queried by `pairs`, as Gmail
    /// sent it, its items under `collection`, as [`Self::raw_list`] reads
    /// it: the settings listings, whose path and key differ.
    pub fn raw_list_at(
        &mut self,
        path: &[&str],
        collection: &str,
        pairs: Vec<(String, String)>,
    ) -> Result<GmailRawPage> {
        let mut rest = self.raw_get_with(path, pairs)?;

        let next_page = match rest.remove("nextPageToken") {
            Some(Value::String(token)) => Some(token),
            _ => None,
        };
        let items = match rest.remove(collection) {
            Some(Value::Array(items)) => items,
            _ => Vec::new(),
        };

        Ok(GmailRawPage {
            items,
            rest,
            next_page,
        })
    }

    /// The resource at `users/<user>/<path>`, as Gmail sent it.
    pub fn raw_get(&mut self, path: &[&str]) -> Result<Map<String, Value>> {
        self.raw_get_with(path, Vec::new())
    }

    /// The resource at `users/<user>/<path>`, queried by `pairs`, as Gmail
    /// sent it.
    pub fn raw_get_with(
        &mut self,
        path: &[&str],
        pairs: Vec<(String, String)>,
    ) -> Result<Map<String, Value>> {
        let user = self.user_id.clone();
        let mut segments = vec!["users", user.as_str()];
        segments.extend_from_slice(path);

        let mut url = gmail_api_url(&segments)?;
        if !pairs.is_empty() {
            url.query_pairs_mut().extend_pairs(pairs);
        }

        let send = GmailSend::<Map<String, Value>>::get(&self.auth, url)?;
        Ok(self.run(send)?.response)
    }

    fn label_index(&mut self) -> Result<&MailboxIndex> {
        if self.label_index.is_none() {
            let labels = self.list_mailboxes(false)?.value;
            self.label_index = Some(MailboxIndex(labels));
        }

        Ok(self.label_index.get_or_insert_default())
    }
}

#[cfg(test)]
impl GmailClient {
    /// A client over `inner`, such as one on a scripted stream, its
    /// labels not fetched yet.
    pub(crate) fn over(inner: Inner) -> Self {
        Self {
            inner,
            label_index: None,
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

impl Deref for GmailClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for GmailClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the Gmail client of an already-resolved account, returning it
/// beside the merged [`Account`].
///
/// Bails when the account declares no `[gmail]` block.
pub fn build_gmail_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, GmailClient)> {
    let gmail_config = account_config
        .gmail
        .take()
        .ok_or_else(|| anyhow!("Gmail config is missing for account `{name}`"))?;
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
    let client = GmailClient::new(gmail_config)?;
    Ok((account, client))
}

/// Resolves the configuration into the bare OAuth 2.0 token, the client
/// adding the `Bearer ` prefix itself.
///
/// The token goes through `resolver`, so an account naming one command
/// here and in another block spawns it once.
pub fn gmail_token(config: GmailAuthConfig, resolver: &mut SecretResolver) -> Result<SecretString> {
    Ok(resolver.resolve(config.token)?)
}

/// The code of a Gmail refusal in an error chain, as [`refusal_of`] reads
/// it.
pub(crate) fn refusal_code(cause: &(dyn std::error::Error + 'static)) -> Option<ErrorCode> {
    let err = match cause.downcast_ref::<GmailClientStdError>() {
        Some(GmailClientStdError::Send(err)) => err,
        Some(_) => return None,
        None => cause.downcast_ref::<GmailSendError>()?,
    };

    refusal_of(err.api()?)
}

/// The code of what Gmail refused: a rate limit (a 429, or a 403 naming
/// one, `rateLimitExceeded`, `userRateLimitExceeded` or `quotaExceeded`)
/// and the daily quota ([`is_daily_limit`]) are `rate-limited`, as the
/// family codes a Google quota, any other 403 `permission-denied`.
pub(crate) fn refusal_of(err: &GmailApiError) -> Option<ErrorCode> {
    if err.is_rate_limited() || is_daily_limit(err) {
        Some(ErrorCode::RateLimited)
    } else if err.status == 403 {
        Some(ErrorCode::PermissionDenied)
    } else {
        None
    }
}

/// Whether Google refused the daily quota: the reason `dailyLimitExceeded`,
/// or a message naming a daily limit or a quota per day, the daily limit
/// of unauthenticated use aside, which a missing credential meets.
fn is_daily_limit(err: &GmailApiError) -> bool {
    let text: String = err
        .message
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    let named = text.contains("dailylimit") || text.contains("perday");

    err.has_reason("dailyLimitExceeded") || (named && !text.contains("unauthenticated"))
}

/// The refusal of a Gmail error in a chain, for the `gmail` commands: the
/// status, the first reason (else the canonical status) and the message
/// of what Gmail answered.
pub(crate) fn refusal(cause: &(dyn std::error::Error + 'static)) -> Option<Refusal> {
    let err = match cause.downcast_ref::<GmailClientStdError>() {
        Some(GmailClientStdError::Send(err)) => err,
        Some(_) => return None,
        None => cause.downcast_ref::<GmailSendError>()?,
    };
    let err = err.api()?;
    let reason = err.reasons.first().or(err.google_status.as_ref());

    Some(Refusal {
        status: Some(err.status),
        reason: reason.cloned(),
        message: err.message.clone(),
        code: None,
        retry_after: err.retry_after.map(|delay| delay.as_secs()),
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Error;
    use io_gmail::v1::{
        client::GmailClientStdError,
        send::{GmailApiError, GmailSendError},
    };

    use crate::error::{self, ErrorCode};

    fn refused(status: u16, reason: &str) -> Error {
        let err = GmailApiError {
            status,
            message: String::from("Refused"),
            reasons: vec![reason.into()],
            ..Default::default()
        };
        let err = GmailClientStdError::Send(GmailSendError::Api(err));
        Error::from(err).context("Cannot list Gmail labels")
    }

    /// A whole command Gmail refused is coded by what it answered: a rate
    /// limit `rate-limited`, another 403 `permission-denied`, the rest
    /// uncoded.
    #[test]
    fn a_whole_refusal_is_coded() {
        let code = |status, reason| error::code(&refused(status, reason));

        assert_eq!(code(429, "rateLimitExceeded"), Some(ErrorCode::RateLimited));
        assert_eq!(
            code(403, "userRateLimitExceeded"),
            Some(ErrorCode::RateLimited)
        );
        assert_eq!(
            code(403, "insufficientPermissions"),
            Some(ErrorCode::PermissionDenied)
        );
        assert_eq!(code(500, "backendError"), None);
    }

    /// A daily quota is a rate limit as the family codes it, though
    /// waiting a minute does not lift it, where it was `permission-denied`.
    #[test]
    fn a_daily_limit_is_rate_limited() {
        let code = |status, reason| error::code(&refused(status, reason));

        assert_eq!(
            code(403, "dailyLimitExceeded"),
            Some(ErrorCode::RateLimited)
        );
    }

    /// A label named by no id, name nor role is `mailbox-not-found`.
    #[test]
    fn a_missing_label_is_coded() {
        use crate::{email::mailbox::MailboxIndex, gmail::client::GmailClient};

        let (mut client, requests) = GmailClient::scripted(&[]);
        client.label_index = Some(MailboxIndex(Vec::new()));

        let err = client.resolve_mailbox_id("Nope").unwrap_err();
        let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
        assert_eq!(code, Some(ErrorCode::MailboxNotFound), "{err}");
        assert!(requests.lock().unwrap().is_empty());
    }
}
