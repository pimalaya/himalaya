#[cfg(feature = "pgp-commands")]
use email::account::CmdsPgpConfig;
#[cfg(feature = "pgp-gpg")]
use email::account::GpgConfig;
#[cfg(feature = "pgp")]
use email::account::PgpConfig;
#[cfg(feature = "pgp-native")]
use email::account::{NativePgpConfig, NativePgpSecretKey, SignedSecretKey};
#[cfg(feature = "notmuch")]
use email::backend::NotmuchConfig;
#[cfg(feature = "imap")]
use email::imap::config::{ImapAuthConfig, ImapConfig};
#[cfg(feature = "smtp")]
use email::smtp::config::{SmtpAuthConfig, SmtpConfig};
use email::{
    account::config::{
        oauth2::{OAuth2Config, OAuth2Method, OAuth2Scopes},
        passwd::PasswdConfig,
    },
    email::config::{EmailHooks, EmailTextPlainFormat},
    folder::sync::FolderSyncStrategy,
    maildir::config::MaildirConfig,
    sendmail::config::SendmailConfig,
};
use keyring::Entry;
use process::{Cmd, Pipeline, SingleCmd};
use secret::Secret;
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{collections::HashSet, ops::Deref, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Entry", from = "String")]
pub struct EntryDef(#[serde(getter = "Deref::deref")] String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SingleCmd", from = "String")]
pub struct SingleCmdDef(#[serde(getter = "Deref::deref")] String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Pipeline", from = "Vec<String>")]
pub struct PipelineDef(
    #[serde(getter = "Deref::deref", serialize_with = "pipeline")] Vec<SingleCmd>,
);

// NOTE: did not find the way to do it with macros…
pub fn pipeline<S>(cmds: &Vec<SingleCmd>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = s.serialize_seq(Some(cmds.len()))?;
    for cmd in cmds {
        seq.serialize_element(&cmd.to_string())?;
    }
    seq.end()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Cmd", untagged)]
pub enum CmdDef {
    #[serde(with = "SingleCmdDef")]
    SingleCmd(SingleCmd),
    #[serde(with = "PipelineDef")]
    Pipeline(Pipeline),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Option<Cmd>", from = "OptionCmd", into = "OptionCmd")]
pub struct OptionCmdDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionCmd {
    #[serde(default, skip)]
    is_some: bool,
    #[serde(flatten, with = "CmdDef")]
    inner: Cmd,
}

impl From<OptionCmd> for Option<Cmd> {
    fn from(cmd: OptionCmd) -> Option<Cmd> {
        if cmd.is_some {
            Some(cmd.inner)
        } else {
            None
        }
    }
}

impl Into<OptionCmd> for Option<Cmd> {
    fn into(self) -> OptionCmd {
        match self {
            Some(cmd) => OptionCmd {
                is_some: true,
                inner: cmd,
            },
            None => OptionCmd {
                is_some: false,
                inner: Default::default(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Secret", rename_all = "kebab-case")]
pub enum SecretDef {
    Raw(String),
    #[serde(with = "CmdDef")]
    Cmd(Cmd),
    #[serde(with = "EntryDef", rename = "keyring")]
    KeyringEntry(Entry),
    #[default]
    Undefined,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Method")]
pub enum OAuth2MethodDef {
    #[serde(rename = "xoauth2", alias = "XOAUTH2")]
    XOAuth2,
    #[serde(rename = "oauthbearer", alias = "OAUTHBEARER")]
    OAuthBearer,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<ImapConfig>",
    from = "OptionImapConfig",
    into = "OptionImapConfig"
)]
pub struct OptionImapConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionImapConfig {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "ImapConfigDef")]
    inner: ImapConfig,
}

impl From<OptionImapConfig> for Option<ImapConfig> {
    fn from(config: OptionImapConfig) -> Option<ImapConfig> {
        if config.is_none {
            None
        } else {
            Some(config.inner)
        }
    }
}

impl Into<OptionImapConfig> for Option<ImapConfig> {
    fn into(self) -> OptionImapConfig {
        match self {
            Some(config) => OptionImapConfig {
                is_none: false,
                inner: config,
            },
            None => OptionImapConfig {
                is_none: true,
                inner: Default::default(),
            },
        }
    }
}

#[cfg(feature = "imap")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "ImapConfig", rename_all = "kebab-case")]
pub struct ImapConfigDef {
    pub host: String,
    pub port: u16,
    pub ssl: Option<bool>,
    pub starttls: Option<bool>,
    pub insecure: Option<bool>,
    pub login: String,
    #[serde(flatten, with = "ImapAuthConfigDef")]
    pub auth: ImapAuthConfig,
    pub notify_cmd: Option<String>,
    pub notify_query: Option<String>,
    pub watch_cmds: Option<Vec<String>>,
}

#[cfg(feature = "imap")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "ImapAuthConfig", tag = "auth")]
pub enum ImapAuthConfigDef {
    #[serde(rename = "passwd", alias = "password", with = "ImapPasswdConfigDef")]
    Passwd(#[serde(default)] PasswdConfig),
    #[serde(rename = "oauth2", with = "ImapOAuth2ConfigDef")]
    OAuth2(OAuth2Config),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "PasswdConfig")]
pub struct ImapPasswdConfigDef {
    #[serde(
        rename = "passwd",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub passwd: Secret,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Config")]
pub struct ImapOAuth2ConfigDef {
    #[serde(rename = "imap-oauth2-method", with = "OAuth2MethodDef", default)]
    pub method: OAuth2Method,
    #[serde(rename = "imap-oauth2-client-id")]
    pub client_id: String,
    #[serde(
        rename = "imap-oauth2-client-secret",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub client_secret: Secret,
    #[serde(rename = "imap-oauth2-auth-url")]
    pub auth_url: String,
    #[serde(rename = "imap-oauth2-token-url")]
    pub token_url: String,
    #[serde(
        rename = "imap-oauth2-access-token",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub access_token: Secret,
    #[serde(
        rename = "imap-oauth2-refresh-token",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub refresh_token: Secret,
    #[serde(flatten, with = "ImapOAuth2ScopesDef")]
    pub scopes: OAuth2Scopes,
    #[serde(rename = "imap-oauth2-pkce", default)]
    pub pkce: bool,
    #[serde(
        rename = "imap-oauth2-redirect-host",
        default = "OAuth2Config::default_redirect_host"
    )]
    pub redirect_host: String,
    #[serde(
        rename = "imap-oauth2-redirect-port",
        default = "OAuth2Config::default_redirect_port"
    )]
    pub redirect_port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Scopes")]
pub enum ImapOAuth2ScopesDef {
    #[serde(rename = "imap-oauth2-scope")]
    Scope(String),
    #[serde(rename = "imap-oauth2-scopes")]
    Scopes(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<MaildirConfig>",
    from = "OptionMaildirConfig",
    into = "OptionMaildirConfig"
)]
pub struct OptionMaildirConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionMaildirConfig {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "MaildirConfigDef")]
    inner: MaildirConfig,
}

impl From<OptionMaildirConfig> for Option<MaildirConfig> {
    fn from(config: OptionMaildirConfig) -> Option<MaildirConfig> {
        if config.is_none {
            None
        } else {
            Some(config.inner)
        }
    }
}

impl Into<OptionMaildirConfig> for Option<MaildirConfig> {
    fn into(self) -> OptionMaildirConfig {
        match self {
            Some(config) => OptionMaildirConfig {
                is_none: false,
                inner: config,
            },
            None => OptionMaildirConfig {
                is_none: true,
                inner: Default::default(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "MaildirConfig", rename_all = "kebab-case")]
pub struct MaildirConfigDef {
    #[serde(rename = "maildir-root-dir")]
    pub root_dir: PathBuf,
}

#[cfg(feature = "notmuch")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<NotmuchConfig>",
    from = "OptionNotmuchConfig",
    into = "OptionNotmuchConfig"
)]
pub struct OptionNotmuchConfigDef;

#[cfg(feature = "notmuch")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionNotmuchConfig {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "NotmuchConfigDef")]
    inner: NotmuchConfig,
}

#[cfg(feature = "notmuch")]
impl From<OptionNotmuchConfig> for Option<NotmuchConfig> {
    fn from(config: OptionNotmuchConfig) -> Option<NotmuchConfig> {
        if config.is_none {
            None
        } else {
            Some(config.inner)
        }
    }
}

#[cfg(feature = "notmuch")]
impl Into<OptionNotmuchConfig> for Option<NotmuchConfig> {
    fn into(self) -> OptionNotmuchConfig {
        match self {
            Some(config) => OptionNotmuchConfig {
                is_none: false,
                inner: config,
            },
            None => OptionNotmuchConfig {
                is_none: true,
                inner: Default::default(),
            },
        }
    }
}

#[cfg(feature = "notmuch")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "NotmuchConfig", rename_all = "kebab-case")]
pub struct NotmuchConfigDef {
    #[serde(rename = "notmuch-db-path")]
    pub db_path: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<EmailTextPlainFormat>",
    from = "OptionEmailTextPlainFormat",
    into = "OptionEmailTextPlainFormat"
)]
pub struct OptionEmailTextPlainFormatDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionEmailTextPlainFormat {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "EmailTextPlainFormatDef")]
    inner: EmailTextPlainFormat,
}

impl From<OptionEmailTextPlainFormat> for Option<EmailTextPlainFormat> {
    fn from(fmt: OptionEmailTextPlainFormat) -> Option<EmailTextPlainFormat> {
        if fmt.is_none {
            None
        } else {
            Some(fmt.inner)
        }
    }
}

impl Into<OptionEmailTextPlainFormat> for Option<EmailTextPlainFormat> {
    fn into(self) -> OptionEmailTextPlainFormat {
        match self {
            Some(config) => OptionEmailTextPlainFormat {
                is_none: false,
                inner: config,
            },
            None => OptionEmailTextPlainFormat {
                is_none: true,
                inner: Default::default(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "EmailTextPlainFormat",
    tag = "type",
    content = "width",
    rename_all = "kebab-case"
)]
pub enum EmailTextPlainFormatDef {
    #[default]
    Auto,
    Flowed,
    Fixed(usize),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<SmtpConfig>",
    from = "OptionSmtpConfig",
    into = "OptionSmtpConfig"
)]
pub struct OptionSmtpConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionSmtpConfig {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "SmtpConfigDef")]
    inner: SmtpConfig,
}

impl From<OptionSmtpConfig> for Option<SmtpConfig> {
    fn from(config: OptionSmtpConfig) -> Option<SmtpConfig> {
        if config.is_none {
            None
        } else {
            Some(config.inner)
        }
    }
}

impl Into<OptionSmtpConfig> for Option<SmtpConfig> {
    fn into(self) -> OptionSmtpConfig {
        match self {
            Some(config) => OptionSmtpConfig {
                is_none: false,
                inner: config,
            },
            None => OptionSmtpConfig {
                is_none: true,
                inner: Default::default(),
            },
        }
    }
}

#[cfg(feature = "smtp")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SmtpConfig")]
struct SmtpConfigDef {
    pub host: String,
    pub port: u16,
    pub ssl: Option<bool>,
    pub starttls: Option<bool>,
    pub insecure: Option<bool>,
    pub login: String,
    #[serde(flatten, with = "SmtpAuthConfigDef")]
    pub auth: SmtpAuthConfig,
}

#[cfg(feature = "smtp")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SmtpAuthConfig", tag = "auth")]
pub enum SmtpAuthConfigDef {
    #[serde(rename = "passwd", alias = "password", with = "SmtpPasswdConfigDef")]
    Passwd(#[serde(default)] PasswdConfig),
    #[serde(rename = "oauth2", with = "SmtpOAuth2ConfigDef")]
    OAuth2(OAuth2Config),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "PasswdConfig", default)]
pub struct SmtpPasswdConfigDef {
    #[serde(
        rename = "passwd",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub passwd: Secret,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Config", rename_all = "kebab-case")]
pub struct SmtpOAuth2ConfigDef {
    #[serde(with = "OAuth2MethodDef", default)]
    pub method: OAuth2Method,
    pub client_id: String,
    #[serde(
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub client_secret: Secret,
    pub auth_url: String,
    pub token_url: String,
    #[serde(
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub access_token: Secret,
    #[serde(
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub refresh_token: Secret,
    #[serde(flatten, with = "SmtpOAuth2ScopesDef")]
    pub scopes: OAuth2Scopes,
    #[serde(default)]
    pub pkce: bool,
    #[serde(default = "OAuth2Config::default_redirect_host")]
    pub redirect_host: String,
    #[serde(default = "OAuth2Config::default_redirect_port")]
    pub redirect_port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Scopes")]
pub enum SmtpOAuth2ScopesDef {
    #[serde(rename = "smtp-oauth2-scope")]
    Scope(String),
    #[serde(rename = "smtp-oauth2-scopes")]
    Scopes(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<SendmailConfig>",
    from = "OptionSendmailConfig",
    into = "OptionSendmailConfig"
)]
pub struct OptionSendmailConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionSendmailConfig {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "SendmailConfigDef")]
    inner: SendmailConfig,
}

impl From<OptionSendmailConfig> for Option<SendmailConfig> {
    fn from(config: OptionSendmailConfig) -> Option<SendmailConfig> {
        if config.is_none {
            None
        } else {
            Some(config.inner)
        }
    }
}

impl Into<OptionSendmailConfig> for Option<SendmailConfig> {
    fn into(self) -> OptionSendmailConfig {
        match self {
            Some(config) => OptionSendmailConfig {
                is_none: false,
                inner: config,
            },
            None => OptionSendmailConfig {
                is_none: true,
                inner: Default::default(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SendmailConfig", rename_all = "kebab-case")]
pub struct SendmailConfigDef {
    #[serde(with = "CmdDef", default = "sendmail_default_cmd")]
    cmd: Cmd,
}

fn sendmail_default_cmd() -> Cmd {
    Cmd::from("/usr/sbin/sendmail")
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<EmailHooks>",
    from = "OptionEmailHooks",
    into = "OptionEmailHooks"
)]
pub struct OptionEmailHooksDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionEmailHooks {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(
        flatten,
        skip_serializing_if = "EmailHooks::is_empty",
        with = "EmailHooksDef"
    )]
    inner: EmailHooks,
}

impl From<OptionEmailHooks> for Option<EmailHooks> {
    fn from(hooks: OptionEmailHooks) -> Option<EmailHooks> {
        if hooks.is_none {
            None
        } else {
            Some(hooks.inner)
        }
    }
}

impl Into<OptionEmailHooks> for Option<EmailHooks> {
    fn into(self) -> OptionEmailHooks {
        match self {
            Some(hooks) => OptionEmailHooks {
                is_none: false,
                inner: hooks,
            },
            None => OptionEmailHooks {
                is_none: true,
                inner: Default::default(),
            },
        }
    }
}

/// Represents the email hooks. Useful for doing extra email
/// processing before or after sending it.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "EmailHooks", rename_all = "kebab-case")]
pub struct EmailHooksDef {
    /// Represents the hook called just before sending an email.
    #[serde(default, with = "OptionCmdDef")]
    pub pre_send: Option<Cmd>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    remote = "Option<FolderSyncStrategy>",
    from = "OptionFolderSyncStrategy",
    into = "OptionFolderSyncStrategy"
)]
pub struct OptionFolderSyncStrategyDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionFolderSyncStrategy {
    #[serde(default, skip)]
    is_some: bool,
    #[serde(
        flatten,
        skip_serializing_if = "FolderSyncStrategy::is_default",
        with = "FolderSyncStrategyDef"
    )]
    inner: FolderSyncStrategy,
}

impl From<OptionFolderSyncStrategy> for Option<FolderSyncStrategy> {
    fn from(option: OptionFolderSyncStrategy) -> Option<FolderSyncStrategy> {
        if option.is_some {
            Some(option.inner)
        } else {
            None
        }
    }
}

impl Into<OptionFolderSyncStrategy> for Option<FolderSyncStrategy> {
    fn into(self) -> OptionFolderSyncStrategy {
        match self {
            Some(strategy) => OptionFolderSyncStrategy {
                is_some: true,
                inner: strategy,
            },
            None => OptionFolderSyncStrategy {
                is_some: false,
                inner: Default::default(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "FolderSyncStrategy", rename_all = "kebab-case")]
pub enum FolderSyncStrategyDef {
    #[default]
    All,
    #[serde(alias = "only")]
    Include(HashSet<String>),
    #[serde(alias = "except")]
    #[serde(alias = "ignore")]
    Exclude(HashSet<String>),
}

#[cfg(feature = "pgp")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Option<PgpConfig>", from = "OptionPgpConfig")]
pub struct OptionPgpConfigDef;

#[cfg(feature = "pgp")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionPgpConfig {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "PgpConfigDef")]
    inner: PgpConfig,
}

#[cfg(feature = "pgp")]
impl From<OptionPgpConfig> for Option<PgpConfig> {
    fn from(config: OptionPgpConfig) -> Option<PgpConfig> {
        if config.is_none {
            None
        } else {
            Some(config.inner)
        }
    }
}

#[cfg(feature = "pgp")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "PgpConfig", tag = "backend", rename_all = "kebab-case")]
pub enum PgpConfigDef {
    #[cfg(feature = "pgp-commands")]
    #[serde(with = "CmdsPgpConfigDef", alias = "commands")]
    Cmds(CmdsPgpConfig),
    #[cfg(feature = "pgp-gpg")]
    #[serde(with = "GpgConfigDef")]
    Gpg(GpgConfig),
    #[cfg(feature = "pgp-native")]
    #[serde(with = "NativePgpConfigDef")]
    Native(NativePgpConfig),
}

#[cfg(feature = "pgp-gpg")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "GpgConfig", rename_all = "kebab-case")]
pub struct GpgConfigDef;

#[cfg(feature = "pgp-commands")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "CmdsPgpConfig", rename_all = "kebab-case")]
pub struct CmdsPgpConfigDef {
    #[serde(default, with = "OptionCmdDef")]
    encrypt_cmd: Option<Cmd>,
    #[serde(default)]
    encrypt_recipient_fmt: Option<String>,
    #[serde(default)]
    encrypt_recipients_sep: Option<String>,
    #[serde(default, with = "OptionCmdDef")]
    decrypt_cmd: Option<Cmd>,
    #[serde(default, with = "OptionCmdDef")]
    sign_cmd: Option<Cmd>,
    #[serde(default, with = "OptionCmdDef")]
    verify_cmd: Option<Cmd>,
}

#[cfg(feature = "pgp-native")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "NativePgpConfig", rename_all = "kebab-case")]
pub struct NativePgpConfigDef {
    #[serde(default, with = "NativePgpSecretKeyDef")]
    secret_key: NativePgpSecretKey,
    #[serde(default, with = "SecretDef")]
    secret_key_passphrase: Secret,
    #[serde(default = "NativePgpConfig::default_wkd")]
    wkd: bool,
    #[serde(default = "NativePgpConfig::default_key_servers")]
    key_servers: Vec<String>,
}

#[cfg(feature = "pgp-native")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "NativePgpSecretKey", rename_all = "kebab-case")]
pub enum NativePgpSecretKeyDef {
    #[default]
    None,
    #[serde(skip)]
    Raw(SignedSecretKey),
    Path(PathBuf),
    #[serde(with = "EntryDef")]
    Keyring(Entry),
}
