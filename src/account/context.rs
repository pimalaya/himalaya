//! # Account context
//!
//! The merged runtime account every command consumes, folded by the
//! dispatch layer from the global [`Config`] then from the selected
//! `[accounts.<name>]` block.
//!
//! Defaults are applied by the accessors at consumption time rather than
//! baked in during the merge, so every field stays an `Option` and the two
//! layers compose.

use std::{
    collections::HashMap,
    env::temp_dir,
    path::{Path, PathBuf},
};

use comfy_table::{Color as TableColor, ContentArrangement};
use crossterm::style::Color;
use dirs::download_dir;

use crate::{
    config::{
        AccountConfig, AttachmentListTableConfig, Config, EnvelopeListTableConfig,
        MailboxListTableConfig, TableArrangementConfig,
    },
    shared::table::DEFAULT_PRESET,
};

/// chrono `strftime` format of the envelope DATE column.
const DEFAULT_DATETIME_FMT: &str = "%F %R%:z";
/// Alias naming the mailbox a command omitting `-m/--mailbox` runs against.
const DEFAULT_MAILBOX_ALIAS: &str = "inbox";
/// Page size of `envelope list` when nothing names one.
const DEFAULT_ENVELOPES_LIST_PAGE_SIZE: u32 = 25;
/// RFC 3676 section 4.3 signature separator.
const DEFAULT_SIGNATURE_DELIM: &str = "-- \n";
/// FLAGS glyph of a message lacking `\Seen`.
const DEFAULT_UNSEEN_CHAR: char = '*';
/// FLAGS glyph of a message carrying `\Answered`.
const DEFAULT_REPLIED_CHAR: char = 'R';
/// FLAGS glyph of a message carrying `\Flagged`.
const DEFAULT_FLAGGED_CHAR: char = '!';
/// ATT glyph of a message carrying an attachment.
const DEFAULT_ATTACHMENT_CHAR: char = '@';

/// Merged runtime account settings consumed by every command.
#[derive(Debug, Default)]
pub struct Account {
    /// Address the account sends as.
    pub email: Option<String>,
    /// Name that address carries.
    pub display_name: Option<String>,
    /// Signature appended to a composed message.
    pub signature: Option<String>,
    /// Separator written before the signature.
    pub signature_delim: Option<String>,
    /// Directory attachments are downloaded to.
    pub downloads_dir: Option<PathBuf>,
    /// `comfy_table` preset string every listing renders with.
    pub table_preset: Option<String>,
    /// `comfy_table` column arrangement every listing renders with.
    pub table_arrangement: Option<TableArrangementConfig>,
    /// chrono `strftime` format of the envelope DATE column.
    pub datetime_fmt: Option<String>,
    /// Whether an envelope date is converted to the local timezone.
    pub datetime_local_tz: Option<bool>,
    /// Page size of `envelope list` when `-s/--page-size` is not passed.
    pub envelopes_list_page_size: Option<u32>,
    /// Per-column colors and flag glyphs of `envelope list`.
    pub envelopes_list_table: EnvelopeListTableConfig,
    /// Per-column colors of `mailbox list`.
    pub mailboxes_list_table: MailboxListTableConfig,
    /// Per-column colors of `attachment list`.
    pub attachments_list_table: AttachmentListTableConfig,
    /// Mailbox aliases, keys lowercased, an account entry overwriting the
    /// global one of the same name.
    pub mailbox_alias: HashMap<String, String>,
}

impl Account {
    /// Folds the fields `other` sets on top of `self`.
    pub fn merge(self, other: Self) -> Self {
        let mut mailbox_alias = self.mailbox_alias;
        mailbox_alias.extend(other.mailbox_alias);

        Self {
            email: other.email.or(self.email),
            display_name: other.display_name.or(self.display_name),
            signature: other.signature.or(self.signature),
            signature_delim: other.signature_delim.or(self.signature_delim),

            downloads_dir: other.downloads_dir.or(self.downloads_dir),
            table_preset: other.table_preset.or(self.table_preset),
            table_arrangement: other.table_arrangement.or(self.table_arrangement),

            datetime_fmt: other.datetime_fmt.or(self.datetime_fmt),
            datetime_local_tz: other.datetime_local_tz.or(self.datetime_local_tz),
            envelopes_list_page_size: other
                .envelopes_list_page_size
                .or(self.envelopes_list_page_size),

            envelopes_list_table: merge_envelope_table(
                self.envelopes_list_table,
                other.envelopes_list_table,
            ),
            mailboxes_list_table: merge_mailbox_table(
                self.mailboxes_list_table,
                other.mailboxes_list_table,
            ),
            attachments_list_table: merge_attachment_table(
                self.attachments_list_table,
                other.attachments_list_table,
            ),

            mailbox_alias,
        }
    }

