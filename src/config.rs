//! # Configuration
//!
//! The TOML schema: a global block plus named account blocks, each
//! carrying the optional per-backend sub-blocks its protocols need.
//!
//! Backend defaults are duplicated here rather than read from the io-*
//! crates, so the schema compiles under any feature subset, none included.

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, bail};
use comfy_table::ContentArrangement;
use crossterm::style::Color;
use io_sasl::{
    login::SaslLoginCreds, mechanism::Sasl, rfc4505::anonymous::SaslAnonymousCreds,
    rfc4616::plain::SaslPlainCreds, rfc5801::SaslGs2ChannelBinding, rfc5802::SaslScramCreds,
    rfc7628::oauthbearer::SaslOauthbearerCreds, xoauth2::SaslXoauth2Creds,
};
use pimalaya_config::{
    secret::Secret,
    toml::{TomlConfig, shell_expanded_string},
};
use pimalaya_stream::tls::{Rustls, RustlsCrypto, Tls, TlsProvider};
use serde::{Deserialize, Serialize};
use url::Url;

/// Skips a field equal to its type's default, so a wizard-generated
/// configuration omits defaulted scalars.
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

fn is_default_imap_alpn(alpn: &[String]) -> bool {
    alpn == default_imap_alpn().as_slice()
}

fn is_default_smtp_alpn(alpn: &[String]) -> bool {
    alpn == default_smtp_alpn().as_slice()
}

fn is_default_sieve_alpn(alpn: &[String]) -> bool {
    alpn.is_empty()
}

fn is_default_jmap_alpn(alpn: &[String]) -> bool {
    alpn == default_jmap_alpn().as_slice()
}

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

fn is_default_gmail_alpn(alpn: &[String]) -> bool {
    alpn == default_gmail_alpn().as_slice()
}

fn is_default_msgraph_alpn(alpn: &[String]) -> bool {
    alpn == default_msgraph_alpn().as_slice()
}

/// The whole TOML configuration file.
///
/// `deny_unknown_fields` is omitted so one file can be shared with
/// himalaya-tui, whose own top-level fields are ignored here.
#[derive(Debug, Default, Deserialize, Serialize)]
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
    pub downloads_dir: Option<PathBuf>,
    /// Table rendering quirks shared by every listing.
    #[serde(default)]
    pub table: TableConfig,
    /// `envelope list` rendering options.
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    /// Mailbox aliases and `mailbox list` rendering options.
    #[serde(default)]
    pub mailbox: MailboxConfig,
    /// `attachment list` rendering options.
    #[serde(default)]
    pub attachment: AttachmentConfig,
    /// `account list` rendering options, global only: the listing of
    /// accounts belongs to no account, so nothing overrides it.
    #[serde(default)]
    pub account: AccountListingConfig,
    /// The named `[accounts.<name>]` blocks.
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

/// The order a rendered account groups its keys in, most defining first.
///
/// A key outside this list still renders, after the listed ones, so a
/// field added to [`AccountConfig`] can never go missing from a generated
/// document just because nobody updated this table.
const RENDER_ORDER: [&str; 18] = [
    "default",
    "email",
    "display-name",
    "signature",
    "signature-delim",
    "imap",
    "jmap",
    "gmail",
    "msgraph",
    "maildir",
    "m2dir",
    "pimdir",
    "smtp",
    "sieve",
    "mailbox",
    "envelope",
    "attachment",
    "table",
];

impl AccountConfig {
    /// Renders this account as an `[accounts.<name>]` block.
    ///
    /// What this adds over the serializer is reading order: dotted keys
    /// come out alphabetically, burying `imap.server` under the
    /// credentials authenticating against it. Groups are reordered and
    /// each endpoint lifted to the top of its own.
    pub fn render(&self, name: &str) -> Result<String> {
        // NOTE: borrowed rather than built into a `Config`, which would
        // mean cloning the account, and so deriving `Clone` down every
        // backend config, to render it.
        #[derive(Serialize)]
        struct AccountDocument<'a> {
            accounts: HashMap<&'a str, &'a AccountConfig>,
        }

        let document = AccountDocument {
            accounts: HashMap::from([(name, self)]),
        };
        let rendered = pimalaya_config::toml::to_string(&document)?;

        let (header, body) = match rendered.split_once('\n') {
            Some((header, body)) => (header, body),
            None => return Ok(rendered),
        };

