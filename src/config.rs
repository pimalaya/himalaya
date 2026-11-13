//! # Configuration
//!
//! The TOML schema: a global block plus named account blocks, each
//! carrying the optional per-backend sub-blocks its protocols need.
//!
//! Backend defaults are duplicated here rather than read from the io-*
//! crates, so the schema compiles under any feature subset, none included.
//!
//! Every build parses the whole schema, the blocks of the backends it
//! leaves out included. A type only some backends read is therefore
//! allowed to go unread in the builds without them
//! (`cfg_attr(not(..), allow(dead_code))`) rather than gated, which would
//! change what a build accepts.

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Error, anyhow};
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "sieve"
))]
use anyhow::{Result, bail};
use clap::ValueEnum;
#[cfg(any(feature = "imap", feature = "smtp", feature = "sieve"))]
use io_sasl::{
    login::SaslLoginCreds, mechanism::Sasl, rfc4505::anonymous::SaslAnonymousCreds,
    rfc4616::plain::SaslPlainCreds, rfc5801::SaslGs2ChannelBinding, rfc5802::SaslScramCreds,
    rfc7628::oauthbearer::SaslOauthbearerCreds, xoauth2::SaslXoauth2Creds,
};
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "sieve"
))]
use pimalaya_config::secret::SecretResolver;
use pimalaya_config::{
    secret::Secret,
    toml::{TomlConfig, opt_shell_expanded_path, shell_expanded_path, shell_expanded_string},
};
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "sieve"
))]
use pimalaya_stream::{
    proxy::{Proxy, ProxyAuth},
    tls::{Rustls, RustlsCrypto, Tls, TlsProvider},
};
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "sieve"
))]
use secrecy::SecretString;
use serde::{Deserialize, de::IgnoredAny};
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "sieve"
))]
use url::Url;

use crate::{backend::Backend, capability};

// NOTE: these mirror the io-* crates' own `default_alpn()`, kept local so
// the schema depends on no backend crate.
pub(crate) fn default_imap_alpn() -> Vec<String> {
    vec![String::from("imap")]
}

pub(crate) fn default_smtp_alpn() -> Vec<String> {
    vec![String::from("smtp")]
}

pub(crate) fn default_sieve_alpn() -> Vec<String> {
    Vec::new()
}

pub(crate) fn default_jmap_alpn() -> Vec<String> {
    vec![String::from("http/1.1")]
}

/// The whole TOML configuration file.
///
/// `deny_unknown_fields` is omitted so one file can be shared with
/// himalaya-tui, whose own top-level fields are ignored here.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Fallback for [`AccountConfig::display_name`].
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,
    /// Fallback for [`AccountConfig::signature`].
    pub signature: Option<String>,
    /// Fallback for [`AccountConfig::signature_delim`].
    pub signature_delim: Option<String>,
    /// Directory attachments are downloaded to.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub downloads_dir: Option<PathBuf>,
    /// `envelope list` rendering options.
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    /// Mailbox aliases.
    #[serde(default)]
    pub mailbox: MailboxConfig,
    /// Fallback for [`AccountConfig::message`].
    #[serde(default)]
    pub message: MessageConfig,
    /// The named `[accounts.<name>]` blocks.
    pub accounts: HashMap<String, AccountConfig>,
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    fn project_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn sample_url() -> &'static str {
        "https://github.com/pimalaya/himalaya/blob/master/config.sample.toml"
    }

    fn accounts(&self) -> impl Iterator<Item = (&str, bool)> {
        let accounts = self.accounts.iter();
        accounts.map(|(name, account)| (name.as_str(), account.default))
    }

    fn take_named_account(&mut self, name: &str) -> Option<(String, Self::Account)> {
        let (name, mut account) = self.accounts.remove_entry(name)?;
        account.inherit_proxy();
        Some((name, account))
    }
}

impl AccountConfig {
    /// Hands the account proxy to every network backend naming none of
    /// its own.
    fn inherit_proxy(&mut self) {
        let Some(proxy) = &self.proxy else {
            return;
        };

        let slots = [
            self.imap.as_mut().map(|c| &mut c.proxy),
            self.jmap.as_mut().map(|c| &mut c.proxy),
            self.gmail.as_mut().map(|c| &mut c.proxy),
            self.msgraph.as_mut().map(|c| &mut c.proxy),
            self.smtp.as_mut().map(|c| &mut c.proxy),
            self.sieve.as_mut().map(|c| &mut c.proxy),
        ];

        for slot in slots.into_iter().flatten() {
            slot.get_or_insert_with(|| proxy.clone());
        }
    }

    /// The error of account `name` where nothing matches `backend`, the
    /// caller being able to use `supported` alone.
    ///
    /// Names the v1 `backend` table or the removed `m2dir` block when
    /// present, it being the likely cause. A `--backend` naming a block the account lacks is refused
    /// under `capability-missing`, as the resolver refuses a verb the
    /// filter leaves unserved.
    pub fn no_backend_error(&self, name: &str, backend: Backend, supported: &[Backend]) -> Error {
        let (reason, missing) = match backend {
            Backend::Auto => {
                let supported: Vec<String> = supported.iter().map(|b| format!("`{b}`")).collect();
                let supported = supported.join(", ");
                let reason =
                    format!("Account `{name}` configures no supported backend ({supported})");
                (reason, false)
            }
            backend if !supported.contains(&backend) => {
                let reason =
                    format!("Backend `{backend}` is not supported by this command or build");
                (reason, false)
            }
            backend => (format!("Account `{name}` has no `{backend}` block"), true),
        };

        let reason = if self.backend.is_some() {
            format!("{reason}: its v1 `backend` table is ignored since v2, see MIGRATION.md")
        } else if self.m2dir.is_some() {
            format!(
                "{reason}: its `m2dir` block is ignored since v3 removed m2dir, see MIGRATION.md"
            )
        } else {
            reason
        };

        if missing {
            capability::missing(reason)
        } else {
            anyhow!(reason)
        }
    }
}

