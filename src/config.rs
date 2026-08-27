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

/// `skip_serializing_if` predicate skipping a field equal to its type's
/// default, so a wizard-generated config omits defaulted scalars (the
/// only serializer is the wizard, see [`crate::wizard`]).
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

// NOTE: these mirror the io-* crates' `default_alpn()` (IMAP `["imap"]`,
// SMTP `["smtp"]`, JMAP `["http/1.1"]`) but are kept local so the config
// schema does not depend on any backend crate — the config compiles
// under any feature subset, like the Gmail/Graph defaults below.
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

/// Global configuration.
///
/// Represents the whole TOML user's configuration file.
/// `deny_unknown_fields` is intentionally omitted so the same TOML
/// file can be shared with `himalaya-tui`: top-level TUI-only fields
/// (`keybinds`, `theme`) are silently ignored here.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Name the `From` address carries when an account declares none
    /// of its own. See [`AccountConfig::display_name`].
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,

    /// Signature appended to a composed message when an account
    /// declares none of its own. See [`AccountConfig::signature`].
    pub signature: Option<String>,

    /// Separator written before the signature when an account declares
    /// none of its own. See [`AccountConfig::signature_delim`].
    pub signature_delim: Option<String>,

    pub downloads_dir: Option<PathBuf>,
    #[serde(default)]
    pub table: TableConfig,
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    #[serde(default)]
    pub mailbox: MailboxConfig,
    #[serde(default)]
    pub attachment: AttachmentConfig,
    /// `account list` rendering options (global only — there is no
    /// per-account override for the listing of accounts).
    #[serde(default)]
    pub account: AccountListingConfig,
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

/// The order the rendered account groups its keys in, most defining
/// first: what the account is, who it speaks for, then the backend it
/// reads from, the transport it sends over, the mailboxes it names,
/// and last the rendering options.
///
/// A key outside this list still renders, after the ones listed, so a
/// field added to [`AccountConfig`] can never go missing from a
/// generated document just because nobody updated this table.
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
    /// Renders this account as an `[accounts.<name>]` block, ready to be
    /// written to a configuration file or appended to one.
    ///
    /// The serializer decides what is written, so a field left at its
    /// default is omitted and nothing has to be listed here twice. What
    /// this adds is reading order: the flattened dotted keys come out
    /// alphabetically, which buries `imap.server` under the credentials
    /// that authenticate against it, and runs every group together. The
    /// groups are reordered, `server` is lifted to the top of its own,
    /// and a blank line separates them.
    pub fn render(&self, name: &str) -> Result<String> {
        // NOTE: borrowed rather than built into a `Config`, which would
        // mean cloning the account (and so deriving `Clone` down every
        // backend config) to render it. The emitter only looks for an
        // `accounts` table, so any shape carrying one will do.
        #[derive(Serialize)]
        struct AccountDocument<'a> {
            accounts: HashMap<&'a str, &'a AccountConfig>,
        }

        let document = AccountDocument {
            accounts: HashMap::from([(name, self)]),
        };
        let rendered = pimalaya_config::toml::to_string(&document)?;

        // The emitter writes the header itself, and everything below it
        // is one dotted key per line.
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

            // The endpoint is what the group is about, so it reads first;
            // the credentials and the quirks qualify it.
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

