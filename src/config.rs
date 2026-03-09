use std::{collections::HashMap, fmt, path::PathBuf, process::Command};

use anyhow::{bail, Result};
use pimalaya_toolbox::config::TomlConfig;
use secrecy::SecretString;
use serde::{de::Visitor, Deserialize, Deserializer};
use url::Url;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    #[serde(alias = "name")]
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub accounts: HashMap<String, AccountConfig>,
    // pub account: Option<AccountsConfig>,
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

/// The account configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountConfig {
    #[serde(default)]
    pub default: bool,
    pub imap: Option<ImapConfig>,
    pub smtp: Option<SmtpConfig>,
    #[serde(deserialize_with = "shell_expanded_string")]
    pub email: String,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    // pub backend: Option<Backend>,
    // #[cfg(feature = "pgp")]
    // pub pgp: Option<PgpConfig>,
    // #[cfg(not(feature = "pgp"))]
    // #[serde(default)]
    // #[serde(skip_serializing, deserialize_with = "missing_pgp_feature")]
    // pub pgp: Option<()>,

    // pub folder: Option<FolderConfig>,
    // pub envelope: Option<EnvelopeConfig>,
    // pub message: Option<MessageConfig>,
    // pub template: Option<TemplateConfig>,
}

/// The account configuration.
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

/// The account configuration.
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

    /// The IMAP extensions configuration.
    #[serde(default)]
    pub extensions: ImapExtensionsConfig,
}

/// The IMAP configuration dedicated to extensions.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapExtensionsConfig {
    #[serde(default)]
    id: ImapIdExtensionConfig,
}

/// The IMAP configuration dedicated to the ID extension.
///
/// https://www.rfc-editor.org/rfc/rfc2971.html
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdExtensionConfig {
    /// Automatically sends the ID command straight after
    /// authentication.
    #[serde(default)]
    always_after_auth: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    pub provider: Option<TlsProviderConfig>,
    #[serde(default)]
    pub rustls: RustlsConfig,
    pub cert: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    Rustls,
    NativeTls,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    pub crypto: Option<RustlsCryptoConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RustlsCryptoConfig {
    Aws,
    Ring,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslConfig {
    #[serde(default = "default_sasl_mechanisms")]
    pub mechanisms: Vec<SaslMechanismConfig>,
    pub login: Option<SaslLoginConfig>,
    pub plain: Option<SaslPlainConfig>,
    pub anonymous: Option<SaslAnonymousConfig>,
}

fn default_sasl_mechanisms() -> Vec<SaslMechanismConfig> {
    vec![SaslMechanismConfig::Plain, SaslMechanismConfig::Login]
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SaslMechanismConfig {
    Login,
    Plain,
    Anonymous,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecretConfig {
    Raw(SecretString),
    Command(Vec<String>),
}

impl SecretConfig {
    pub fn get(&self) -> Result<SecretString> {
        match self {
            Self::Raw(secret) => Ok(secret.clone()),
            Self::Command(args) => {
                let Some((program, args)) = args.split_first() else {
                    bail!("Secret command cannot be empty")
                };

                let mut cmd = Command::new(program);
                cmd.args(args);
                let out = cmd.output()?;

                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    bail!("Cannot read secret from command: {err}");
                }

                let secret = String::from_utf8_lossy(&out.stdout);
                let secret = secret.trim_matches(['\r', '\n']);
                let secret = match secret.split_once('\n') {
                    Some((secret, _)) => secret.trim_matches(['\r', '\n']),
                    None => secret,
                };

                Ok(SecretString::from(secret))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    pub username: String,
    pub password: SecretConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    pub authzid: Option<String>,
    pub authcid: String,
    pub passwd: SecretConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    pub message: Option<String>,
}

struct ShellExpandedStringVisitor;

impl<'de> Visitor<'de> for ShellExpandedStringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an string containing environment variable(s)")
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        match shellexpand::full(&v) {
            Ok(v) => Ok(v.to_string()),
            Err(_) => Ok(v),
        }
    }
}

pub fn shell_expanded_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    deserializer.deserialize_string(ShellExpandedStringVisitor)
}