/// One `[accounts.<name>]` block.
///
/// `deny_unknown_fields` is omitted so a block written for one binary
/// loads in the other: the CLI-only sub-blocks have to be tolerated by
/// himalaya-tui, which models the rest.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountConfig {
    /// Whether a command with no `-a/--account` runs against this one.
    #[serde(default)]
    pub default: bool,
    /// Address this account sends as, absent `--from`.
    ///
    /// Neither validated nor required, an account that never composes
    /// having no use for it. Aliased to `from`, the spelling himalaya-tui
    /// writes, the two binaries sharing one file.
    #[serde(alias = "from")]
    pub email: Option<String>,
    /// Name the `From` address carries, falling back to the global one.
    ///
    /// Quoting and encoding are the MIME builder's business, so write the
    /// name as it should read. Aliased to `from-name`, the spelling
    /// himalaya-tui writes.
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,
    /// Signature appended to a composed message, falling back to the
    /// global one.
    ///
    /// The value is the signature alone, the separator before it being
    /// `signature-delim`'s business, so one written for either binary
    /// reads the same in the other.
    pub signature: Option<String>,
    /// Separator written before the signature, defaulting to the RFC 3676
    /// section 4.3 `"-- \n"`.
    ///
    /// Written verbatim, so a value meant to stand on its own line carries
    /// its own trailing newline.
    pub signature_delim: Option<String>,
    /// Directory attachments are downloaded to.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub downloads_dir: Option<PathBuf>,
    /// `envelope list` rendering options.
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    /// Mailbox aliases.
    #[serde(default)]
    pub mailbox: MailboxConfig,
    /// Message sending options.
    #[serde(default)]
    pub message: MessageConfig,
    /// Proxy every network backend of this account goes through, unless
    /// its own block names one.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// The IMAP backend of this account.
    #[allow(unused)]
    pub imap: Option<ImapConfig>,
    /// The JMAP backend of this account.
    #[allow(unused)]
    pub jmap: Option<JmapConfig>,
    /// The Gmail backend of this account.
    #[allow(unused)]
    pub gmail: Option<GmailConfig>,
    /// The Microsoft Graph backend of this account.
    #[allow(unused)]
    pub msgraph: Option<MsgraphConfig>,
    /// The Maildir backend of this account.
    #[allow(unused)]
    pub maildir: Option<MaildirConfig>,
    /// The mbox backend of this account.
    #[allow(unused)]
    pub mbox: Option<MboxConfig>,
    /// The pimdir backend of this account.
    #[allow(unused)]
    pub pimdir: Option<PimdirConfig>,
    /// The SMTP transport of this account.
    #[allow(unused)]
    pub smtp: Option<SmtpConfig>,
    /// The ManageSieve endpoint of this account.
    #[allow(unused)]
    pub sieve: Option<SieveConfig>,
    /// The v1 `backend` table, kept only to point at MIGRATION.md.
    #[serde(default)]
    pub backend: Option<IgnoredAny>,
    /// The `m2dir` block, whose backend was removed, kept only to point
    /// at MIGRATION.md.
    #[serde(default)]
    pub m2dir: Option<IgnoredAny>,
}

/// Envelope-level rendering options.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeConfig {
    /// `envelope list` rendering options.
    #[serde(default)]
    pub list: EnvelopeListConfig,
}

/// Mailbox aliases.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxConfig {
    /// Friendly names mapped to backend-native mailbox ids, resolved
    /// case-insensitively.
    ///
    /// An alias named after a mailbox role overrides the role the backend
    /// reports, the `inbox` one being the default mailbox of a unified
    /// command omitting `-m/--mailbox`.
    #[serde(default, rename = "alias", alias = "aliases")]
    pub aliases: HashMap<String, String>,
}

/// Message options under `message.*`.
///
/// Unknown keys are accepted rather than denied, so the rest of a v1
/// `[message]` table (`read`, `write`, `delete`...) keeps loading.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageConfig {
    /// Sending options.
    #[serde(default)]
    pub send: MessageSendConfig,
    /// `message reply` options.
    #[serde(default)]
    pub reply: MessageQuoteConfig,
    /// `message forward` options.
    #[serde(default)]
    pub forward: MessageQuoteConfig,
}

/// Quoting options under `message.reply.*` and `message.forward.*`.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageQuoteConfig {
    /// Posting style when `--posting-style` is not passed.
    pub posting_style: Option<PostingStyle>,
}

/// Where a quoted source body sits relative to the written one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[clap(rename_all = "kebab-case")]
pub enum PostingStyle {
    /// The written body above the quoted source body.
    #[default]
    Top,
    /// The quoted source body above the written body.
    Bottom,
    /// The written body alone, the source left unquoted, for a writer
    /// who already laid the quote out.
    None,
}

/// Sending options under `message.send.*`.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageSendConfig {
    /// Mailbox a sent message is copied to when `--save` is not passed.
    ///
    /// Leave it unset on Gmail and Microsoft Graph, which file the sent
    /// message themselves.
    pub save_copy: Option<SaveCopyConfig>,
}