        let mut groups: Vec<(String, Vec<&str>)> = Vec::new();

        for line in body.lines().filter(|line| !line.trim().is_empty()) {
            let key = line.split(['.', ' ']).next().unwrap_or(line).to_string();

            match groups.iter_mut().find(|(name, _)| *name == key) {
                Some((_, lines)) => lines.push(line),
                None => groups.push((key, vec![line])),
            }
        }

        groups.sort_by_key(|(key, _)| {
            RENDER_ORDER
                .iter()
                .position(|known| known == key)
                .unwrap_or(RENDER_ORDER.len())
        });

        let mut document = format!("{header}\n");

        for (index, (key, mut lines)) in groups.into_iter().enumerate() {
            if index > 0 {
                document.push('\n');
            }

            // NOTE: the endpoint is what the group is about, so it reads
            // first, the credentials and the quirks qualifying it.
            let server = format!("{key}.server ");
            lines.sort_by_key(|line| !line.starts_with(&server));

            for line in lines {
                document.push_str(line);
                document.push('\n');
            }
        }

        Ok(document)
    }
}

/// One `[accounts.<name>]` block.
///
/// `deny_unknown_fields` is omitted so a block written for one binary
/// loads in the other: the CLI-only sub-blocks have to be tolerated by
/// himalaya-tui, which models the rest.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountConfig {
    /// Whether a command with no `-a/--account` runs against this one.
    #[serde(default, skip_serializing_if = "is_default")]
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
    pub downloads_dir: Option<PathBuf>,
    /// Table rendering quirks shared by every listing.
    #[serde(default)]
    pub table: TableConfig,
    /// `envelope list` rendering options.
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    /// Mailbox aliases and `mailbox list` rendering options.
    #[serde(default)]
    pub mailbox: MailboxConfig,
    /// `attachment list` rendering options.
    #[serde(default)]
    pub attachment: AttachmentConfig,
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
    /// The m2dir backend of this account.
    #[allow(unused)]
    pub m2dir: Option<M2dirConfig>,
    /// The pimdir backend of this account.
    #[allow(unused)]
    pub pimdir: Option<PimdirConfig>,
    /// The SMTP transport of this account.
    #[allow(unused)]
    pub smtp: Option<SmtpConfig>,
    /// The ManageSieve endpoint of this account.
    #[allow(unused)]
    pub sieve: Option<SieveConfig>,
}

/// Envelope-level rendering options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeConfig {
    /// `envelope list` rendering options.
    #[serde(default)]
    pub list: EnvelopeListConfig,
}

/// Mailbox aliases and `mailbox list` rendering options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxConfig {
    /// Friendly names mapped to backend-native mailbox ids, resolved
    /// case-insensitively.
    ///
    /// The `inbox` alias doubles as the implicit default mailbox of a
    /// shared command omitting `-m/--mailbox`.
    #[serde(default, rename = "alias", alias = "aliases")]
    pub aliases: HashMap<String, String>,
    /// `mailbox list` rendering options.
    #[serde(default)]
    pub list: MailboxListConfig,
}

/// `mailbox list` rendering options under `mailbox.list.*`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxListConfig {
    /// Per-column colors of the rendered table.
    #[serde(default)]
    pub table: MailboxListTableConfig,
}

/// Per-column color overrides for the `mailbox list` table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxListTableConfig {
    /// Color of the ID column.
    pub id_color: Option<Color>,
    /// Color of the NAME column.
    pub name_color: Option<Color>,
    /// Color of the TOTAL column.
    pub total_color: Option<Color>,
    /// Color of the UNREAD column.
    pub unread_color: Option<Color>,
}

/// `attachment list` rendering options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AttachmentConfig {
    /// `attachment list` rendering options.
    #[serde(default)]
    pub list: AttachmentListConfig,
}

/// `attachment list` rendering options under `attachment.list.*`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AttachmentListConfig {
    /// Per-column colors of the rendered table.
    #[serde(default)]
    pub table: AttachmentListTableConfig,
}

/// Per-column color overrides for the `attachment list` table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AttachmentListTableConfig {
    /// Color of the ID column.
    pub id_color: Option<Color>,
    /// Color of the FILENAME column.
    pub filename_color: Option<Color>,
    /// Color of the TYPE column.
    pub type_color: Option<Color>,
    /// Color of the SIZE column.
    pub size_color: Option<Color>,
    /// Color of the INLINE column.
    pub inline_color: Option<Color>,
    /// Color of the PATH column.
    pub path_color: Option<Color>,
}