/// Account configuration.
///
/// `deny_unknown_fields` is omitted so a block written for one binary
/// loads in the other: every account field himalaya-tui models is
/// modelled here too, but the CLI-only sub-blocks (`table`,
/// `envelope`, `mailbox`, `attachment`) have to be tolerated there.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountConfig {
    #[serde(default, skip_serializing_if = "is_default")]
    pub default: bool,

    /// Address this account sends as, used as the `From` header of a
    /// composed message when `--from` is not passed. Not validated and
    /// not required: an account that never composes has no use for it.
    ///
    /// Aliased to `from`, the spelling himalaya-tui writes, the two
    /// binaries sharing one configuration file.
    #[serde(alias = "from")]
    pub email: Option<String>,

    /// Name the `From` address carries, e.g. `Alice` in
    /// `Alice <alice@example.org>`. Falls back to the global
    /// [`Config::display_name`]. Quoting and encoding are the MIME
    /// builder's business, so write the name as it should read.
    ///
    /// Aliased to `from-name`, the spelling himalaya-tui writes.
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,

    /// Signature appended to a composed message, after the body and
    /// after any quoted source text, when neither `--signature` nor
    /// `--signature-file` is passed. Falls back to the global
    /// [`Config::signature`].
    ///
    /// The value is the signature alone: the separator before it is
    /// [`AccountConfig::signature_delim`]'s business, so a signature
    /// written for one binary reads the same in the other.
    pub signature: Option<String>,

    /// Separator written before the signature, defaulting to the
    /// RFC 3676 §4.3 `"-- \n"`. Falls back to the global
    /// [`Config::signature_delim`].
    ///
    /// Written verbatim, so a value meant to stand on its own line
    /// carries its own trailing newline.
    pub signature_delim: Option<String>,

    pub downloads_dir: Option<PathBuf>,
    #[serde(default)]
    pub table: TableConfig,
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    #[serde(default)]
    pub mailbox: MailboxConfig,
    #[serde(default)]
    pub attachment: AttachmentConfig,

    #[allow(unused)]
    pub imap: Option<ImapConfig>,
    #[allow(unused)]
    pub jmap: Option<JmapConfig>,
    #[allow(unused)]
    pub gmail: Option<GmailConfig>,
    #[allow(unused)]
    pub msgraph: Option<MsgraphConfig>,
    #[allow(unused)]
    pub maildir: Option<MaildirConfig>,
    #[allow(unused)]
    pub m2dir: Option<M2dirConfig>,
    #[allow(unused)]
    pub pimdir: Option<PimdirConfig>,
    #[allow(unused)]
    pub smtp: Option<SmtpConfig>,
    #[allow(unused)]
    pub sieve: Option<SieveConfig>,
}

/// Envelope-level rendering options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeConfig {
    #[serde(default)]
    pub list: EnvelopeListConfig,
}

/// Mailbox-level configuration.
///
/// Exposes user-defined aliases mapping a friendly name to a
/// backend-native id (looked up case-insensitively at resolution
/// time; the `inbox` alias acts as the implicit default mailbox when
/// a shared command omits `-m/--mailbox`) and the `mailboxes list`
/// rendering options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxConfig {
    #[serde(default, rename = "alias", alias = "aliases")]
    pub aliases: HashMap<String, String>,

    #[serde(default)]
    pub list: MailboxListConfig,
}

/// `mailboxes list` rendering options under `mailbox.list.*`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxListConfig {
    #[serde(default)]
    pub table: MailboxListTableConfig,
}

/// Per-column color overrides for the `mailboxes list` table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxListTableConfig {
    pub id_color: Option<Color>,
    pub name_color: Option<Color>,
    pub total_color: Option<Color>,
    pub unread_color: Option<Color>,
}

/// `attachments list` rendering options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AttachmentConfig {
    #[serde(default)]
    pub list: AttachmentListConfig,
}

/// `attachments list` rendering options under `attachment.list.*`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AttachmentListConfig {
    #[serde(default)]
    pub table: AttachmentListTableConfig,
}

/// Per-column color overrides for the `attachments list` table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AttachmentListTableConfig {
    pub id_color: Option<Color>,
    pub filename_color: Option<Color>,
    pub type_color: Option<Color>,
    pub size_color: Option<Color>,
    pub inline_color: Option<Color>,
    pub path_color: Option<Color>,
}

/// `account list` rendering options. Top-level only — there is no
/// per-account override.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountListingConfig {
    #[serde(default)]
    pub list: AccountListingListConfig,
}

/// `account list` rendering options under `account.list.*`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountListingListConfig {
    #[serde(default)]
    pub table: AccountListingTableConfig,
}

/// Per-column color overrides for the `account list` table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountListingTableConfig {
    pub name_color: Option<Color>,
    pub backends_color: Option<Color>,
    pub default_color: Option<Color>,
}