/// The `message.send.save-copy` value.
#[cfg_attr(not(any(backend, feature = "smtp")), allow(dead_code))]
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SaveCopyConfig {
    /// `true` for the `sent` mailbox, `false` for no copy, the v1 form.
    Enabled(bool),
    /// A mailbox name, alias or role.
    Mailbox(String),
}

/// `envelope list` rendering options under `envelope.list.*`.
#[cfg_attr(
    not(any(backend, feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeListConfig {
    /// chrono `strftime` format of an envelope date, defaulting to
    /// `"%F %R"`.
    pub datetime_fmt: Option<String>,
    /// Whether the `Date:` offset is converted to the local timezone
    /// before formatting, the default `true`; `false` keeps the offset
    /// the message was sent with.
    pub datetime_local_tz: Option<bool>,
    /// Default `-s/--page-size`, the flag winning when passed and 25
    /// being the hard fallback.
    pub page_size: Option<u32>,
    /// The v2 `table` block, whatever it holds, ignored rather than
    /// refused: its glyph and colour keys no longer apply, and a file
    /// shared with himalaya-tui keeps loading.
    #[serde(rename = "table")]
    _table: Option<IgnoredAny>,
}

/// Parses a backend `server` string into a [`Url`].
///
/// A full URL, a bare authority or a bare host, the two bare forms taking
/// the default scheme. Absence of `://` is what detects them, the parser
/// otherwise reading `mail.example.com:993` as a scheme.
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "sieve"
))]
pub fn parse_server(server: &str, default_scheme: &str, allowed: &[&str]) -> Result<Url> {
    let url = if server.contains("://") {
        Url::parse(server)?
    } else {
        Url::parse(&format!("{default_scheme}://{server}"))?
    };

    let scheme = url.scheme();

    if !allowed.contains(&scheme) {
        bail!("Invalid server scheme `{scheme}`: expected one of {allowed:?}");
    }

    Ok(url)
}

/// IMAP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapConfig {
    /// IMAP server address, a bare authority or a full URL.
    ///
    /// A bare authority takes `imaps://`, implicit TLS. A full URL is used
    /// verbatim, `imap://` being cleartext with an optional STARTTLS
    /// upgrade. Mirrors [`JmapConfig::server`].
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, valid only for an `imap://` server.
    #[serde(default)]
    pub starttls: bool,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// the RFC 7595 registered `["imap"]`.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_imap_alpn")]
    pub alpn: Vec<String>,
    /// SASL credentials, omitted to skip authentication entirely.
    ///
    /// No `AUTHENTICATE` command is then sent at all. Advertising the
    /// ANONYMOUS mechanism is `sasl.anonymous = {}` instead.
    pub sasl: Option<SaslConfig>,
    /// RFC 4959 SASL-IR quirk, unset following the advertised capability.
    ///
    /// `false` waits for the server's continuation request rather than
    /// inlining the credentials, which Coremail (126.com, 163.com) needs:
    /// it advertises SASL-IR falsely.
    #[serde(default)]
    pub sasl_ir: Option<bool>,
    /// RFC 2971 `ID` extension quirks.
    ///
    /// Some providers, notably mail.qq.com and fastmail, want an `ID`
    /// exchange straight after authentication: `id.auto = true` opts in.
    #[serde(default)]
    pub id: ImapIdConfig,
    /// RFC 5256 `SORT` extension options.
    #[serde(default)]
    pub sort: ImapSortConfig,
}

/// Per-account `imap.sort.*` options.
#[cfg_attr(not(feature = "imap"), allow(dead_code))]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapSortConfig {
    /// Forces the client-side sort fallback on or off.
    ///
    /// On, the client sorts with SEARCH and FETCH; off, it always issues a
    /// server `SORT`. Unset, the fallback runs when the server lacks the
    /// SORT capability, and on iCloud, whose SORT ignores its keys.
    pub fallback: Option<bool>,
}

/// Per-account `imap.id.*` quirks.
#[cfg_attr(not(feature = "imap"), allow(dead_code))]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdConfig {
    /// Whether the auth coroutine chains an `ID` round-trip after the
    /// tagged auth response, default `false` skipping it.
    #[serde(default)]
    pub auto: bool,
    /// Parameters sent with the auto-`ID` command.
    ///
    /// Unset sends himalaya's canned value for every well-known key
    /// (`name`, `version`, `vendor`, `support-url`), and an empty map
    /// sends `ID NIL`. `true` substitutes the canned value for a
    /// well-known key and `NIL` otherwise, `false` always sends `NIL`,
    /// and an absent key is not transmitted.
    #[serde(default)]
    pub fields: Option<HashMap<String, bool>>,
}

/// Header carrying custom keywords inline with a message body.
///
/// Mirrors io-maildir's `KeywordHeader`, kept local so the config
/// schema does not depend on any backend crate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MaildirKeywordHeaderConfig {
    /// `X-Keywords`, comma-separated (OfflineIMAP, mbsync).
    XKeywords,
    /// `X-Label`, space-separated (mutt, notmuch).
    XLabel,
}

/// Per-account `maildir.keywords.*` options, both unset reading none.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirKeywordsConfig {
    /// Whether keywords are resolved through each mailbox's own
    /// dovecot-keywords file, which maps a lowercase info-section letter
    /// to a keyword, default `false` leaving those letters unread.
    #[serde(default)]
    pub dovecot: bool,
    /// The body header keywords are read from, unset reading none.
    #[serde(default)]
    pub header: Option<MaildirKeywordHeaderConfig>,
}