/// `account list` rendering options, top-level only.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountListingConfig {
    /// `account list` rendering options.
    #[serde(default)]
    pub list: AccountListingListConfig,
}

/// `account list` rendering options under `account.list.*`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountListingListConfig {
    /// Per-column colors of the rendered table.
    #[serde(default)]
    pub table: AccountListingTableConfig,
}

/// Per-column color overrides for the `account list` table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountListingTableConfig {
    /// Color of the NAME column.
    pub name_color: Option<Color>,
    /// Color of the BACKENDS column.
    pub backends_color: Option<Color>,
    /// Color of the DEFAULT column.
    pub default_color: Option<Color>,
}

/// `envelope list` rendering options under `envelope.list.*`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeListConfig {
    /// chrono `strftime` format of the DATE column, defaulting to
    /// `"%F %R%:z"`.
    pub datetime_fmt: Option<String>,
    /// Whether the `Date:` offset is converted to the local timezone
    /// before formatting, the default `false` keeping the wire offset.
    pub datetime_local_tz: Option<bool>,
    /// Default `-s/--page-size`, the flag winning when passed and 25
    /// being the hard fallback.
    pub page_size: Option<u32>,
    /// Per-column colors and flag glyphs of the rendered table.
    ///
    /// A color is a named [crossterm color], or an `{ Rgb = { r, g, b } }`
    /// or `{ AnsiValue = N }` table.
    ///
    /// [crossterm color]: https://docs.rs/crossterm/latest/crossterm/style/enum.Color.html
    #[serde(default)]
    pub table: EnvelopeListTableConfig,
}

/// Per-column color and flag glyph overrides for the envelopes table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeListTableConfig {
    /// FLAGS glyph of a message lacking `\Seen`, defaulting to `*`.
    pub unseen_char: Option<char>,
    /// FLAGS glyph of a message carrying `\Answered`, defaulting to `R`.
    pub replied_char: Option<char>,
    /// FLAGS glyph of a message carrying `\Flagged`, defaulting to `!`.
    pub flagged_char: Option<char>,
    /// ATT glyph of a message with an attachment, defaulting to `@`.
    pub attachment_char: Option<char>,
    /// Color of the ID column.
    pub id_color: Option<Color>,
    /// Color of the FLAGS column.
    pub flags_color: Option<Color>,
    /// Color of the ATT column.
    pub att_color: Option<Color>,
    /// Color of the SUBJECT column.
    pub subject_color: Option<Color>,
    /// Color of the FROM column.
    pub from_color: Option<Color>,
    /// Color of the TO column.
    pub to_color: Option<Color>,
    /// Color of the DATE column.
    pub date_color: Option<Color>,
    /// Color of the SIZE column.
    pub size_color: Option<Color>,
}

/// Table rendering quirks shared by every listing.
///
/// The per-column colors live under `*.list.table.*-color` instead, one
/// block per listing.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TableConfig {
    /// One character per table component, a space skipping it,
    /// defaulting to `UTF8_FULL_CONDENSED`.
    ///
    /// [`style_from_preset`] documents the component order.
    ///
    /// [`style_from_preset`]: crate::shared::table::style_from_preset
    pub preset: Option<String>,
    /// Column-arrangement strategy, defaulting to `dynamic`.
    pub arrangement: Option<TableArrangementConfig>,
}

/// Column-arrangement strategy for rendered tables.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TableArrangementConfig {
    /// Fit the columns to the terminal width.
    #[default]
    Dynamic,
    /// Fit the columns to the terminal width, always filling it.
    DynamicFullWidth,
    /// Let each column take the width of its widest cell.
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

/// Parses a backend `server` string into a [`Url`].
///
/// A full URL, a bare authority or a bare host, the two bare forms taking
/// the default scheme. Absence of `://` is what detects them, the parser
/// otherwise reading `mail.example.com:993` as a scheme.
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, valid only for an `imap://` server.
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: bool,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// the RFC 7595 registered `["imap"]`.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(
        default = "default_imap_alpn",
        skip_serializing_if = "is_default_imap_alpn"
    )]
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
    #[serde(default, skip_serializing_if = "is_default")]
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
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapSortConfig {
    /// Forces the client-side sort fallback on or off.
    ///
    /// On, the client sorts with SEARCH and FETCH; off, it always issues a
    /// server `SORT`. Unset, the fallback runs only when the server lacks
    /// the SORT capability.
    pub fallback: Option<bool>,
}