/// `envelopes list` rendering options under `envelope.list.*`.
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

    /// Default `-s/--page-size` value for `envelopes list`. The CLI
    /// flag wins when passed; otherwise the merged account/global
    /// config wins; otherwise the hard fallback (25) is used.
    pub page_size: Option<u32>,

    /// Per-column color overrides + flag glyph customization for the
    /// rendered envelopes table. Keys mirror the v1.2.0 layout
    /// (`envelope.list.table.id-color`, `envelope.list.table.unseen-char`,
    /// etc.). Color values accept either a named [crossterm color]
    /// (`"red"`, `"dark-magenta"`, …) or an `{ Rgb = { r = .., g = ..,
    /// b = .. } }`/`{ AnsiValue = N }` table.
    ///
    /// [crossterm color]: https://docs.rs/crossterm/latest/crossterm/style/enum.Color.html
    #[serde(default)]
    pub table: EnvelopeListTableConfig,
}

/// Per-column color and flag glyph overrides for the envelopes table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeListTableConfig {
    /// Single character used in the FLAGS column for messages that
    /// lack `\Seen`. Defaults to `*` (v1.2.0 default).
    pub unseen_char: Option<char>,
    /// Single character used in the FLAGS column for messages with
    /// `\Answered`. Defaults to `R`.
    pub replied_char: Option<char>,
    /// Single character used in the FLAGS column for messages with
    /// `\Flagged`. Defaults to `!`.
    pub flagged_char: Option<char>,
    /// Single character used in the ATT column for messages with at
    /// least one attachment. Defaults to `@`.
    pub attachment_char: Option<char>,

    pub id_color: Option<Color>,
    pub flags_color: Option<Color>,
    pub att_color: Option<Color>,
    pub subject_color: Option<Color>,
    pub from_color: Option<Color>,
    pub to_color: Option<Color>,
    pub date_color: Option<Color>,
    pub size_color: Option<Color>,
}

/// Global / per-account table rendering quirks shared across every list
/// command (envelopes, mailboxes, attachments). The per-column color
/// blocks live under `*.list.table.*-color` (see [`EnvelopeListTableConfig`]
/// & co.).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TableConfig {
    /// Preset string: one char per table component (borders, corners,
    /// separators), a space meaning "don't draw this one". Defaults to
    /// `UTF8_FULL_CONDENSED`. See [`style_from_preset`] for the
    /// component order.
    ///
    /// [`style_from_preset`]: crate::shared::table::style_from_preset
    pub preset: Option<String>,
    /// Column-arrangement strategy. Defaults to `Dynamic`.
    pub arrangement: Option<TableArrangementConfig>,
}

/// Column-arrangement strategy for rendered tables.
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

/// Parses a backend `server` config string into a [`Url`], accepting
/// three forms: a full `scheme://host[:port][/path]` URL, a bare
/// authority `host:port`, or a bare `host`. The last two default to
/// `default_scheme` (the protocol's secure scheme).
///
/// A bare `host:port` must be detected by the absence of `://`: the
/// URL parser would otherwise read it as `scheme:path` (e.g.
/// `mail.example.com:993` parses as scheme `mail.example.com`), so any
/// string without an explicit `://` is treated as an authority. The
/// resulting scheme is validated against `allowed`.
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
    /// IMAP server address. Either a bare authority
    /// (`imap.example.com[:port]`, treated as `imaps://<authority>` by
    /// default), or a full URL with `imap://` (cleartext, with
    /// optional STARTTLS upgrade) or `imaps://` (implicit TLS) scheme
    /// used verbatim. Mirrors [`JmapConfig::server`].
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting. Only valid when the server resolves to `imap://`.
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: bool,
    /// ALPN protocol identifiers offered during the TLS handshake. Defaults to
    /// `["imap"]` (RFC 7595, IANA registry). Set to `[]` to skip ALPN
    /// negotiation entirely. Only relevant for the rustls provider;
    /// `native-tls` ignores ALPN.
    #[serde(
        default = "default_imap_alpn",
        skip_serializing_if = "is_default_imap_alpn"
    )]
    pub alpn: Vec<String>,
    /// Optional SASL credentials. When omitted, the connection skips
    /// authentication entirely (no `AUTHENTICATE` command is sent); to
    /// advertise the ANONYMOUS mechanism explicitly, set `sasl.anonymous = {}`.
    pub sasl: Option<SaslConfig>,
    /// RFC 4959 SASL-IR quirk. Left unset, follows the advertised `SASL-IR`
    /// capability; `false` waits for the server's continuation request rather
    /// than inlining credentials with `AUTHENTICATE`. Coremail (126.com,
    /// 163.com) advertises it falsely.
    #[serde(default, skip_serializing_if = "is_default")]
    pub sasl_ir: Option<bool>,
    /// RFC 2971 `ID` extension quirks. Some providers (notably mail.qq.com,
    /// fastmail) require an `ID` exchange straight after authentication; set
    /// `id.auto = true` to opt in.
    #[serde(default)]
    pub id: ImapIdConfig,
    /// RFC 5256 `SORT` extension config.
    #[serde(default)]
    pub sort: ImapSortConfig,
}