/// Maildir configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirConfig {
    /// The Maildir root, one directory per mailbox below it.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub root: PathBuf,
    /// How custom, non-IANA keywords are read, if at all.
    #[serde(default)]
    pub keywords: MaildirKeywordsConfig,
}

/// mbox configuration: a directory of mbox files, plus the spool.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MboxConfig {
    /// The directory holding the mbox files, one file per mailbox,
    /// walked recursively.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub root: PathBuf,
    /// The spool surfaced as the `INBOX` mailbox, typically `$MAIL`,
    /// wherever it lives.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub inbox: Option<PathBuf>,
    /// The variant new messages are written in, and whose quoting reads
    /// undo.
    #[serde(default)]
    pub format: MboxFormatConfig,
    /// Whether the children of mailbox `a` live in `a.sbd`, as
    /// Thunderbird lays them out, rather than in `a`.
    #[serde(default)]
    pub thunderbird: bool,
    /// The locks every write takes.
    #[serde(default)]
    pub lock: MboxLockConfig,
}

/// One of the four mbox variants.
///
/// Mirrors io-mbox's `MboxFormat`, kept local so the config schema does
/// not depend on any backend crate.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MboxFormatConfig {
    /// `From ` lines quoted with one `>`, lossy.
    Mboxo,
    /// Reversible quoting of `>*From ` lines.
    #[default]
    Mboxrd,
    /// mboxo quoting plus a `Content-Length` header.
    Mboxcl,
    /// No quoting, a `Content-Length` header delimits the body.
    Mboxcl2,
}

/// Per-account `mbox.lock.*` options, both locks taken by default.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MboxLockConfig {
    /// Whether to take the `<mbox>.lock` dotlock, which a spool
    /// directory the user cannot write to refuses.
    #[serde(default = "default_true")]
    pub dotlock: bool,
    /// Whether to take the fcntl lock.
    #[serde(default = "default_true")]
    pub fcntl: bool,
    /// Seconds to wait for a lock another process holds, 10 when
    /// unset.
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl Default for MboxLockConfig {
    fn default() -> Self {
        Self {
            dotlock: true,
            fcntl: true,
            timeout: None,
        }
    }
}

fn default_true() -> bool {
    true
}

/// pimdir configuration, a local store read as an offline cache.
///
/// The store is a SQLite index beside content-addressed blobs, populated
/// by the Neverest sync engine.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PimdirConfig {
    /// The store directory, holding `pimdir.db` and `objects/`.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub root: PathBuf,
    /// The sync engine account whose collections this client reads,
    /// per pimdir STORAGE §9.2.
    ///
    /// Usually left unset, a store synced by one account being read as
    /// that one. Set it for a store several accounts share, where guessing
    /// would show the wrong mailbox set.
    #[serde(default)]
    pub account: Option<String>,
}

/// SMTP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SmtpConfig {
    /// SMTP server address, a bare authority or a full URL.
    ///
    /// A bare authority takes `smtps://`, implicit TLS. A full URL is used
    /// verbatim, `smtp://` being cleartext with an optional STARTTLS
    /// upgrade. Mirrors [`JmapConfig::server`].
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, valid only for an `smtp://` server.
    #[serde(default)]
    pub starttls: bool,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// the RFC 7595 registered `["smtp"]`.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_smtp_alpn")]
    pub alpn: Vec<String>,
    /// SASL credentials, see [`ImapConfig::sasl`].
    pub sasl: Option<SaslConfig>,
}

/// ManageSieve configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SieveConfig {
    /// ManageSieve server address, a bare authority or a full URL.
    ///
    /// RFC 5804 registers one port and reaches TLS on it through STARTTLS,
    /// so a bare authority takes `sieve://` unlike the IMAP and SMTP ones.
    /// `sieves://` is for the deployments the specification does not
    /// define, listening for a handshake straight away.
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, unset following the scheme.
    ///
    /// On for `sieve://`, the only upgrade path RFC 5804 defines, off for
    /// `sieves://` and `unix://`. Setting it on `sieves://` is an error,
    /// TLS being already up.
    #[serde(default)]
    pub starttls: Option<bool>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// none since ManageSieve registers none.
    #[serde(default = "default_sieve_alpn")]
    pub alpn: Vec<String>,
    /// Whether a mechanism disclosing a reusable credential may run over a
    /// cleartext connection.
    ///
    /// `plain`, `login`, `oauthbearer` and `xoauth2` hand a passive
    /// observer something it can replay, so they are refused unless the
    /// connection is encrypted. Set this for a trusted local link.
    #[serde(default)]
    pub allow_cleartext_auth: bool,
    /// SASL credentials, see [`ImapConfig::sasl`].
    pub sasl: Option<SaslConfig>,
}

/// Proxy configuration.
///
/// `url` is a `socks5://`, `socks5h://` or `http://` proxy URL. Its user
/// info authenticates too, but `username` and `password` keep the secret
/// out of the URL.
#[cfg_attr(
    not(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "sieve"
    )),
    allow(dead_code)
)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProxyConfig {
    /// The proxy URL.
    pub url: String,
    /// The proxy username, required by `password`.
    pub username: Option<String>,
    /// The proxy password.
    pub password: Option<Secret>,
}

