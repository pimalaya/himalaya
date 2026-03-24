use std::{collections::HashMap, path::PathBuf};

use comfy_table::ContentArrangement;
use pimalaya_toolbox::{
    config::{shell_expanded_string, TomlConfig},
    sasl::{sasl_default_mechanisms, Sasl, SaslAnonymous, SaslLogin, SaslMechanism, SaslPlain},
    secret::{Secret, SecretError},
    stream::{Rustls, RustlsCrypto, Tls, TlsProvider},
};
use serde::Deserialize;
use url::Url;

/// Global configuration.
///
/// Represents the whole TOML user's configuration file.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub downloads_dir: Option<PathBuf>,
    pub table_preset: Option<String>,
    pub table_arrangement: Option<TableArrangementConfig>,
    pub accounts: HashMap<String, AccountConfig>,
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    fn project_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn find_default_account(&self) -> Option<(String, Self::Account)> {
        self.accounts
            .iter()
            .find(|(_, account)| account.default)
            .map(|(name, account)| (name.to_owned(), account.clone()))
    }

    fn find_account(&self, name: &str) -> Option<(String, Self::Account)> {
        self.accounts
            .get(name)
            .map(|account| (name.to_owned(), account.clone()))
    }
}

/// Account configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountConfig {
    #[serde(default)]
    pub default: bool,

    pub downloads_dir: Option<PathBuf>,
    pub table_preset: Option<String>,
    pub table_arrangement: Option<TableArrangementConfig>,

    #[allow(unused)]
    pub imap: Option<ImapConfig>,
    pub jmap: Option<JmapConfig>,
    #[allow(unused)]
    pub maildir: Option<MaildirConfig>,
    #[allow(unused)]
    pub smtp: Option<SmtpConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirConfig {
    pub root: PathBuf,
}

/// SMTP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    pub provider: Option<TlsProviderConfig>,
    #[serde(default)]
    pub rustls: RustlsConfig,
    pub cert: Option<PathBuf>,
}

/// SSL/TLS provider configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    Rustls,
    NativeTls,
}

/// Rustls configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    pub crypto: Option<RustlsCryptoConfig>,
}

/// Rustls crypto provider configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RustlsCryptoConfig {
    Aws,
    Ring,
}

impl TryFrom<TlsConfig> for Tls {
    type Error = SecretError;

    fn try_from(config: TlsConfig) -> Result<Self, Self::Error> {
        Ok(Tls {
            provider: match config.provider {
                None => None,
                Some(config) => Some(match config {
                    TlsProviderConfig::Rustls => TlsProvider::Rustls,
                    TlsProviderConfig::NativeTls => TlsProvider::NativeTls,
                }),
            },
            rustls: Rustls {
                crypto: match config.rustls.crypto {
                    None => None,
                    Some(config) => Some(match config {
                        RustlsCryptoConfig::Aws => RustlsCrypto::Aws,
                        RustlsCryptoConfig::Ring => RustlsCrypto::Ring,
                    }),
                },
            },
            cert: config.cert,
        })
    }
}

/// SASL configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslConfig {
    pub mechanisms: Option<Vec<SaslMechanismConfig>>,
    pub login: Option<SaslLoginConfig>,
    pub plain: Option<SaslPlainConfig>,
    pub anonymous: Option<SaslAnonymousConfig>,
}

/// SASL mechanism configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SaslMechanismConfig {
    Login,
    Plain,
    Anonymous,
}

/// SASL LOGIN configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub password: Secret,
}

/// SASL PLAIN configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    pub authzid: Option<String>,
    #[serde(deserialize_with = "shell_expanded_string")]
    pub authcid: String,
    pub passwd: Secret,
}

/// SASL ANONYMOUS configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    pub message: Option<String>,
}

impl TryFrom<SaslConfig> for Sasl {
    type Error = SecretError;

    fn try_from(config: SaslConfig) -> Result<Self, Self::Error> {
        Ok(Sasl {
            mechanisms: match config.mechanisms {
                None => sasl_default_mechanisms(),
                Some(config) => config
                    .into_iter()
                    .map(|m| match m {
                        SaslMechanismConfig::Anonymous => SaslMechanism::Anonymous,
                        SaslMechanismConfig::Plain => SaslMechanism::Plain,
                        SaslMechanismConfig::Login => SaslMechanism::Login,
                    })
                    .collect(),
            },
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
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JmapConfig {
    /// The HTTPS base URL of the JMAP server.
    ///
    /// Must use the `https://` or `jmap://` scheme. Session discovery
    /// (`GET /.well-known/jmap`) is performed automatically on connection.
    pub url: Url,

    /// TLS configuration.
    #[serde(default)]
    pub tls: TlsConfig,

    /// Authentication configuration.
    pub auth: JmapAuthConfig,
}

/// JMAP authentication configuration.
// https://www.iana.org/assignments/http-authschemes/http-authschemes.xhtml#authschemes
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum JmapAuthConfig {
    /// Bearer token (OAuth 2.0 access token).
    Bearer { token: Secret },
    /// HTTP Basic authentication (username + password).
    Basic {
        #[serde(deserialize_with = "shell_expanded_string")]
        username: String,
        password: Secret,
    },
}

#[cfg(feature = "jmap")]
impl TryFrom<JmapAuthConfig> for pimalaya_toolbox::stream::jmap::JmapAuth {
    type Error = pimalaya_toolbox::secret::SecretError;

    fn try_from(config: JmapAuthConfig) -> Result<Self, Self::Error> {
        match config {
            JmapAuthConfig::Bearer { token } => Ok(Self::Bearer(token.get()?)),
            JmapAuthConfig::Basic { username, password } => Ok(Self::Basic {
                username,
                password: password.get()?,
            }),
        }
    }
}