/// Per-account `imap.sort.*` options.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapSortConfig {
    /// Forces the SORT fallback on or off. `Some(true)` always sorts
    /// client-side via SEARCH + FETCH; `Some(false)` always issues a server
    /// `SORT`. Left unset, the fallback is enabled only when the server lacks
    /// the SORT capability.
    pub fallback: Option<bool>,
}

/// Per-account `imap.id.*` quirks.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdConfig {
    /// When `true`, the auth coroutine chains an `ID` round-trip
    /// after the tagged auth response. Default `false` skips ID
    /// entirely.
    #[serde(default, skip_serializing_if = "is_default")]
    pub auto: bool,

    /// Parameters sent with the auto-ID command. Empty (default)
    /// sends `ID NIL`. For each entry: `true` substitutes himalaya's
    /// canned value for the well-known keys (`name`, `version`,
    /// `vendor`, `support-url`) or `NIL` for unknown keys; `false`
    /// always sends `NIL`. Keys absent from this map are not
    /// transmitted.
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

/// Per-account `maildir.keywords.*` options, naming how a mailbox
/// encodes custom keywords. Both unset reads none.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirKeywordsConfig {
    /// Whether to resolve custom keywords through each mailbox's own
    /// `dovecot-keywords` file, which maps a lowercase info-section
    /// letter to a keyword. Default `false`, leaving those letters
    /// unread.
    #[serde(default, skip_serializing_if = "is_default")]
    pub dovecot: bool,
    /// The body header custom keywords are read from. Unset, the
    /// default, reads neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<MaildirKeywordHeaderConfig>,
}

/// Maildir configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirConfig {
    pub root: PathBuf,
    /// How custom (non-IANA) keywords are read, if at all.
    #[serde(default)]
    pub keywords: MaildirKeywordsConfig,
}

/// m2dir configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct M2dirConfig {
    pub root: PathBuf,
}

/// pimdir configuration: a local [pimdir](https://github.com/pimalaya/pimdir)
/// store (SQLite index + content-addressed blobs) used as an offline cache the
/// sync engine (Neverest) populates.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PimdirConfig {
    /// The store directory (holds `pimdir.db` and `objects/`).
    pub root: PathBuf,
    /// The account whose collections this client reads (pimdir SPEC §9.2), the
    /// sync engine's account name. Usually left unset: a store synced by one
    /// account is read as that one. Set it only for a store several accounts
    /// share, where guessing would show the wrong mailbox set.
    #[serde(default)]
    pub account: Option<String>,
    /// The namespace the sync binds this account's collections under, stripped
    /// off a collection id to name a mailbox: `imap/INBOX` reads as `INBOX`.
    /// Usually left unset: it is derived when every mail collection shares one
    /// prefix, which a single-source account always does. Set it only for a
    /// store whose mail collections span two namespaces.
    #[serde(default)]
    pub namespace: Option<String>,
}

/// SMTP configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SmtpConfig {
    /// SMTP server address. Either a bare authority
    /// (`smtp.example.com[:port]`, treated as `smtps://<authority>`
    /// by default), or a full URL with `smtp://` (cleartext, with
    /// optional STARTTLS upgrade) or `smtps://` (implicit TLS) scheme
    /// used verbatim. Mirrors [`JmapConfig::server`].
    pub server: String,

    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: bool,

    /// ALPN protocol identifiers offered during the TLS handshake.
    /// Defaults to `["smtp"]` (RFC 7595, IANA registry). Set to `[]`
    /// to skip ALPN negotiation entirely. Only relevant for the
    /// rustls provider; `native-tls` ignores ALPN.
    #[serde(
        default = "default_smtp_alpn",
        skip_serializing_if = "is_default_smtp_alpn"
    )]
    pub alpn: Vec<String>,

    /// Optional SASL credentials. See [`ImapConfig::sasl`].
    pub sasl: Option<SaslConfig>,
}