impl ProxyConfig {
    /// Resolves an optional configuration, an absent one reading the
    /// environment at connect time.
    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "sieve"
    ))]
    pub fn resolve(config: Option<Self>, resolver: &mut SecretResolver) -> Result<Proxy> {
        match config {
            Some(config) => config.try_into_proxy(resolver),
            None => Ok(Proxy::System),
        }
    }

    /// Resolves the configuration into a runtime [`Proxy`], the password
    /// going through `resolver`.
    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "sieve"
    ))]
    pub fn try_into_proxy(self, resolver: &mut SecretResolver) -> Result<Proxy> {
        let mut proxy = Proxy::from_url(&self.url)?;

        let auth = match (self.username, self.password) {
            (None, None) => return Ok(proxy),
            (None, Some(_)) => bail!("Proxy password requires a username"),
            (Some(user), pass) => ProxyAuth {
                user,
                pass: match pass {
                    Some(pass) => resolver.resolve(pass)?,
                    None => SecretString::default(),
                },
            },
        };

        match &mut proxy {
            Proxy::Socks5 { auth: slot, .. } | Proxy::Http { auth: slot, .. } => {
                *slot = Some(auth);
            }
            Proxy::None | Proxy::System => {}
        }

        Ok(proxy)
    }
}

/// SSL/TLS configuration.
#[cfg_attr(
    not(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "sieve"
    )),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    /// The TLS implementation, defaulting to rustls.
    pub provider: Option<TlsProviderConfig>,
    /// Rustls-only options.
    #[serde(default)]
    pub rustls: RustlsConfig,
    /// A custom certificate to trust, in PEM format.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub cert: Option<PathBuf>,
}

/// SSL/TLS provider configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    /// The pure-Rust rustls stack.
    Rustls,
    /// The platform's own TLS stack.
    NativeTls,
}

/// Rustls configuration.
#[cfg_attr(
    not(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "sieve"
    )),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    /// The cryptographic provider rustls runs on.
    pub crypto: Option<RustlsCryptoConfig>,
}

/// Rustls crypto provider configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RustlsCryptoConfig {
    /// aws-lc-rs.
    Aws,
    /// ring.
    Ring,
}

impl TlsConfig {
    /// Builds the runtime [`Tls`] handle the connect helpers expect,
    /// folding in the protocol-level `alpn` list.
    ///
    /// The TOML schema never exposes `tls.rustls.alpn` directly, the
    /// per-protocol `*.alpn` field standing for it. An empty list skips
    /// ALPN.
    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "sieve"
    ))]
    pub fn into_tls(self, alpn: Vec<String>) -> Tls {
        Tls {
            provider: self.provider.map(|p| match p {
                TlsProviderConfig::Rustls => TlsProvider::Rustls,
                TlsProviderConfig::NativeTls => TlsProvider::NativeTls,
            }),
            rustls: Rustls {
                crypto: self.rustls.crypto.map(|c| match c {
                    RustlsCryptoConfig::Aws => RustlsCrypto::Aws,
                    RustlsCryptoConfig::Ring => RustlsCrypto::Ring,
                }),
                alpn,
            },
            cert: self.cert,
        }
    }
}

