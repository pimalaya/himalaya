//! Merged runtime account — the DTO every command consumes.
//!
//! Built by the dispatch layer (`crate::cli`) in this order:
//!
//! 1. [`Account::default`] (all fields `None` / empty).
//! 2. Fold the global [`Config`] via `Account::from(config)`.
//! 3. Fold the selected `[accounts.<name>]` via [`Account::merge`]
//!    with `Account::from(account_config)`.
//!
//! Defaults are applied at consumption time by the `*` accessor
//! methods, not baked in during merge — keeping `Option<T>` fields
//! lets layers compose cleanly.

use std::{collections::HashMap, env::temp_dir, path::PathBuf};

use comfy_table::{Color as TableColor, ContentArrangement, presets};
use crossterm::style::Color;
use dirs::download_dir;

use crate::config::{
    AccountConfig, AttachmentListTableConfig, Config, EnvelopeListTableConfig,
    MailboxListTableConfig, TableArrangementConfig,
};

const DEFAULT_DATETIME_FMT: &str = "%F %R%:z";
const DEFAULT_MAILBOX_ALIAS: &str = "inbox";
const DEFAULT_ENVELOPES_LIST_PAGE_SIZE: u32 = 25;

const DEFAULT_UNSEEN_CHAR: char = '*';
const DEFAULT_REPLIED_CHAR: char = 'R';
const DEFAULT_FLAGGED_CHAR: char = '!';
const DEFAULT_ATTACHMENT_CHAR: char = '@';

/// Merged runtime account settings consumed by every command.
#[derive(Debug, Default)]
pub struct Account {
    pub downloads_dir: Option<PathBuf>,
    pub table_preset: Option<String>,
    pub table_arrangement: Option<TableArrangementConfig>,

    pub datetime_fmt: Option<String>,
    pub datetime_local_tz: Option<bool>,
    pub envelopes_list_page_size: Option<u32>,

    /// Per-column color + flag glyph overrides for `envelopes list`.
    pub envelopes_list_table: EnvelopeListTableConfig,
    /// Per-column color overrides for `mailboxes list`.
    pub mailboxes_list_table: MailboxListTableConfig,
    /// Per-column color overrides for `attachments list`.
    pub attachments_list_table: AttachmentListTableConfig,

    /// Mailbox aliases, keys lowercased. Populated from
    /// `mailbox.alias` at the global and account levels; account
    /// entries overwrite same-named global entries.
    pub mailbox_alias: HashMap<String, String>,
}