/// ManageSieve configuration.
#[allow(unused)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SieveConfig {
    /// ManageSieve server address. Either a bare authority
    /// (`sieve.example.com[:port]`, treated as `sieve://<authority>` by
    /// default), a full `sieve://` URL for STARTTLS or cleartext, a
    /// `sieves://` URL for implicit TLS, or a `unix://` socket.
    ///
    /// RFC 5804 registers one port, 4190, and reaches TLS through
    /// STARTTLS rather than through a second port, so a bare authority
    /// resolves to `sieve://` unlike the IMAP and SMTP ones. `sieves://`
    /// is for the deployments listening for a TLS handshake straight
    /// away, which the specification does not define.
    pub server: String,

    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,

    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting. Left unset, follows the scheme: on for `sieve://`,
    /// which is the only upgrade path RFC 5804 defines, off for
    /// `sieves://` and `unix://`. Setting it on a `sieves://` server is
    /// an error, TLS being already up.
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: Option<bool>,

    /// ALPN protocol identifiers offered during the TLS handshake.
    /// ManageSieve registers none, so the default is an empty list. Set
    /// it when a server wants a private identifier.
    #[serde(
        default = "default_sieve_alpn",
        skip_serializing_if = "is_default_sieve_alpn"
    )]
    pub alpn: Vec<String>,

    /// Whether a mechanism disclosing a reusable credential may run
    /// over a cleartext connection.
    ///
    /// `plain`, `login`, `oauthbearer` and `xoauth2` hand a passive
    /// observer something it can replay, so they are refused by default
    /// unless the connection is encrypted. Set this for a server
    /// reached over a trusted local link.
    #[serde(default, skip_serializing_if = "is_default")]
    pub allow_cleartext_auth: bool,

    /// Optional SASL credentials. See [`ImapConfig::sasl`].
    pub sasl: Option<SaslConfig>,
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

impl TlsConfig {
    /// Builds the runtime [`Tls`] handle the connect helpers expect.
    /// `alpn` is the protocol-level ALPN list (e.g. `["imap"]`,
    /// `["smtp"]`, `["http/1.1"]`); pass an empty vec to skip ALPN.
    /// The TOML schema never exposes `tls.rustls.alpn` directly: the
    /// per-protocol `*.alpn` field is folded in here.
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

/// SASL configuration.
///
/// Exactly one mechanism per `[*.sasl]` block. Each variant carries
/// only the bits its mechanism actually transmits; serde picks the
/// variant from the field name (`plain`, `login`, `anonymous`,
/// `oauthbearer`, `xoauth2`, `scram-sha-256`).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum SaslConfig {
    Anonymous(SaslAnonymousConfig),
    Login(SaslLoginConfig),
    Plain(SaslPlainConfig),
    Oauthbearer(SaslOauthbearerConfig),
    Xoauth2(SaslXoauth2Config),
    #[serde(rename = "scram-sha-256")]
    ScramSha256(SaslScramSha256Config),
}

/// SASL ANONYMOUS configuration <sup>[rfc4505]</sup>.
///
/// [rfc4505]: https://www.iana.org/go/rfc4505
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    pub message: Option<String>,
}

/// SASL LOGIN configuration <sup>[draft]</sup>.
///
/// [draft]: https://datatracker.ietf.org/doc/html/draft-murchison-sasl-login-00
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub password: Secret,
}

/// SASL PLAIN configuration <sup>[rfc4616]</sup>.
///
/// [rfc4616]: https://www.iana.org/go/rfc4616
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    pub authzid: Option<String>,
    #[serde(deserialize_with = "shell_expanded_string")]
    #[serde(alias = "username")]
    pub authcid: String,
    #[serde(alias = "password")]
    pub passwd: Secret,
}

/// SASL OAUTHBEARER configuration <sup>[rfc7628]</sup>.
///
/// The `host` and `port` echoed in the GS2 header are derived from
/// the live IMAP/SMTP server URL at connect time, so they aren't part
/// of the user-facing config.
///
/// [rfc7628]: https://www.iana.org/go/rfc7628
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslOauthbearerConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub token: Secret,
}

/// SASL XOAUTH2 configuration. Google's pre-standard OAuth 2.0 SASL
/// scheme; see <https://developers.google.com/gmail/imap/xoauth2-protocol>.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslXoauth2Config {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub token: Secret,
}