    /// Resolves the `From` header into an address and its name.
    ///
    /// The two are kept apart so the MIME builder does the quoting.
    /// `--from` wins whole, so a configured name is never grafted onto an
    /// address the user spelled out, and a `None` address leaves the
    /// header out.
    pub fn resolve_from<'a>(&'a self, over: Option<&'a str>) -> (Option<&'a str>, Option<&'a str>) {
        match over {
            Some(address) => (Some(address), None),
            None => (self.email.as_deref(), self.display_name.as_deref()),
        }
    }

    /// Resolves the signature a composed message ends with.
    ///
    /// `--signature` wins, and `--signature-file` answers in the builder
    /// instead, which is why the configured signature stands down rather
    /// than shadowing it. With neither, the merged account answers.
    pub fn resolve_signature<'a>(
        &'a self,
        over: Option<&'a str>,
        file: Option<&Path>,
    ) -> Option<&'a str> {
        match (over, file) {
            (Some(signature), _) => Some(signature),
            (None, Some(_)) => None,
            (None, None) => self.signature.as_deref(),
        }
    }

    /// Separator written before the signature, verbatim, defaulting to
    /// the RFC 3676 section 4.3 `"-- \n"`.
    pub fn signature_delim(&self) -> &str {
        self.signature_delim
            .as_deref()
            .unwrap_or(DEFAULT_SIGNATURE_DELIM)
    }

    /// Directory attachments are downloaded to, falling back to the
    /// system one then to the temporary directory.
    pub fn downloads_dir(&self) -> PathBuf {
        self.downloads_dir
            .as_ref()
            .and_then(|dir| dir.to_str())
            .and_then(|dir| shellexpand::full(dir).ok())
            .map(|dir| PathBuf::from(dir.to_string()))
            .or_else(download_dir)
            .unwrap_or_else(temp_dir)
    }

    /// `comfy_table` preset string, defaulting to `UTF8_FULL_CONDENSED`.
    pub fn table_preset(&self) -> &str {
        self.table_preset.as_deref().unwrap_or(DEFAULT_PRESET)
    }

    /// `comfy_table` content arrangement, defaulting to `Dynamic`.
    pub fn table_arrangement(&self) -> ContentArrangement {
        self.table_arrangement
            .clone()
            .unwrap_or(TableArrangementConfig::Dynamic)
            .into()
    }

    /// chrono `strftime` format of the DATE column, defaulting to
    /// `%F %R%:z`.
    pub fn datetime_fmt(&self) -> &str {
        self.datetime_fmt.as_deref().unwrap_or(DEFAULT_DATETIME_FMT)
    }

    /// Whether a `Date:` header is converted to the local timezone,
    /// defaulting to `false`.
    pub fn datetime_local_tz(&self) -> bool {
        self.datetime_local_tz.unwrap_or(false)
    }

    /// Page size of `envelope list` when `-s/--page-size` is not passed,
    /// defaulting to 25.
    pub fn envelopes_list_page_size(&self) -> u32 {
        self.envelopes_list_page_size
            .unwrap_or(DEFAULT_ENVELOPES_LIST_PAGE_SIZE)
    }

    /// Resolves `name` through the alias map, case-insensitively.
    ///
    /// An unmatched name comes back verbatim, so a caller passes either an
    /// alias or a raw backend id without knowing which.
    pub fn resolve_mailbox<'a>(&'a self, name: &'a str) -> &'a str {
        let key = name.to_lowercase();
        self.mailbox_alias
            .get(&key)
            .map(String::as_str)
            .unwrap_or(name)
    }

    /// Id the `inbox` alias maps to, which is the mailbox a shared command
    /// omitting `-m/--mailbox` runs against.
    pub fn default_mailbox(&self) -> Option<&str> {
        self.mailbox_alias
            .get(DEFAULT_MAILBOX_ALIAS)
            .map(String::as_str)
    }

    /// FLAGS glyph of a message lacking `\Seen`, defaulting to `*`.
    pub fn envelopes_list_table_unseen_char(&self) -> char {
        self.envelopes_list_table
            .unseen_char
            .unwrap_or(DEFAULT_UNSEEN_CHAR)
    }

    /// FLAGS glyph of a message carrying `\Answered`, defaulting to `R`.
    pub fn envelopes_list_table_replied_char(&self) -> char {
        self.envelopes_list_table
            .replied_char
            .unwrap_or(DEFAULT_REPLIED_CHAR)
    }

    /// FLAGS glyph of a message carrying `\Flagged`, defaulting to `!`.
    pub fn envelopes_list_table_flagged_char(&self) -> char {
        self.envelopes_list_table
            .flagged_char
            .unwrap_or(DEFAULT_FLAGGED_CHAR)
    }

    /// ATT glyph of a message carrying an attachment, defaulting to `@`.
    pub fn envelopes_list_table_attachment_char(&self) -> char {
        self.envelopes_list_table
            .attachment_char
            .unwrap_or(DEFAULT_ATTACHMENT_CHAR)
    }

    /// Color of the ID column, defaulting to the v1.2.0 red.
    pub fn envelopes_list_table_id_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.id_color, Color::Red)
    }

    /// Color of the FLAGS column, defaulting to the v1.2.0 neutral.
    pub fn envelopes_list_table_flags_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.flags_color, Color::Reset)
    }

    /// Color of the ATT column, neutral for want of a v1.2.0 precedent:
    /// the attachment glyph then lived inside FLAGS.
    pub fn envelopes_list_table_att_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.att_color, Color::Reset)
    }

    /// Color of the SUBJECT column, defaulting to the v1.2.0 green.
    pub fn envelopes_list_table_subject_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.subject_color, Color::Green)
    }

    /// Color of the FROM column, defaulting to the v1.2.0 blue.
    pub fn envelopes_list_table_from_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.from_color, Color::Blue)
    }

    /// Color of the TO column, mirroring FROM for want of a v1.2.0
    /// precedent.
    pub fn envelopes_list_table_to_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.to_color, Color::Blue)
    }

    /// Color of the DATE column, defaulting to the v1.2.0 dark yellow.
    pub fn envelopes_list_table_date_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.date_color, Color::DarkYellow)
    }

    /// Color of the SIZE column, neutral for want of a v1.2.0 precedent.
    pub fn envelopes_list_table_size_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.size_color, Color::Reset)
    }

    /// Color of the ID column, neutral for want of a v1.2.0 precedent.
    pub fn mailboxes_list_table_id_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.id_color, Color::Reset)
    }

    /// Color of the NAME column, defaulting to the v1.2.0 blue.
    pub fn mailboxes_list_table_name_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.name_color, Color::Blue)
    }

    /// Color of the TOTAL column, neutral for want of a v1.2.0 precedent.
    pub fn mailboxes_list_table_total_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.total_color, Color::Reset)
    }

    /// Color of the UNREAD column, neutral for want of a v1.2.0 precedent.
    pub fn mailboxes_list_table_unread_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.unread_color, Color::Reset)
    }

    /// Color of the ID column, neutral for want of a v1.2.0 precedent.
    pub fn attachments_list_table_id_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.id_color, Color::Reset)
    }

    /// Color of the FILENAME column, neutral like the whole listing.
    pub fn attachments_list_table_filename_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.filename_color, Color::Reset)
    }

    /// Color of the TYPE column, neutral like the whole listing.
    pub fn attachments_list_table_type_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.type_color, Color::Reset)
    }

    /// Color of the SIZE column, neutral like the whole listing.
    pub fn attachments_list_table_size_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.size_color, Color::Reset)
    }

    /// Color of the INLINE column, neutral like the whole listing.
    pub fn attachments_list_table_inline_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.inline_color, Color::Reset)
    }

    /// Color of the PATH column, neutral like the whole listing.
    pub fn attachments_list_table_path_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.path_color, Color::Reset)
    }
}

