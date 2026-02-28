// pub mod arg;
// pub mod command;
// pub mod config;

use std::{fmt, path::PathBuf};

use serde::{de::Visitor, Deserialize, Deserializer, Serialize};

use crate::{sasl::SaslConfig, tls::TlsConfig};

/// The account configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapConfig {
    /// The IMAP server host name.
    pub host: String,
    /// The IMAP server host port.
    pub port: Option<u16>,

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
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapExtensionsConfig {
    #[serde(default)]
    id: ImapIdExtensionConfig,
}

/// The IMAP configuration dedicated to the ID extension.
///
/// https://www.rfc-editor.org/rfc/rfc2971.html
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdExtensionConfig {
    /// Automatically sends the ID command straight after
    /// authentication.
    #[serde(default)]
    send_after_auth: bool,
}

/// The account configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Account {
    #[serde(default)]
    pub default: bool,

    pub imap: Option<ImapConfig>,

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
