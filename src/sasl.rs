use pimalaya_toolbox::secret::Secret;
use serde::{Deserialize, Serialize};

/// The account configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    pub username: String,
    pub password: Secret,
}

/// The account configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    pub authzid: Option<String>,
    pub authcid: String,
    pub passwd: Secret,
}

/// The account configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    pub message: Option<String>,
}

/// The account configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslConfig {
    #[serde(default = "default_sasl_mechanisms")]
    pub mechanisms: Vec<SaslMechanism>,
    pub anonymous: Option<SaslAnonymousConfig>,
    pub login: Option<SaslLoginConfig>,
    pub plain: Option<SaslPlainConfig>,
}

impl Default for SaslConfig {
    fn default() -> Self {
        Self {
            mechanisms: vec![SaslMechanism::Anonymous],
            anonymous: None,
            login: None,
            plain: None,
        }
    }
}

/// The account configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum SaslMechanism {
    Anonymous,
    Login,
    Plain,
}

fn default_sasl_mechanisms() -> Vec<SaslMechanism> {
    vec![SaslMechanism::Plain, SaslMechanism::Login]
}