/// SASL SCRAM-SHA-256 configuration <sup>[rfc7677]</sup>.
///
/// [rfc7677]: https://www.iana.org/go/rfc7677
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslScramSha256Config {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub password: Secret,
}

impl SaslConfig {
    /// Resolves the SASL config into a runtime [`Sasl`]. `host` and
    /// `port` come from the live server URL; they are only used by
    /// OAUTHBEARER (echoed in the GS2 header) and ignored by every
    /// other mechanism.
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
            // NOTE: an empty nonce means "draw one for me": the client
            // fills it before the exchange, an I/O-free coroutine having
            // no way to generate randomness itself.
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

    /// ALPN protocol identifiers offered during the TLS handshake.
    /// Defaults to `["http/1.1"]` (JMAP rides on HTTP/1.1). Set to
    /// `[]` to skip ALPN negotiation entirely. Only relevant for the
    /// rustls provider; `native-tls` ignores ALPN.
    #[serde(
        default = "default_jmap_alpn",
        skip_serializing_if = "is_default_jmap_alpn"
    )]
    pub alpn: Vec<String>,

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
    /// Full raw Authorization header value, sent verbatim.
    Header(Secret),
    /// Bearer token (OAuth 2.0 access token).
    Bearer { token: Secret },
    /// HTTP Basic authentication (username + password).
    Basic {
        #[serde(deserialize_with = "shell_expanded_string")]
        username: String,
        password: Secret,
    },
}

/// Gmail REST API configuration.
///
/// Gmail has no per-account server address: the client always talks to
/// `https://gmail.googleapis.com`. Only the mailbox owner, TLS and the
/// OAuth 2.0 credential are configurable.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GmailConfig {
    /// Gmail user id (the mailbox owner). Defaults to `me`, the
    /// authenticated user.
    #[serde(default = "default_gmail_user_id")]
    pub user_id: String,

    /// TLS configuration.
    #[serde(default)]
    pub tls: TlsConfig,

    /// ALPN protocol identifiers offered during the TLS handshake.
    /// Defaults to `["http/1.1"]` (the Gmail REST API rides on
    /// HTTP/1.1). Set to `[]` to skip ALPN negotiation entirely. Only
    /// relevant for the rustls provider; `native-tls` ignores ALPN.
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
/// Gmail only accepts OAuth 2.0 bearer tokens; supply a short-lived
/// access token (e.g. minted by an external helper such as `ortie`).
/// Token refresh is the caller's responsibility.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GmailAuthConfig {
    /// OAuth 2.0 bearer access token; sent as `Bearer <token>`. It is
    /// the only authorization Gmail's REST API accepts.
    pub token: Secret,
}

/// Microsoft Graph API configuration.
///
/// Graph has no per-account server address: the client always talks to
/// `https://graph.microsoft.com`. Only the mailbox owner, TLS and the
/// OAuth 2.0 credential are configurable.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MsgraphConfig {
    /// Graph user id (the mailbox owner). Defaults to `me`, the
    /// authenticated user; set it to a user id or principal name to
    /// target another mailbox.
    #[serde(default = "default_msgraph_user_id")]
    pub user_id: String,

    /// TLS configuration.
    #[serde(default)]
    pub tls: TlsConfig,

    /// ALPN protocol identifiers offered during the TLS handshake.
    /// Defaults to `["http/1.1"]` (the Graph API rides on HTTP/1.1). Set
    /// to `[]` to skip ALPN negotiation entirely. Only relevant for the
    /// rustls provider; `native-tls` ignores ALPN.
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
/// Graph only accepts OAuth 2.0 bearer tokens; supply a short-lived
/// access token (e.g. minted by an external helper such as `ortie`).
/// Token refresh is the caller's responsibility.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MsgraphAuthConfig {
    /// OAuth 2.0 bearer access token; sent as `Bearer <token>`. It is
    /// the only authorization the Graph API accepts.
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
        // No explicit port: the protocol's default (e.g. 993) is
        // applied by the backend client, not by this parser.
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
    /// file backs both binaries, so a config it wrote must reach the
    /// same fields the composers read. `signature` and `signature-delim`
    /// need no alias, both binaries having always spelled them alike.
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