/// Maps a TOML crossterm color into the comfy-table one the renderers
/// take, substituting `fallback` when the field is unset.
pub(crate) fn map_color_or(color: Option<Color>, fallback: Color) -> TableColor {
    match color.unwrap_or(fallback) {
        Color::Reset => TableColor::Reset,
        Color::Black => TableColor::Black,
        Color::DarkGrey => TableColor::DarkGrey,
        Color::Red => TableColor::Red,
        Color::DarkRed => TableColor::DarkRed,
        Color::Green => TableColor::Green,
        Color::DarkGreen => TableColor::DarkGreen,
        Color::Yellow => TableColor::Yellow,
        Color::DarkYellow => TableColor::DarkYellow,
        Color::Blue => TableColor::Blue,
        Color::DarkBlue => TableColor::DarkBlue,
        Color::Magenta => TableColor::Magenta,
        Color::DarkMagenta => TableColor::DarkMagenta,
        Color::Cyan => TableColor::Cyan,
        Color::DarkCyan => TableColor::DarkCyan,
        Color::White => TableColor::White,
        Color::Grey => TableColor::Grey,
        Color::Rgb { r, g, b } => TableColor::Rgb { r, g, b },
        Color::AnsiValue(n) => TableColor::AnsiValue(n),
    }
}

