use std::{collections::HashMap, fs, path::Path, path::PathBuf};

use anyhow::{Context, Result};
use comfy_table::ContentArrangement;
use pimalaya_config::{
    secret::{Secret, SecretError},
    toml::{shell_expanded_string, TomlConfig},
};
use pimalaya_stream::{
    sasl::{Sasl, SaslAnonymous, SaslLogin, SaslMechanism, SaslPlain},
    std::tls::{Rustls, RustlsCrypto, Tls, TlsProvider},
};
use serde::{Deserialize, Serialize};
use url::Url;

/// Global configuration.
///
/// Represents the whole TOML user's configuration file.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub downloads_dir: Option<PathBuf>,
    pub table_preset: Option<String>,
    pub table_arrangement: Option<TableArrangementConfig>,
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    #[serde(default)]
    pub message: MessageConfig,
    pub accounts: HashMap<String, AccountConfig>,
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    fn project_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn take_named_account(&mut self, name: &str) -> Option<(String, Self::Account)> {
        self.accounts.remove_entry(name)
    }

    fn take_default_account(&mut self) -> Option<(String, Self::Account)> {
        let name = self
            .accounts
            .iter()
            .find_map(|(name, account)| account.default.then(|| name.clone()))?;

        self.take_named_account(&name)
    }
}

impl Config {
    /// Serializes `self` to TOML and writes it to `path`, creating
    /// any missing parent directories. Used by the wizard to persist
    /// a freshly-built configuration.
    pub fn write(&self, path: &Path) -> Result<()> {
        let toml = toml::to_string_pretty(self).context("Serialize TOML config error")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Create TOML config parent `{}` error", parent.display())
            })?;
        }

        fs::write(path, toml)
            .with_context(|| format!("Write TOML config `{}` error", path.display()))?;

        Ok(())
    }
}

/// Account configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountConfig {
    #[serde(default)]
    pub default: bool,

    pub downloads_dir: Option<PathBuf>,
    pub table_preset: Option<String>,
    pub table_arrangement: Option<TableArrangementConfig>,

    #[serde(default)]
    pub envelope: EnvelopeConfig,

    #[allow(unused)]
    pub imap: Option<ImapConfig>,
    #[allow(unused)]
    pub jmap: Option<JmapConfig>,
    #[allow(unused)]
    pub maildir: Option<MaildirConfig>,
    #[allow(unused)]
    pub smtp: Option<SmtpConfig>,
}

/// Envelope-level rendering options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeConfig {
    #[serde(default)]
    pub list: EnvelopeListConfig,
}

/// `envelopes list` rendering options. Mirrors the pre-v2
/// `envelope.list.*` keys.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeListConfig {
    /// chrono `strftime` format used to render the DATE column.
    /// Defaults to `"%F %R%:z"` (e.g. `2026-05-06 14:30+02:00`) when
    /// neither the global nor the account config sets it.
    pub datetime_fmt: Option<String>,

    /// When `true`, the `Date:` header timezone offset is converted
    /// to the system's local timezone before formatting. Defaults to
    /// `false`, which preserves the wire offset.
    pub datetime_local_tz: Option<bool>,
}

/// Message-level configuration: user-defined composers and readers.
///
/// Composers produce a MIME draft on stdout (called by `compose-with`,
/// `reply-with`, `forward-with`). Readers consume a MIME message from
/// stdin and emit human-readable bytes on stdout (called by
/// `read-with`). Both are looked up by name; the entry flagged
/// `default = true` is used when no name is passed.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MessageConfig {
    #[serde(default)]
    pub composer: HashMap<String, ComposerConfig>,
    #[serde(default)]
    pub reader: HashMap<String, ReaderConfig>,
}

/// Single composer entry under `[message.composer.<name>]`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ComposerConfig {
    /// Shell command line invoked via `sh -c`. Stdin carries the
    /// source MIME bytes (empty for new messages); stdout is
    /// captured as the MIME draft; stderr is inherited so the
    /// composer can prompt the user.
    pub command: String,

    /// Marks this entry as the fallback when `compose-with` /
    /// `reply-with` / `forward-with` are invoked without a name.
    /// Exactly one composer should set this; if several do, the
    /// first one returned by the config lookup wins.
    #[serde(default)]
    pub default: bool,
}

/// Single reader entry under `[message.reader.<name>]`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ReaderConfig {
    /// Shell command line invoked via `sh -c`. Stdin carries the
    /// source MIME bytes; stdout is forwarded to the terminal (zero
    /// bytes is fine — the reader may have spawned its own UI);
    /// stderr is inherited.
    pub command: String,

    /// Marks this entry as the fallback when `read-with` is
    /// invoked without a name.
    #[serde(default)]
    pub default: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TableArrangementConfig {
    #[default]
    Dynamic,
    DynamicFullWidth,
    Disabled,
}

impl From<TableArrangementConfig> for ContentArrangement {
    fn from(arrangement: TableArrangementConfig) -> Self {
        match arrangement {
            TableArrangementConfig::Dynamic => ContentArrangement::Dynamic,
            TableArrangementConfig::DynamicFullWidth => ContentArrangement::DynamicFullWidth,
            TableArrangementConfig::Disabled => ContentArrangement::Disabled,
        }
    }
}

