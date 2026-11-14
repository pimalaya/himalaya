//! # SASL configuration
//!
//! The `sasl` block of the IMAP, SMTP and ManageSieve backends.

use pimalaya_config::{secret::Secret, toml::shell_expanded_string};
use serde::Deserialize;

/// SASL configuration, exactly one mechanism per `[*.sasl]` block.
///
/// Each variant carries only what its mechanism transmits, and serde picks
/// it from the field name.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum SaslConfig {
    /// The ANONYMOUS mechanism.
    Anonymous(SaslAnonymousConfig),
    /// The LOGIN mechanism.
    Login(SaslLoginConfig),
    /// The PLAIN mechanism.
    Plain(SaslPlainConfig),
    /// The OAUTHBEARER mechanism.
    Oauthbearer(SaslOauthbearerConfig),
    /// The XOAUTH2 mechanism.
    Xoauth2(SaslXoauth2Config),
    /// The SCRAM-SHA-256 mechanism.
    #[serde(rename = "scram-sha-256")]
    ScramSha256(SaslScramSha256Config),
}

/// SASL ANONYMOUS configuration <sup>[rfc4505]</sup>.
///
/// [rfc4505]: https://www.iana.org/go/rfc4505
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    /// The optional trace message handed to the server.
    pub message: Option<String>,
}

/// SASL LOGIN configuration <sup>[draft]</sup>.
///
/// [draft]: https://datatracker.ietf.org/doc/html/draft-murchison-sasl-login-00
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    /// The account to authenticate as.
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    /// Its password.
    pub password: Secret,
}

/// SASL PLAIN configuration <sup>[rfc4616]</sup>.
///
/// [rfc4616]: https://www.iana.org/go/rfc4616
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    /// The identity to act as, when it differs from the authenticated one.
    pub authzid: Option<String>,
    /// The account to authenticate as.
    #[serde(deserialize_with = "shell_expanded_string")]
    #[serde(alias = "username")]
    pub authcid: String,
    /// Its password.
    #[serde(alias = "password")]
    pub passwd: Secret,
}

/// SASL OAUTHBEARER configuration <sup>[rfc7628]</sup>.
///
/// The host and port echoed in the GS2 header come from the live server
/// URL at connect time, so neither is configured here.
///
/// [rfc7628]: https://www.iana.org/go/rfc7628
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslOauthbearerConfig {
    /// The account to authenticate as.
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    /// Its OAuth 2.0 access token.
    pub token: Secret,
}

/// SASL XOAUTH2 configuration, [Google's pre-standard scheme][xoauth2].
///
/// [xoauth2]: https://developers.google.com/gmail/imap/xoauth2-protocol
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslXoauth2Config {
    /// The account to authenticate as.
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    /// Its OAuth 2.0 access token.
    pub token: Secret,
}

/// SASL SCRAM-SHA-256 configuration <sup>[rfc7677]</sup>.
///
/// [rfc7677]: https://www.iana.org/go/rfc7677
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslScramSha256Config {
    /// The account to authenticate as.
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    /// Its password.
    pub password: Secret,
}