fn merge_envelope_table(
    base: EnvelopeListTableConfig,
    over: EnvelopeListTableConfig,
) -> EnvelopeListTableConfig {
    EnvelopeListTableConfig {
        unseen_char: over.unseen_char.or(base.unseen_char),
        replied_char: over.replied_char.or(base.replied_char),
        flagged_char: over.flagged_char.or(base.flagged_char),
        attachment_char: over.attachment_char.or(base.attachment_char),
        id_color: over.id_color.or(base.id_color),
        flags_color: over.flags_color.or(base.flags_color),
        att_color: over.att_color.or(base.att_color),
        subject_color: over.subject_color.or(base.subject_color),
        from_color: over.from_color.or(base.from_color),
        to_color: over.to_color.or(base.to_color),
        date_color: over.date_color.or(base.date_color),
        size_color: over.size_color.or(base.size_color),
    }
}

fn merge_mailbox_table(
    base: MailboxListTableConfig,
    over: MailboxListTableConfig,
) -> MailboxListTableConfig {
    MailboxListTableConfig {
        id_color: over.id_color.or(base.id_color),
        name_color: over.name_color.or(base.name_color),
        total_color: over.total_color.or(base.total_color),
        unread_color: over.unread_color.or(base.unread_color),
    }
}

fn merge_attachment_table(
    base: AttachmentListTableConfig,
    over: AttachmentListTableConfig,
) -> AttachmentListTableConfig {
    AttachmentListTableConfig {
        id_color: over.id_color.or(base.id_color),
        filename_color: over.filename_color.or(base.filename_color),
        type_color: over.type_color.or(base.type_color),
        size_color: over.size_color.or(base.size_color),
        inline_color: over.inline_color.or(base.inline_color),
        path_color: over.path_color.or(base.path_color),
    }
}

/// Lowercases every alias key, leaving the values untouched.
///
/// Run at the config boundary, so both the merge and
/// [`Account::resolve_mailbox`] read already-normalized keys.
fn lowercase_alias_keys(aliases: HashMap<String, String>) -> HashMap<String, String> {
    aliases
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect()
}

impl From<Config> for Account {
    fn from(config: Config) -> Self {
        Self {
            // NOTE: the address is per-account by nature, so only the
            // name it carries has a global default.
            email: None,
            display_name: config.display_name,
            signature: config.signature,
            signature_delim: config.signature_delim,

            downloads_dir: config.downloads_dir,
            table_preset: config.table.preset,
            table_arrangement: config.table.arrangement,

            datetime_fmt: config.envelope.list.datetime_fmt,
            datetime_local_tz: config.envelope.list.datetime_local_tz,
            envelopes_list_page_size: config.envelope.list.page_size,

            envelopes_list_table: config.envelope.list.table,
            mailboxes_list_table: config.mailbox.list.table,
            attachments_list_table: config.attachment.list.table,

            mailbox_alias: lowercase_alias_keys(config.mailbox.aliases),
        }
    }
}

