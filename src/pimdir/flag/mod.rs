//! # pimdir flag
//!
//! The `pimdir flag` command family, naming any flag on a pimdir message,
//! keywords included, where the shared `flag` commands take the four
//! flags every backend carries.

pub mod add;
pub mod cli;
pub mod list;
pub mod remove;
pub mod set;

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::table::sanitize;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    email::flag::{Flag, IanaFlag, classify_iana},
    pimdir::backend::PimdirFlagged,
    shared::flag::arg::MessageIdsArg,
};

/// The ids a write command acts on, refusing none, as the shared flag
/// commands fail when no given id matched.
pub fn message_ids(arg: &MessageIdsArg) -> Result<Vec<&str>> {
    if arg.inner.is_empty() {
        bail!("No flag staged: no message id given");
    }
    Ok(arg.inner.iter().map(String::as_str).collect())
}

/// The repeatable `-f/--flag` option of the write commands.
#[derive(Debug, Parser)]
pub struct RawFlagsArg {
    /// A flag by its raw spelling, the option repeating for several.
    ///
    /// A system flag keeps its backslash (`\Seen`), a registered keyword
    /// its dollar (`$Forwarded`), and any other word is a custom keyword
    /// (`act`). A registered flag is only taken in that spelling, case
    /// aside, so
    /// `seen` is refused rather than staged as a keyword the store would
    /// read as `\Seen`.
    #[arg(name = "flags", value_name = "FLAG")]
    #[arg(short = 'f', long = "flag", required = true)]
    pub inner: Vec<String>,
}

impl RawFlagsArg {
    /// Parses every flag, refusing the first invalid one before anything
    /// is staged.
    pub fn parse(&self) -> Result<Vec<Flag>> {
        self.inner.iter().map(|raw| parse_flag(raw)).collect()
    }
}

/// The characters IMAP forbids in a flag (RFC 3501 `atom-specials` less
/// the ones checked apart), a backslash being allowed only in front of a
/// system flag.
const SPECIALS: &[char] = &['(', ')', '{', '%', '*', '"', ']', '\\'];

/// Parses a raw flag, refusing what the store cannot carry as typed.
///
/// A registered flag is accepted in its canonical spelling only, compared
/// without case as IMAP does, and staged in that canonical case.
/// `\Deleted` is a deletion and `\Recent` belongs to the server, so
/// neither can be staged as a flag. A backslash names a system flag, so
/// one in front of any other word is refused, as are a character outside
/// ASCII and one IMAP forbids in a flag.
pub fn parse_flag(raw: &str) -> Result<Flag> {
    if raw.is_empty() {
        bail!("A flag cannot be empty");
    }
    if raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        bail!(
            "Flag `{}` holds whitespace or a control character",
            raw.escape_debug()
        );
    }
    if raw.eq_ignore_ascii_case("\\Recent") {
        bail!("`\\Recent` is set by the server and cannot be stored");
    }
    if let Some(c) = raw.chars().find(|c| !c.is_ascii()) {
        bail!("Flag `{raw}` holds `{c}`: IMAP and JMAP flags are ASCII only");
    }
    let word = raw.strip_prefix('\\').unwrap_or(raw);
    if let Some(c) = word.chars().find(|c| SPECIALS.contains(c)) {
        bail!("Flag `{raw}` holds `{c}`, which IMAP does not allow in a flag");
    }

    let Some(iana) = classify_iana(raw) else {
        if raw.starts_with('\\') {
            bail!("`{raw}` is not a system flag: write a keyword without the backslash");
        }
        return Ok(Flag::from_raw(raw));
    };

    let canonical = Flag::from_iana(iana);
    if iana == IanaFlag::Deleted {
        bail!("`\\Deleted` deletes the message: use `message delete` instead");
    }
    if !raw.eq_ignore_ascii_case(canonical.raw()) {
        bail!(
            "`{raw}` names the registered flag `{}`: write it that way",
            canonical.raw()
        );
    }
    Ok(canonical)
}

/// The output of `pimdir flag add`, `set` and `remove`: the row each
/// message's change waits in.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct PimdirFlagsStaged {
    /// One row per message, in the order given.
    pub queued: Vec<PimdirFlagRow>,
}

/// One message's staged change.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirFlagRow {
    /// The message's public id.
    pub id: String,
    /// The `set-flags` row, which `pimdir queue show` and `cancel` take.
    pub queue_id: i64,
}

impl PimdirFlagsStaged {
    /// The rows a write staged, failing when none of its ids matched, as
    /// the shared flag commands do.
    pub fn found(staged: Vec<PimdirFlagged>) -> Result<Self> {
        if staged.is_empty() {
            bail!("No flag staged: no id matched in the mailbox");
        }
        Ok(Self::from(staged))
    }
}