/// SASL configuration, exactly one mechanism per `[*.sasl]` block.
///
/// Each variant carries only what its mechanism transmits, and serde picks
/// it from the field name.
#[cfg_attr(
    not(any(feature = "imap", feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
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
#[cfg_attr(
    not(any(feature = "imap", feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    /// The optional trace message handed to the server.
    pub message: Option<String>,
}

/// SASL LOGIN configuration <sup>[draft]</sup>.
///
/// [draft]: https://datatracker.ietf.org/doc/html/draft-murchison-sasl-login-00
#[cfg_attr(
    not(any(feature = "imap", feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
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
#[cfg_attr(
    not(any(feature = "imap", feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
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
#[cfg_attr(
    not(any(feature = "imap", feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
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
#[cfg_attr(
    not(any(feature = "imap", feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
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
#[cfg_attr(
    not(any(feature = "imap", feature = "smtp", feature = "sieve")),
    allow(dead_code)
)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslScramSha256Config {
    /// The account to authenticate as.
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    /// Its password.
    pub password: Secret,
}

impl SaslConfig {
    /// Resolves the configuration into a runtime [`Sasl`].
    ///
    /// The host and port come from the live server URL. OAUTHBEARER alone
    /// reads them, echoing them in its GS2 header.
    ///
    /// The credential goes through `resolver`, so an account whose IMAP,
    /// SMTP and ManageSieve blocks name one command spawns it once.
    #[cfg(any(feature = "imap", feature = "smtp", feature = "sieve"))]
    pub fn try_into_sasl(
        self,
        host: impl ToString,
        port: u16,
        resolver: &mut SecretResolver,
    ) -> Result<Sasl> {
        Ok(match self {
            SaslConfig::Anonymous(c) => Sasl::Anonymous(SaslAnonymousCreds { message: c.message }),
            SaslConfig::Login(c) => Sasl::Login(SaslLoginCreds {
                username: c.username,
                password: resolver.resolve(c.password)?,
            }),
            SaslConfig::Plain(c) => Sasl::Plain(SaslPlainCreds {
                authzid: c.authzid,
                authcid: c.authcid,
                passwd: resolver.resolve(c.passwd)?,
            }),
            SaslConfig::Oauthbearer(c) => Sasl::Oauthbearer(SaslOauthbearerCreds {
                username: c.username,
                host: host.to_string(),
                port,
                token: resolver.resolve(c.token)?,
            }),
            SaslConfig::Xoauth2(c) => Sasl::Xoauth2(SaslXoauth2Creds {
                username: c.username,
                token: resolver.resolve(c.token)?,
            }),
            // NOTE: an empty nonce means draw one for me: the client fills
            // it in, an I/O-free coroutine having no randomness of its own.
            SaslConfig::ScramSha256(c) => Sasl::ScramSha256(SaslScramCreds {
                username: c.username,
                password: resolver.resolve(c.password)?,
                nonce: Vec::new(),
                channel_binding: SaslGs2ChannelBinding::Unsupported,
            }),
        })
    }
}

/// JMAP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JmapConfig {
    /// The JMAP server address, a bare authority or a full URL.
    ///
    /// A bare authority is discovered through `GET /.well-known/jmap`,
    /// a full URL reaching the session endpoint directly. The schemes are
    /// `http` and `https`, a bare authority taking `https`. The host is
    /// ASCII, an internationalised one written in Punycode (`xn--…`).
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, JMAP riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_jmap_alpn")]
    pub alpn: Vec<String>,
    /// Authentication configuration.
    pub auth: JmapAuthConfig,
    /// Identity id `message send` submits under, required for JMAP send
    /// alone and discoverable with `himalaya jmap identity get`.
    pub identity_id: Option<String>,
    /// Mailbox id `message send` stages a message in before submitting it.
    ///
    /// Required for JMAP send alone, and discoverable with `himalaya jmap
    /// mailbox query --role drafts`.
    pub drafts_mailbox_id: Option<String>,
    /// Mailbox id a sent message is moved to once submitted, defaulting
    /// to the `sent`-role mailbox.
    ///
    /// Discoverable with `himalaya jmap mailbox query --role sent`.
    pub sent_mailbox_id: Option<String>,
}

/// JMAP authentication configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum JmapAuthConfig {
    /// A full `Authorization` header value, sent verbatim.
    Header(Secret),
    /// An OAuth 2.0 bearer token.
    Bearer {
        /// The access token.
        token: Secret,
    },
    /// HTTP Basic authentication.
    Basic {
        /// The account to authenticate as.
        #[serde(deserialize_with = "shell_expanded_string")]
        username: String,
        /// Its password.
        password: Secret,
    },
}

/// Gmail REST API configuration.
///
/// Gmail has no per-account server address, the client always talking to
/// gmail.googleapis.com, so only the mailbox owner, TLS and the OAuth 2.0
/// credential are configured.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GmailConfig {
    /// The mailbox owner, defaulting to `me`, the authenticated user.
    #[serde(default = "default_gmail_user_id")]
    pub user_id: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, the REST API riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_gmail_alpn")]
    pub alpn: Vec<String>,
    /// Authentication configuration.
    pub auth: GmailAuthConfig,
}

fn default_gmail_user_id() -> String {
    String::from("me")
}

fn default_gmail_alpn() -> Vec<String> {
    vec![String::from("http/1.1")]
}

/// Gmail authentication configuration.
///
/// Gmail accepts OAuth 2.0 bearer tokens alone, so this is a short-lived
/// access token an external helper such as ortie mints. Refreshing it is
/// the caller's business.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GmailAuthConfig {
    /// The access token, sent as `Bearer <token>`.
    pub token: Secret,
}

/// Microsoft Graph API configuration.
///
/// Graph has no per-account server address, the client always talking to
/// graph.microsoft.com, so only the mailbox owner, TLS and the OAuth 2.0
/// credential are configured.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MsgraphConfig {
    /// The mailbox owner, defaulting to `me`, the authenticated user.
    ///
    /// Set it to a user id or a principal name to target another mailbox.
    #[serde(default = "default_msgraph_user_id")]
    pub user_id: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, the Graph API riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_msgraph_alpn")]
    pub alpn: Vec<String>,
    /// Authentication configuration.
    pub auth: MsgraphAuthConfig,
}

fn default_msgraph_user_id() -> String {
    String::from("me")
}

fn default_msgraph_alpn() -> Vec<String> {
    vec![String::from("http/1.1")]
}

/// Microsoft Graph authentication configuration.
///
/// Graph accepts OAuth 2.0 bearer tokens alone, so this is a short-lived
/// access token an external helper such as ortie mints. Refreshing it is
/// the caller's business.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MsgraphAuthConfig {
    /// The access token, sent as `Bearer <token>`.
    pub token: Secret,
}

#[cfg(test)]
mod tests {
    use std::{env::var, fs, io::Write};

    use pimalaya_config::sample::{Variant, variants};
    use tempfile::NamedTempFile;

    use crate::config::*;

    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "sieve"
    ))]
    const IMAP: &[&str] = &["imap", "imaps"];

    /// Run after run the map order changes, so the error has to come
    /// whichever default the iteration would have met first.
    #[test]
    fn several_defaults_are_refused_unless_an_account_is_named() {
        let toml = "[accounts.work]\ndefault = true\n\
                    [accounts.perso]\ndefault = true\n\
                    [accounts.spare]\n";

        let mut config: Config = toml::from_str(toml).unwrap();
        let err = config.take_account(None).unwrap_err().to_string();
        assert!(err.contains("(perso, work)"), "{err}");

        let mut config: Config = toml::from_str(toml).unwrap();
        let err = config.take_account(Some("default")).unwrap_err();
        assert!(err.to_string().starts_with("Several accounts"), "{err}");

        let mut config: Config = toml::from_str(toml).unwrap();
        let (name, _) = config.take_account(Some("spare")).unwrap();
        assert_eq!(name, "spare");
    }

    /// The v2 `envelope.list.table` block loads whatever it holds, while
    /// `envelope.list` still refuses an unknown key of its own, and a
    /// `mailbox` colour key stays refused.
    #[test]
    fn envelope_table_is_ignored() {
        let toml = "[envelope.list.table]\nunseen-char = \"N\"\nsubject-color = \"green\"\n\
                    [accounts.example]\nenvelope.list.table.flagged-char = \"F\"\n";
        toml::from_str::<Config>(toml).unwrap();

        let toml = "[accounts.example]\nenvelope.list.datetime-fnt = \"%F\"\n";
        let err = toml::from_str::<Config>(toml).unwrap_err().to_string();
        assert!(err.contains("datetime-fnt"), "{err}");

        let toml = "[mailbox.list.table]\nname-color = \"blue\"\n[accounts]\n";
        let err = toml::from_str::<Config>(toml).unwrap_err().to_string();
        assert!(err.contains("unknown field `list`"), "{err}");
    }

    /// An `m2dir` block loads as if absent, whatever it holds, and the
    /// error of an account left with no backend names its removal.
    #[test]
    fn m2dir_block_names_its_removal() {
        let toml =
            "[accounts.example]\ndefault = true\nm2dir.root = \"~/Mail\"\nm2dir.unknown = true\n";
        let mut config: Config = toml::from_str(toml).unwrap();
        let (name, account) = config.take_account(None).unwrap();

        let err = account.no_backend_error(&name, Backend::Auto, &[]);
        let err = err.to_string();
        assert!(err.contains("`m2dir` block is ignored"), "{err}");
        assert!(err.contains("MIGRATION.md"), "{err}");
    }

    #[test]
    fn relative_dates_are_refused() {
        let toml = "[envelope.list]\ndatetime-relative = true\n[accounts]\n";
        let err = toml::from_str::<Config>(toml).unwrap_err().to_string();
        assert!(err.contains("datetime-relative"), "{err}");
    }

    #[test]
    fn maildir_root_alone_keeps_keywords_off() {
        let config: MaildirConfig = toml::from_str(r#"root = "/tmp/mail""#).unwrap();
        assert!(!config.keywords.dovecot);
        assert_eq!(config.keywords.header, None);
    }

    #[test]
    fn maildir_keyword_options_parse() {
        let config: MaildirConfig = toml::from_str(
            r#"
                root = "/tmp/mail"
                keywords.dovecot = true
                keywords.header = "x-label"
            "#,
        )
        .unwrap();

        assert!(config.keywords.dovecot);
        assert_eq!(
            config.keywords.header,
            Some(MaildirKeywordHeaderConfig::XLabel)
        );
    }

    #[test]
    fn mbox_root_alone_takes_both_locks_and_writes_mboxrd() {
        let config: MboxConfig = toml::from_str(r#"root = "/tmp/mail""#).unwrap();

        assert_eq!(config.inbox, None);
        assert_eq!(config.format, MboxFormatConfig::Mboxrd);
        assert!(!config.thunderbird);
        assert_eq!(config.lock, MboxLockConfig::default());
        assert!(config.lock.dotlock && config.lock.fcntl);
    }

    #[test]
    fn mbox_options_parse() {
        let config: MboxConfig = toml::from_str(
            r#"
                root = "/tmp/mail"
                inbox = "/var/mail/me"
                format = "mboxcl2"
                thunderbird = true
                lock.dotlock = false
                lock.timeout = 30
            "#,
        )
        .unwrap();

        assert_eq!(config.inbox, Some(PathBuf::from("/var/mail/me")));
        assert_eq!(config.format, MboxFormatConfig::Mboxcl2);
        assert!(config.thunderbird);
        assert!(!config.lock.dotlock && config.lock.fcntl);
        assert_eq!(config.lock.timeout, Some(30));
    }

    #[test]
    fn sieve_config_defaults_to_no_alpn_and_refuses_cleartext_auth() {
        let config: SieveConfig = toml::from_str(
            r#"
                server = "sieve.example.com:4190"
            "#,
        )
        .unwrap();

        assert_eq!(config.server, "sieve.example.com:4190");
        assert_eq!(config.starttls, None);
        assert!(config.alpn.is_empty());
        assert!(!config.allow_cleartext_auth);
        assert!(config.sasl.is_none());
    }

    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "sieve"
    ))]
    #[test]
    fn bare_host_defaults_to_secure_scheme() {
        let url = parse_server("mail.example.com", "imaps", IMAP).unwrap();
        assert_eq!(url.scheme(), "imaps");
        assert_eq!(url.host_str(), Some("mail.example.com"));
        // NOTE: with no explicit port, the backend client applies the
        // protocol default rather than this parser.
        assert_eq!(url.port(), None);
    }

    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "sieve"
    ))]
    #[test]
    fn bare_host_port_keeps_port_and_secure_scheme() {
        let url = parse_server("mail.example.com:1993", "imaps", IMAP).unwrap();
        assert_eq!(url.scheme(), "imaps");
        assert_eq!(url.host_str(), Some("mail.example.com"));
        assert_eq!(url.port(), Some(1993));
    }

    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "sieve"
    ))]
    #[test]
    fn full_url_scheme_host_port_is_kept_verbatim() {
        let url = parse_server("imap://mail.example.com:143", "imaps", IMAP).unwrap();
        assert_eq!(url.scheme(), "imap");
        assert_eq!(url.host_str(), Some("mail.example.com"));
        assert_eq!(url.port(), Some(143));
    }

    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "sieve"
    ))]
    #[test]
    fn unknown_scheme_is_rejected() {
        let err = parse_server("ftp://mail.example.com", "imaps", IMAP).unwrap_err();
        assert!(err.to_string().contains("Invalid server scheme `ftp`"));
    }

    /// himalaya-tui spells the identity `from` and `from-name`, and one
    /// file backs both binaries, so a config it wrote must reach the same
    /// fields the composers read.
    #[test]
    fn the_composing_config_reads_under_the_tui_spelling() {
        let config: Config = toml::from_str(
            r#"
            from-name = "Alice"
            signature = "Alice"

            [accounts.example]
            from = "alice@example.org"
            from-name = "Alice at work"
            signature-delim = "~~~\n"
            "#,
        )
        .expect("the himalaya-tui spelling must deserialize");

        let account = config.accounts.get("example").expect("the example account");

        assert_eq!(config.display_name.as_deref(), Some("Alice"));
        assert_eq!(config.signature.as_deref(), Some("Alice"));
        assert_eq!(account.email.as_deref(), Some("alice@example.org"));
        assert_eq!(account.display_name.as_deref(), Some("Alice at work"));
        assert_eq!(account.signature_delim.as_deref(), Some("~~~\n"));
    }

    #[test]
    fn backends_inherit_the_account_proxy_unless_they_name_one() {
        let mut config: Config = toml::from_str(
            r#"
            [accounts.example]
            proxy.url = "socks5h://account:1080"
            imap.server = "imap.example.org"
            smtp.server = "smtp.example.org"
            smtp.proxy.url = "http://smtp:3128"
            "#,
        )
        .expect("the proxy blocks must deserialize");

        let (_, account) = config.take_named_account("example").unwrap();
        let proxy = |proxy: Option<ProxyConfig>| proxy.map(|proxy| proxy.url);

        assert_eq!(
            proxy(account.imap.unwrap().proxy).as_deref(),
            Some("socks5h://account:1080")
        );
        assert_eq!(
            proxy(account.smtp.unwrap().proxy).as_deref(),
            Some("http://smtp:3128")
        );
    }

    #[test]
    fn local_roots_expand_the_leading_tilde() {
        let home = PathBuf::from(var("HOME").expect("HOME must be set"));

        let maildir: MaildirConfig = toml::from_str(r#"root = "~/Mail""#).unwrap();
        let pimdir: PimdirConfig = toml::from_str(r#"root = "~/Mail""#).unwrap();
        let mbox: MboxConfig = toml::from_str(
            r#"root = "~/Mail"
inbox = "~/spool""#,
        )
        .unwrap();

        assert_eq!(maildir.root, home.join("Mail"));
        assert_eq!(pimdir.root, home.join("Mail"));
        assert_eq!(mbox.root, home.join("Mail"));
        assert_eq!(mbox.inbox, Some(home.join("spool")));
    }

    #[test]
    fn optional_paths_expand_the_leading_tilde_and_stay_absent() {
        let home = PathBuf::from(var("HOME").expect("HOME must be set"));

        let tls: TlsConfig = toml::from_str(r#"cert = "~/ca.pem""#).unwrap();
        assert_eq!(tls.cert, Some(home.join("ca.pem")));

        let tls: TlsConfig = toml::from_str("").unwrap();
        assert_eq!(tls.cert, None);
    }

    #[cfg(any(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "sieve"
    ))]
    #[test]
    fn path_is_preserved_for_full_url() {
        let url = parse_server("https://example.com/jmap/session", "https", &["https"]).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.path(), "/jmap/session");
    }

    /// The sections of config.sample.toml, one of which a user
    /// uncomments.
    const SECTIONS: [&str; 10] = [
        "# IMAP and SMTP",
        "# ManageSieve",
        "# JMAP",
        "# Gmail",
        "# Microsoft Graph",
        "# Microsoft IMAP and SMTP",
        "# Fastmail",
        "# Maildir",
        "# mbox",
        "# pimdir",
    ];

    /// Checks config.sample.toml: one `[accounts.example]` table and no
    /// other, its sections each parsed through the loader `-c` uses with
    /// every variant on top, `example` resolving as the default account
    /// and declaring a backend.
    #[test]
    fn the_sample_configuration_parses_with_each_variant() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.sample.toml");
        let sample = fs::read_to_string(&path).unwrap();
        let headers: Vec<_> = sample.lines().filter(|l| l.starts_with('[')).collect();
        assert_eq!(headers, ["[accounts.example]"]);

        for Variant { name, text } in variants(&sample, &SECTIONS) {
            let context = format!("config.sample.toml with {name} uncommented");

            let mut file = NamedTempFile::new().unwrap();
            file.write_all(text.as_bytes()).unwrap();

            let mut config = Config::load(&[file.path().to_path_buf()])
                .unwrap_or_else(|err| panic!("{context}: {err:?}"));
            let (account, account_config) = config
                .take_account(None)
                .unwrap_or_else(|err| panic!("{context}: {err:?}"));

            let blocks = [
                account_config.imap.is_some(),
                account_config.jmap.is_some(),
                account_config.gmail.is_some(),
                account_config.msgraph.is_some(),
                account_config.maildir.is_some(),
                account_config.mbox.is_some(),
                account_config.pimdir.is_some(),
                account_config.sieve.is_some(),
            ];

            assert_eq!(account, "example", "{context}");
            assert!(blocks.contains(&true), "{context}: no backend declared");
        }
    }
}