impl Account {
    /// Folds `other`'s set fields on top of `self`. Each `Option`
    /// field is taken from `other` when `Some`, otherwise from
    /// `self`.
    pub fn merge(self, other: Self) -> Self {
        let mut mailbox_alias = self.mailbox_alias;
        mailbox_alias.extend(other.mailbox_alias);

        Self {
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

    /// Effective downloads directory. Tries the merged
    /// `downloads_dir` (shell-expanded), then the system default
    /// downloads dir, then the temp dir.
    pub fn downloads_dir(&self) -> PathBuf {
        self.downloads_dir
            .as_ref()
            .and_then(|dir| dir.to_str())
            .and_then(|dir| shellexpand::full(dir).ok())
            .map(|dir| PathBuf::from(dir.to_string()))
            .or_else(download_dir)
            .unwrap_or_else(temp_dir)
    }

    /// Effective `comfy_table` preset string. Defaults to
    /// `UTF8_FULL_CONDENSED`.
    pub fn table_preset(&self) -> &str {
        self.table_preset
            .as_deref()
            .unwrap_or(presets::UTF8_FULL_CONDENSED)
    }

    /// Effective `comfy_table` content arrangement. Defaults to
    /// `Dynamic`.
    pub fn table_arrangement(&self) -> ContentArrangement {
        self.table_arrangement
            .clone()
            .unwrap_or(TableArrangementConfig::Dynamic)
            .into()
    }

    /// Effective `chrono` `strftime` format for envelope DATE
    /// columns. Defaults to `%F %R%:z`.
    pub fn datetime_fmt(&self) -> &str {
        self.datetime_fmt.as_deref().unwrap_or(DEFAULT_DATETIME_FMT)
    }

    /// Whether to convert envelope `Date:` headers to the system
    /// local timezone. Defaults to `false`.
    pub fn datetime_local_tz(&self) -> bool {
        self.datetime_local_tz.unwrap_or(false)
    }

    /// Effective default page size for `envelopes list` when the
    /// `-s/--page-size` flag is not passed. Defaults to 25.
    pub fn envelopes_list_page_size(&self) -> u32 {
        self.envelopes_list_page_size
            .unwrap_or(DEFAULT_ENVELOPES_LIST_PAGE_SIZE)
    }

    /// Resolves `name` through the alias map.
    ///
    /// Lookup is case-insensitive on the alias name. When `name`
    /// matches an alias, the stored id is returned verbatim;
    /// otherwise `name` itself is returned, allowing callers to pass
    /// either an alias or a raw backend id transparently.
    pub fn resolve_mailbox<'a>(&'a self, name: &'a str) -> &'a str {
        let key = name.to_lowercase();
        self.mailbox_alias
            .get(&key)
            .map(String::as_str)
            .unwrap_or(name)
    }

    /// Resolved id of the implicit default mailbox.
    ///
    /// Returns the id mapped to the `inbox` alias (case-insensitive),
    /// or `None` when no such alias is configured. Used by shared
    /// commands when `-m/--mailbox` is omitted.
    pub fn default_mailbox(&self) -> Option<&str> {
        self.mailbox_alias
            .get(DEFAULT_MAILBOX_ALIAS)
            .map(String::as_str)
    }

    // ── envelopes list — flag glyphs ─────────────────────────────────────

    pub fn envelopes_list_table_unseen_char(&self) -> char {
        self.envelopes_list_table
            .unseen_char
            .unwrap_or(DEFAULT_UNSEEN_CHAR)
    }
    pub fn envelopes_list_table_replied_char(&self) -> char {
        self.envelopes_list_table
            .replied_char
            .unwrap_or(DEFAULT_REPLIED_CHAR)
    }
    pub fn envelopes_list_table_flagged_char(&self) -> char {
        self.envelopes_list_table
            .flagged_char
            .unwrap_or(DEFAULT_FLAGGED_CHAR)
    }
    pub fn envelopes_list_table_attachment_char(&self) -> char {
        self.envelopes_list_table
            .attachment_char
            .unwrap_or(DEFAULT_ATTACHMENT_CHAR)
    }

    // ── envelopes list — column colors ───────────────────────────────────
    //
    // Defaults mirror pimalaya-tui v1.2.0
    // (`ListEnvelopesTableConfig::{id,flags,subject,sender,date}_color`).
    pub fn envelopes_list_table_id_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.id_color, Color::Red)
    }
    pub fn envelopes_list_table_flags_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.flags_color, Color::Reset)
    }
    pub fn envelopes_list_table_att_color(&self) -> TableColor {
        // No v1 precedent for a standalone ATT column (v1 embedded the
        // attachment glyph inside FLAGS); leave it neutral.
        map_color_or(self.envelopes_list_table.att_color, Color::Reset)
    }
    pub fn envelopes_list_table_subject_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.subject_color, Color::Green)
    }
    pub fn envelopes_list_table_from_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.from_color, Color::Blue)
    }
    pub fn envelopes_list_table_to_color(&self) -> TableColor {
        // `to` mirrors `from`'s default; v1 didn't surface a TO column.
        map_color_or(self.envelopes_list_table.to_color, Color::Blue)
    }
    pub fn envelopes_list_table_date_color(&self) -> TableColor {
        map_color_or(self.envelopes_list_table.date_color, Color::DarkYellow)
    }
    pub fn envelopes_list_table_size_color(&self) -> TableColor {
        // New in v2, no v1 precedent.
        map_color_or(self.envelopes_list_table.size_color, Color::Reset)
    }

    // ── mailboxes list — column colors ───────────────────────────────────
    //
    // `name` matches the v1 `folder.list.table.name-color` default
    // (`pimalaya-tui::ListFoldersTableConfig::name_color`); the other
    // columns are new in v2.
    pub fn mailboxes_list_table_id_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.id_color, Color::Reset)
    }
    pub fn mailboxes_list_table_name_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.name_color, Color::Blue)
    }
    pub fn mailboxes_list_table_total_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.total_color, Color::Reset)
    }
    pub fn mailboxes_list_table_unread_color(&self) -> TableColor {
        map_color_or(self.mailboxes_list_table.unread_color, Color::Reset)
    }

    // ── attachments list — column colors ─────────────────────────────────
    //
    // No v1 precedent; defaults left neutral.
    pub fn attachments_list_table_id_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.id_color, Color::Reset)
    }
    pub fn attachments_list_table_filename_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.filename_color, Color::Reset)
    }
    pub fn attachments_list_table_type_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.type_color, Color::Reset)
    }
    pub fn attachments_list_table_size_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.size_color, Color::Reset)
    }
    pub fn attachments_list_table_inline_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.inline_color, Color::Reset)
    }
    pub fn attachments_list_table_path_color(&self) -> TableColor {
        map_color_or(self.attachments_list_table.path_color, Color::Reset)
    }
}

/// Maps a [`crossterm::style::Color`] (deserialized from TOML) into a
/// [`comfy_table::Color`] used by the renderers, substituting
/// `fallback` when the TOML field is unset.
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

/// Lowercases every key of `aliases`, leaving values untouched. Used at
/// the [`Config`] / [`AccountConfig`] -> [`Account`] boundary so that
/// the merge and the [`Account::resolve_mailbox`] lookup can both rely
/// on already-normalized keys.
fn lowercase_alias_keys(aliases: HashMap<String, String>) -> HashMap<String, String> {
    aliases
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect()
}

impl From<Config> for Account {
    fn from(config: Config) -> Self {
        Self {
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
    fn merge_lets_account_override_global_alias() {
        let global = account_with_aliases(&[("inbox", "INBOX"), ("sent", "Sent")]);
        let per_account = account_with_aliases(&[("inbox", "Mailbox/0")]);
        let merged = global.merge(per_account);
        assert_eq!(merged.resolve_mailbox("inbox"), "Mailbox/0");
        assert_eq!(merged.resolve_mailbox("sent"), "Sent");
    }
}
