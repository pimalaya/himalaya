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

use anyhow::{Result, anyhow, bail};
use io_gmail::v1::{
    client::{GmailClientStd as Inner, GmailClientStdConnectOptions, GmailClientStdError},
    send::{GmailApiError, GmailSendError},
};
use pimalaya_config::secret::SecretResolver;
use secrecy::{ExposeSecret, SecretString};

use crate::{
    account::context::Account,
    config::{
        AccountConfig, Config,
        gmail::{GmailAuthConfig, GmailConfig},
        proxy::ProxyConfig,
    },
    email::mailbox::{MailboxIndex, MailboxRole},
    error::ErrorCode,
};

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
            tls: config.tls.into_tls(config.alpn),
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
            bail!("Gmail has no label `{mailbox}`, by id, name or role");
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

    fn label_index(&mut self) -> Result<&MailboxIndex> {
        if self.label_index.is_none() {
            let labels = self.list_mailboxes(false)?.value;
            self.label_index = Some(MailboxIndex(labels));
        }

        Ok(self.label_index.get_or_insert_default())
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
/// is `rate-limited`, any other 403 `permission-denied`.
pub(crate) fn refusal_of(err: &GmailApiError) -> Option<ErrorCode> {
    if err.is_rate_limited() {
        Some(ErrorCode::RateLimited)
    } else if err.status == 403 {
        Some(ErrorCode::PermissionDenied)
    } else {
        None
    }
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
}
