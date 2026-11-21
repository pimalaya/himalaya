//! # Envelope configuration
//!
//! The `envelope` block, global or per account.

use serde::Deserialize;

/// Envelope-level rendering options.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeConfig {
    /// `envelope list` rendering options.
    #[serde(default)]
    pub list: EnvelopeListConfig,
}

/// `envelope list` rendering options under `envelope.list.*`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvelopeListConfig {
    /// chrono `strftime` format of an envelope date, defaulting to
    /// `"%F %R"`.
    pub datetime_fmt: Option<String>,
    /// Whether the `Date:` offset is converted to the local timezone
    /// (default `true`); `false` keeps the sender's offset.
    pub datetime_local_tz: Option<bool>,
    /// Default `-s/--page-size`, the flag winning when passed and 25
    /// being the hard fallback.
    pub page_size: Option<u32>,
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    /// The v2 `envelope.list.table` block, an unknown `envelope.list` key
    /// and a `mailbox` colour key are all unknown keys.
    #[test]
    fn envelope_table_is_an_unknown_key() {
        let root = "[envelope.list.table]\nunseen-char = \"N\"\n[accounts.example]\n";
        let account = "[accounts.example]\nenvelope.list.table.flagged-char = \"F\"\n";

        for toml in [root, account] {
            let err = toml::from_str::<Config>(toml).unwrap_err().to_string();
            assert!(err.contains("unknown field `table`"), "{err}");
            assert!(!err.contains("MIGRATION.md"), "{err}");
        }

        let toml = "[accounts.example]\nenvelope.list.datetime-fnt = \"%F\"\n";
        let err = toml::from_str::<Config>(toml).unwrap_err().to_string();
        assert!(err.contains("datetime-fnt"), "{err}");

        let toml = "[mailbox.list.table]\nname-color = \"blue\"\n[accounts]\n";
        let err = toml::from_str::<Config>(toml).unwrap_err().to_string();
        assert!(err.contains("unknown field `list`"), "{err}");
    }

    #[test]
    fn relative_dates_are_refused() {
        let toml = "[envelope.list]\ndatetime-relative = true\n[accounts]\n";
        let err = toml::from_str::<Config>(toml).unwrap_err().to_string();
        assert!(err.contains("datetime-relative"), "{err}");
    }
}