impl From<Vec<PimdirFlagged>> for PimdirFlagsStaged {
    fn from(staged: Vec<PimdirFlagged>) -> Self {
        let queued = staged
            .into_iter()
            .map(|flagged| PimdirFlagRow {
                id: flagged.id,
                queue_id: flagged.queue_id,
            })
            .collect();
        Self { queued }
    }
}

impl fmt::Display for PimdirFlagsStaged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rows: Vec<String> = self
            .queued
            .iter()
            .map(|row| format!("{} as row {}", sanitize(&row.id), row.queue_id))
            .collect();
        writeln!(
            f,
            "Flags staged on {} message(s): {}",
            self.queued.len(),
            rows.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keyword_is_taken_verbatim() {
        assert_eq!(parse_flag("act").unwrap().raw(), "act");
        assert_eq!(parse_flag("$label5").unwrap().raw(), "$label5");
        assert_eq!(parse_flag("NonJunk").unwrap().raw(), "NonJunk");
    }

    #[test]
    fn a_registered_flag_is_taken_in_its_canonical_spelling() {
        assert_eq!(parse_flag("\\Seen").unwrap().raw(), "\\Seen");
        assert_eq!(parse_flag("\\seen").unwrap().raw(), "\\Seen");
        assert_eq!(parse_flag("$forwarded").unwrap().raw(), "$Forwarded");
    }

    #[test]
    fn a_registered_flag_in_another_spelling_is_refused_naming_it() {
        let err = parse_flag("seen").unwrap_err().to_string();
        assert!(err.contains("`\\Seen`"), "{err}");
        let err = parse_flag("$seen").unwrap_err().to_string();
        assert!(err.contains("`\\Seen`"), "{err}");
        let err = parse_flag("forwarded").unwrap_err().to_string();
        assert!(err.contains("`$Forwarded`"), "{err}");
    }

    #[test]
    fn deleted_recent_and_malformed_values_are_refused() {
        assert!(parse_flag("\\Deleted").is_err());
        assert!(parse_flag("deleted").is_err());
        assert!(parse_flag("\\Recent").is_err());
        assert!(parse_flag("\\recent").is_err());
        assert!(parse_flag("").is_err());
        assert!(parse_flag("two words").is_err());
        assert!(parse_flag("tab\there").is_err());
        assert!(parse_flag("bell\u{7}").is_err());
    }

    #[test]
    fn a_backslash_word_or_an_imap_special_is_refused() {
        let err = parse_flag("\\Foo").unwrap_err().to_string();
        assert!(err.contains("not a system flag"), "{err}");
        assert!(parse_flag("\\\\Seen").is_err());
        assert!(parse_flag("\\*").is_err());
        assert!(parse_flag("a(b").is_err());
        assert!(parse_flag("quote\"d").is_err());
        assert!(parse_flag("tail]").is_err());
        assert!(parse_flag("100%").is_err());
        let err = parse_flag("a\\b").unwrap_err().to_string();
        assert!(err.contains("IMAP does not allow"), "{err}");
    }

    #[test]
    fn recent_is_refused_as_the_server_s() {
        let err = parse_flag("\\Recent").unwrap_err().to_string();
        assert!(err.contains("set by the server"), "{err}");
    }

    #[test]
    fn a_flag_outside_ascii_is_refused() {
        let err = parse_flag("über").unwrap_err().to_string();
        assert!(err.contains("ASCII"), "{err}");
        assert!(parse_flag("日本").is_err());
    }

    #[test]
    fn a_write_matching_no_id_fails() {
        let err = PimdirFlagsStaged::found(Vec::new())
            .unwrap_err()
            .to_string();
        assert!(err.contains("no id matched"), "{err}");
    }

    #[test]
    fn a_write_with_no_id_is_refused() {
        let none = MessageIdsArg { inner: Vec::new() };
        assert!(message_ids(&none).is_err());
        let two = MessageIdsArg {
            inner: vec!["41".into(), "42".into()],
        };
        assert_eq!(message_ids(&two).unwrap(), ["41", "42"]);
    }

    #[test]
    fn staged_rows_print_and_serialize() {
        let output = PimdirFlagsStaged::from(vec![
            PimdirFlagged {
                id: "42".into(),
                queue_id: 7,
            },
            PimdirFlagged {
                id: "43".into(),
                queue_id: 8,
            },
        ]);
        assert_eq!(
            output.to_string(),
            "Flags staged on 2 message(s): 42 as row 7, 43 as row 8\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"queued":[{"id":"42","queueId":7},{"id":"43","queueId":8}]}"#
        );
    }
}