impl From<AccountConfig> for Account {
    fn from(config: AccountConfig) -> Self {
        Self {
            email: config.email,
            display_name: config.display_name,
            signature: config.signature,
            signature_delim: config.signature_delim,

            downloads_dir: config.downloads_dir,
            table_preset: config.table.preset,
            table_arrangement: config.table.arrangement,

            datetime_fmt: config.envelope.list.datetime_fmt,
            datetime_local_tz: config.envelope.list.datetime_local_tz,
            envelopes_list_page_size: config.envelope.list.page_size,

            envelopes_list_table: config.envelope.list.table,
            mailboxes_list_table: config.mailbox.list.table,
            attachments_list_table: config.attachment.list.table,

            mailbox_alias: lowercase_alias_keys(config.mailbox.aliases),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MailboxConfig;

    fn account_with_aliases(pairs: &[(&str, &str)]) -> Account {
        let aliases = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        let config = Config {
            mailbox: MailboxConfig {
                aliases,
                ..MailboxConfig::default()
            },
            ..Config::default()
        };
        Account::from(config)
    }

    #[test]
    fn resolve_mailbox_returns_alias_target() {
        let account = account_with_aliases(&[("inbox", "INBOX")]);
        assert_eq!(account.resolve_mailbox("inbox"), "INBOX");
    }

    #[test]
    fn resolve_mailbox_lookup_is_case_insensitive() {
        let account = account_with_aliases(&[("inbox", "INBOX")]);
        assert_eq!(account.resolve_mailbox("INBOX"), "INBOX");
        assert_eq!(account.resolve_mailbox("Inbox"), "INBOX");
        assert_eq!(account.resolve_mailbox("iNbOx"), "INBOX");
    }

    #[test]
    fn resolve_mailbox_normalizes_keys_stored_with_any_case() {
        let account = account_with_aliases(&[("INBOX", "raw-id")]);
        assert_eq!(account.resolve_mailbox("inbox"), "raw-id");
    }

    #[test]
    fn resolve_mailbox_preserves_id_case() {
        let account = account_with_aliases(&[("sent", "Sent Items")]);
        assert_eq!(account.resolve_mailbox("SENT"), "Sent Items");
    }

    #[test]
    fn resolve_mailbox_falls_back_to_input_when_no_alias() {
        let account = account_with_aliases(&[]);
        assert_eq!(account.resolve_mailbox("INBOX"), "INBOX");
    }

    #[test]
    fn default_mailbox_returns_inbox_alias() {
        let account = account_with_aliases(&[("inbox", "raw-id")]);
        assert_eq!(account.default_mailbox(), Some("raw-id"));
    }

    #[test]
    fn default_mailbox_is_none_without_inbox_alias() {
        let account = account_with_aliases(&[("sent", "Sent Items")]);
        assert_eq!(account.default_mailbox(), None);
    }

    #[test]
    fn resolve_from_takes_the_global_name_and_the_account_address() {
        let global = Account::from(Config {
            display_name: Some("Alice".to_string()),
            ..Config::default()
        });
        let per_account = Account::from(AccountConfig {
            email: Some("alice@example.org".to_string()),
            ..AccountConfig::default()
        });
        let account = global.merge(per_account);

        assert_eq!(
            account.resolve_from(None),
            (Some("alice@example.org"), Some("Alice")),
        );

        // NOTE: an address the user spelled out is theirs whole, so the
        // configured name is not grafted onto it.
        assert_eq!(
            account.resolve_from(Some("alias@example.org")),
            (Some("alias@example.org"), None),
        );
    }

    #[test]
    fn resolve_signature_stands_down_for_either_flag() {
        let global = Account::from(Config {
            signature: Some("Alice".to_string()),
            ..Config::default()
        });
        let per_account = Account::from(AccountConfig {
            signature_delim: Some("~~~\n".to_string()),
            ..AccountConfig::default()
        });
        let account = global.merge(per_account);

        assert_eq!(account.resolve_signature(None, None), Some("Alice"));
        assert_eq!(account.signature_delim(), "~~~\n");

        // NOTE: `--signature-file` names the file the builder reads, so
        // the configured signature stands down rather than shadowing it.
        assert_eq!(
            account.resolve_signature(Some("Alias"), None),
            Some("Alias"),
        );
        assert_eq!(
            account.resolve_signature(None, Some(Path::new("/tmp/sig"))),
            None,
        );
    }

    #[test]
    fn signature_delim_defaults_to_the_rfc_separator() {
        assert_eq!(Account::default().signature_delim(), "-- \n");
    }

    #[test]
    fn merge_lets_account_override_global_alias() {
        let global = account_with_aliases(&[("inbox", "INBOX"), ("sent", "Sent")]);
        let per_account = account_with_aliases(&[("inbox", "Mailbox/0")]);
        let merged = global.merge(per_account);
        assert_eq!(merged.resolve_mailbox("inbox"), "Mailbox/0");
        assert_eq!(merged.resolve_mailbox("sent"), "Sent");
    }
}