/// IMAP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapConfig {
    pub url: Url,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub starttls: bool,
    #[serde(default)]
    pub sasl: SaslConfig,
}

/// Maildir configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirConfig {
    pub root: PathBuf,
}

/// SMTP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SmtpConfig {
    pub url: Url,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub starttls: bool,
    #[serde(default)]
    pub sasl: SaslConfig,
}

/// SSL/TLS configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    pub provider: Option<TlsProviderConfig>,
    #[serde(default)]
    pub rustls: RustlsConfig,
    pub cert: Option<PathBuf>,
}

/// SSL/TLS provider configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    Rustls,
    NativeTls,
}

/// Rustls configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    pub crypto: Option<RustlsCryptoConfig>,
}

/// Rustls crypto provider configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RustlsCryptoConfig {
    Aws,
    Ring,
}

impl TryFrom<TlsConfig> for Tls {
    type Error = SecretError;

    fn try_from(config: TlsConfig) -> Result<Self, Self::Error> {
        Ok(Tls {
            provider: config.provider.map(|config| match config {
                TlsProviderConfig::Rustls => TlsProvider::Rustls,
                TlsProviderConfig::NativeTls => TlsProvider::NativeTls,
            }),
            rustls: Rustls {
                crypto: config.rustls.crypto.map(|config| match config {
                    RustlsCryptoConfig::Aws => RustlsCrypto::Aws,
                    RustlsCryptoConfig::Ring => RustlsCrypto::Ring,
                }),
            },
            cert: config.cert,
        })
    }
}

/// SASL configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslConfig {
    pub mechanism: Option<SaslMechanismConfig>,
    pub login: Option<SaslLoginConfig>,
    pub plain: Option<SaslPlainConfig>,
    pub anonymous: Option<SaslAnonymousConfig>,
}

/// SASL mechanism configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SaslMechanismConfig {
    Login,
    Plain,
    #[default]
    Anonymous,
}

/// SASL LOGIN configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub password: Secret,
}

/// SASL PLAIN configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    pub authzid: Option<String>,
    #[serde(deserialize_with = "shell_expanded_string")]
    pub authcid: String,
    pub passwd: Secret,
}

/// SASL ANONYMOUS configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    pub message: Option<String>,
}

impl TryFrom<SaslConfig> for Sasl {
    type Error = SecretError;

    fn try_from(config: SaslConfig) -> Result<Self, Self::Error> {
        Ok(Sasl {
            mechanism: config.mechanism.map(|m| match m {
                SaslMechanismConfig::Anonymous => SaslMechanism::Anonymous,
                SaslMechanismConfig::Plain => SaslMechanism::Plain,
                SaslMechanismConfig::Login => SaslMechanism::Login,
            }),
            anonymous: match config.anonymous {
                None => None,
                Some(config) => Some(SaslAnonymous {
                    message: config.message,
                }),
            },
            plain: match config.plain {
                None => None,
                Some(config) => Some(SaslPlain {
                    authzid: config.authzid,
                    authcid: config.authcid,
                    passwd: config.passwd.get()?,
                }),
            },
            login: match config.login {
                None => None,
                Some(config) => Some(SaslLogin {
                    username: config.username,
                    password: config.password.get()?,
                }),
            },
        })
    }
}

/// JMAP configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JmapConfig {
    /// The JMAP server address.
    ///
    /// Accepts either a bare authority (`fastmail.com`, `mail.example.com:8080`)
    /// for automatic discovery via `GET /.well-known/jmap`, or a full URL
    /// (`https://api.fastmail.com/jmap/api/`) to connect directly to the
    /// session endpoint. Supported schemes: `http`, `https`, `jmap` (→ http),
    /// `jmaps` (→ https).
    pub server: String,

    /// TLS configuration.
    #[serde(default)]
    pub tls: TlsConfig,

    /// Authentication configuration.
    pub auth: JmapAuthConfig,

    /// Identity id used by `messages send` to submit emails. Required
    /// only for JMAP send; can be discovered with `himalaya jmap
    /// identity get`.
    pub identity_id: Option<String>,

    /// Drafts mailbox id used by `messages send` to stage emails before
    /// submission. Required only for JMAP send; can be discovered with
    /// `himalaya jmap mailbox query --role drafts`.
    pub drafts_mailbox_id: Option<String>,
}

/// JMAP authentication configuration.
// https://www.iana.org/assignments/http-authschemes/http-authschemes.xhtml#authschemes
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum JmapAuthConfig {
    Header(Secret),
    /// Bearer token (OAuth 2.0 access token).
    Bearer {
        token: Secret,
    },
    /// HTTP Basic authentication (username + password).
    Basic {
        #[serde(deserialize_with = "shell_expanded_string")]
        username: String,
        password: Secret,
    },
}

#[cfg(feature = "jmap")]
impl TryFrom<JmapAuthConfig> for crate::jmap::session::JmapAuth {
    type Error = pimalaya_config::secret::SecretError;

    fn try_from(config: JmapAuthConfig) -> Result<Self, Self::Error> {
        match config {
            JmapAuthConfig::Header(token) => Ok(Self::Header(token.get()?)),
            JmapAuthConfig::Bearer { token } => Ok(Self::Bearer(token.get()?)),
            JmapAuthConfig::Basic { username, password } => Ok(Self::Basic {
                username,
                password: password.get()?,
            }),
        }
    }
}