/// Per-account `imap.id.*` quirks.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdConfig {
    /// Whether the auth coroutine chains an `ID` round-trip after the
    /// tagged auth response, default `false` skipping it.
    #[serde(default, skip_serializing_if = "is_default")]
    pub auto: bool,
    /// Parameters sent with the auto-`ID` command, empty sending `ID NIL`.
    ///
    /// `true` substitutes himalaya's canned value for a well-known key
    /// (`name`, `version`, `vendor`, `support-url`) and `NIL` otherwise,
    /// `false` always sends `NIL`, and an absent key is not transmitted.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, bool>,
}

/// Header carrying custom keywords inline with a message body.
///
/// Mirrors io-maildir's `KeywordHeader`, kept local so the config
/// schema does not depend on any backend crate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaildirKeywordHeaderConfig {
    /// `X-Keywords`, comma-separated (OfflineIMAP, mbsync).
    XKeywords,
    /// `X-Label`, space-separated (mutt, notmuch).
    XLabel,
}

/// Per-account `maildir.keywords.*` options, both unset reading none.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirKeywordsConfig {
    /// Whether keywords are resolved through each mailbox's own
    /// dovecot-keywords file, which maps a lowercase info-section letter
    /// to a keyword, default `false` leaving those letters unread.
    #[serde(default, skip_serializing_if = "is_default")]
    pub dovecot: bool,
    /// The body header keywords are read from, unset reading none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<MaildirKeywordHeaderConfig>,
}

/// Maildir configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirConfig {
    /// The Maildir root, one directory per mailbox below it.
    pub root: PathBuf,
    /// How custom, non-IANA keywords are read, if at all.
    #[serde(default)]
    pub keywords: MaildirKeywordsConfig,
}

/// m2dir configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct M2dirConfig {
    /// The m2dir root, one directory per mailbox below it.
    pub root: PathBuf,
}

/// pimdir configuration, a local store read as an offline cache.
///
/// The store is a SQLite index beside content-addressed blobs, populated
/// by the Neverest sync engine.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PimdirConfig {
    /// The store directory, holding `pimdir.db` and `objects/`.
    pub root: PathBuf,
    /// The sync engine account whose collections this client reads,
    /// per pimdir SPEC section 9.2.
    ///
    /// Usually left unset, a store synced by one account being read as
    /// that one. Set it for a store several accounts share, where guessing
    /// would show the wrong mailbox set.
    #[serde(default)]
    pub account: Option<String>,
}

/// SMTP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, valid only for an `smtp://` server.
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: bool,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// the RFC 7595 registered `["smtp"]`.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(
        default = "default_smtp_alpn",
        skip_serializing_if = "is_default_smtp_alpn"
    )]
    pub alpn: Vec<String>,
    /// SASL credentials, see [`ImapConfig::sasl`].
    pub sasl: Option<SaslConfig>,
}

/// ManageSieve configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, unset following the scheme.
    ///
    /// On for `sieve://`, the only upgrade path RFC 5804 defines, off for
    /// `sieves://` and `unix://`. Setting it on `sieves://` is an error,
    /// TLS being already up.
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: Option<bool>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// none since ManageSieve registers none.
    #[serde(
        default = "default_sieve_alpn",
        skip_serializing_if = "is_default_sieve_alpn"
    )]
    pub alpn: Vec<String>,
    /// Whether a mechanism disclosing a reusable credential may run over a
    /// cleartext connection.
    ///
    /// `plain`, `login`, `oauthbearer` and `xoauth2` hand a passive
    /// observer something it can replay, so they are refused unless the
    /// connection is encrypted. Set this for a trusted local link.
    #[serde(default, skip_serializing_if = "is_default")]
    pub allow_cleartext_auth: bool,
    /// SASL credentials, see [`ImapConfig::sasl`].
    pub sasl: Option<SaslConfig>,
}

/// SSL/TLS configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    /// The TLS implementation, defaulting to rustls.
    pub provider: Option<TlsProviderConfig>,
    /// Rustls-only options.
    #[serde(default)]
    pub rustls: RustlsConfig,
    /// A custom certificate to trust, in PEM format.
    pub cert: Option<PathBuf>,
}

/// SSL/TLS provider configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    /// The pure-Rust rustls stack.
    Rustls,
    /// The platform's own TLS stack.
    NativeTls,
}

/// Rustls configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    /// The cryptographic provider rustls runs on.
    pub crypto: Option<RustlsCryptoConfig>,
}

/// Rustls crypto provider configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    /// The optional trace message handed to the server.
    pub message: Option<String>,
}

/// SASL LOGIN configuration <sup>[draft]</sup>.
///
/// [draft]: https://datatracker.ietf.org/doc/html/draft-murchison-sasl-login-00
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub fn try_into_sasl(self, host: impl ToString, port: u16) -> Result<Sasl> {
        Ok(match self {
            SaslConfig::Anonymous(c) => Sasl::Anonymous(SaslAnonymousCreds { message: c.message }),
            SaslConfig::Login(c) => Sasl::Login(SaslLoginCreds {
                username: c.username,
                password: c.password.get()?,
            }),
            SaslConfig::Plain(c) => Sasl::Plain(SaslPlainCreds {
                authzid: c.authzid,
                authcid: c.authcid,
                passwd: c.passwd.get()?,
            }),
            SaslConfig::Oauthbearer(c) => Sasl::Oauthbearer(SaslOauthbearerCreds {
                username: c.username,
                host: host.to_string(),
                port,
                token: c.token.get()?,
            }),
            SaslConfig::Xoauth2(c) => Sasl::Xoauth2(SaslXoauth2Creds {
                username: c.username,
                token: c.token.get()?,
            }),
            // NOTE: an empty nonce means draw one for me: the client fills
            // it in, an I/O-free coroutine having no randomness of its own.
            SaslConfig::ScramSha256(c) => Sasl::ScramSha256(SaslScramCreds {
                username: c.username,
                password: c.password.get()?,
                nonce: Vec::new(),
                channel_binding: SaslGs2ChannelBinding::Unsupported,
            }),
        })
    }
}

/// JMAP configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JmapConfig {
    /// The JMAP server address, a bare authority or a full URL.
    ///
    /// A bare authority is discovered through `GET /.well-known/jmap`,
    /// a full URL reaching the session endpoint directly. The schemes are
    /// `http`, `https`, `jmap` and `jmaps`.
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, JMAP riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(
        default = "default_jmap_alpn",
        skip_serializing_if = "is_default_jmap_alpn"
    )]
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
}

/// JMAP authentication configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GmailConfig {
    /// The mailbox owner, defaulting to `me`, the authenticated user.
    #[serde(default = "default_gmail_user_id")]
    pub user_id: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, the REST API riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(
        default = "default_gmail_alpn",
        skip_serializing_if = "is_default_gmail_alpn"
    )]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, the Graph API riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(
        default = "default_msgraph_alpn",
        skip_serializing_if = "is_default_msgraph_alpn"
    )]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MsgraphAuthConfig {
    /// The access token, sent as `Bearer <token>`.
    pub token: Secret,
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAP: &[&str] = &["imap", "imaps"];

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

    #[test]
    fn bare_host_defaults_to_secure_scheme() {
        let url = parse_server("mail.example.com", "imaps", IMAP).unwrap();
        assert_eq!(url.scheme(), "imaps");
        assert_eq!(url.host_str(), Some("mail.example.com"));
        // NOTE: with no explicit port, the backend client applies the
        // protocol default rather than this parser.
        assert_eq!(url.port(), None);
    }

    #[test]
    fn bare_host_port_keeps_port_and_secure_scheme() {
        let url = parse_server("mail.example.com:1993", "imaps", IMAP).unwrap();
        assert_eq!(url.scheme(), "imaps");
        assert_eq!(url.host_str(), Some("mail.example.com"));
        assert_eq!(url.port(), Some(1993));
    }

    #[test]
    fn full_url_scheme_host_port_is_kept_verbatim() {
        let url = parse_server("imap://mail.example.com:143", "imaps", IMAP).unwrap();
        assert_eq!(url.scheme(), "imap");
        assert_eq!(url.host_str(), Some("mail.example.com"));
        assert_eq!(url.port(), Some(143));
    }

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
    fn path_is_preserved_for_full_url() {
        let url = parse_server("https://example.com/jmap/session", "https", &["https"]).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.path(), "/jmap/session");
    }
}
