//! Email flag (a.k.a. keyword) shared across all protocols.
//!
//! A [`Flag`] carries two views of the same value: the original wire
//! spelling as observed on the backend, and an optional [`IanaFlag`]
//! classification when the wire string matches an IANA-registered
//! keyword (case-insensitive, leading `\` or `$` stripped). Custom
//! user-defined keywords flow through unchanged with `iana = None`.
//!
//! Equality and ordering are IANA-first so that wire spellings like
//! `\Seen`, `$seen` and `seen` collapse to the same logical flag while
//! custom keywords compare case-insensitively. This lets a
//! `BTreeSet<Flag>` (the storage shape used by [`Envelope::flags`]) act
//! as a normalised set across backends.
//!
//! [`Envelope::flags`]: crate::email::envelope::Envelope::flags

use std::{cmp::Ordering, hash::Hash};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A flag attached to an envelope or message.
///
/// Constructed via [`Flag::from_raw`] (wire spelling in, IANA lookup
/// derived) or [`Flag::from_iana`] (IANA tag in, canonical wire
/// spelling synthesised).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Flag {
    raw: String,
    iana: Option<IanaFlag>,
}

/// IANA-registered email keywords supported by the shared API.
///
/// Listed in the canonical wire-spelling table at
/// <https://www.iana.org/assignments/imap-jmap-keywords/>. Variant
/// order is the lookup order; [`Ord`] is derived from declaration
/// order to give stable per-IANA-key sorting.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum IanaFlag {
    Seen,
    Answered,
    Flagged,
    Draft,
    /// Sync verb. Adapters round-trip `\Deleted` so single-side
    /// workflows behave, but a sync engine should translate
    /// "one side has Deleted" into `delete_message` on the other
    /// side rather than propagating the flag.
    Deleted,
    Forwarded,
    Junk,
    NotJunk,
    Phishing,
    Important,
    MdnSent,
}

impl Flag {
    /// Builds a [`Flag`] from a wire spelling. The raw string is kept
    /// verbatim; the IANA classification is derived by
    /// [`classify_iana`].
    pub fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let iana = classify_iana(&raw);
        Self { raw, iana }
    }

    /// Builds a [`Flag`] tagged with an [`IanaFlag`] and the matching
    /// canonical wire spelling (`\Seen`, `$Forwarded`, …). Used by
    /// adapters whose wire format does not carry casing (Maildir
    /// letters) or when synthesising flags client-side.
    pub fn from_iana(iana: IanaFlag) -> Self {
        Self {
            raw: canonical_raw(iana).to_string(),
            iana: Some(iana),
        }
    }

    /// Original wire spelling as observed on the backend (or the
    /// canonical spelling when built from an [`IanaFlag`]).
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// IANA classification when the raw spelling matched a registered
    /// keyword; `None` for user-defined custom keywords.
    pub fn iana(&self) -> Option<IanaFlag> {
        self.iana
    }

    pub fn is_seen(&self) -> bool {
        matches!(self.iana, Some(IanaFlag::Seen))
    }

    pub fn is_answered(&self) -> bool {
        matches!(self.iana, Some(IanaFlag::Answered))
    }

    pub fn is_flagged(&self) -> bool {
        matches!(self.iana, Some(IanaFlag::Flagged))
    }

    pub fn is_draft(&self) -> bool {
        matches!(self.iana, Some(IanaFlag::Draft))
    }

    pub fn is_junk(&self) -> bool {
        matches!(self.iana, Some(IanaFlag::Junk))
    }

    pub fn is_important(&self) -> bool {
        matches!(self.iana, Some(IanaFlag::Important))
    }
}

impl PartialEq for Flag {
    fn eq(&self, other: &Self) -> bool {
        match (self.iana, other.iana) {
            (Some(a), Some(b)) => a == b,
            (None, None) => self.raw.eq_ignore_ascii_case(&other.raw),
            _ => false,
        }
    }
}

impl Eq for Flag {}

impl Ord for Flag {
    /// IANA-tagged flags sort before custom keywords (IANA-first
    /// canonical ordering). Within each group, IANA flags use the
    /// derived enum order and custom keywords compare on their
    /// lowercase raw text so that `"Foo"` and `"foo"` order
    /// consistently with equality.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.iana, other.iana) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => self
                .raw
                .to_ascii_lowercase()
                .cmp(&other.raw.to_ascii_lowercase()),
        }
    }
}

impl PartialOrd for Flag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Flag {
    /// Hashes the IANA tag first; falls back to the lowercase raw
    /// bytes when none, so that values comparing equal hash equal.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.iana {
            Some(iana) => {
                0u8.hash(state);
                iana.hash(state);
            }
            None => {
                1u8.hash(state);
                for b in self.raw.as_bytes() {
                    b.to_ascii_lowercase().hash(state);
                }
            }
        }
    }
}

/// Classifies a wire flag spelling against the IANA keyword table.
///
/// Strips a single leading `\` or `$` prefix, lowercases the rest, and
/// matches against the canonical name list. Returns `None` for custom
/// user-defined keywords.
pub fn classify_iana(raw: &str) -> Option<IanaFlag> {
    let stripped = raw
        .strip_prefix('\\')
        .or_else(|| raw.strip_prefix('$'))
        .unwrap_or(raw);

    match stripped.to_ascii_lowercase().as_str() {
        "seen" => Some(IanaFlag::Seen),
        "answered" => Some(IanaFlag::Answered),
        "flagged" => Some(IanaFlag::Flagged),
        "draft" => Some(IanaFlag::Draft),
        "deleted" => Some(IanaFlag::Deleted),
        "forwarded" => Some(IanaFlag::Forwarded),
        "junk" => Some(IanaFlag::Junk),
        "notjunk" => Some(IanaFlag::NotJunk),
        "phishing" => Some(IanaFlag::Phishing),
        "important" => Some(IanaFlag::Important),
        "mdnsent" => Some(IanaFlag::MdnSent),
        _ => None,
    }
}

/// Canonical wire spelling for each IANA keyword. The four RFC 3501
/// system flags use the `\Capital` form; the rest use the `$Capital`
/// form per the IANA mail keywords registry.
fn canonical_raw(iana: IanaFlag) -> &'static str {
    match iana {
        IanaFlag::Seen => "\\Seen",
        IanaFlag::Answered => "\\Answered",
        IanaFlag::Flagged => "\\Flagged",
        IanaFlag::Draft => "\\Draft",
        IanaFlag::Deleted => "\\Deleted",
        IanaFlag::Forwarded => "$Forwarded",
        IanaFlag::Junk => "$Junk",
        IanaFlag::NotJunk => "$NotJunk",
        IanaFlag::Phishing => "$Phishing",
        IanaFlag::Important => "$Important",
        IanaFlag::MdnSent => "$MDNSent",
    }
}

/// Direction of a flag store operation.
///
/// `Set` replaces the message's flag set with the given list; `Add`
/// and `Remove` patch the existing set.
#[derive(Clone, Copy, Debug)]
pub enum FlagOp {
    Add,
    Set,
    Remove,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn classify_strips_prefix_and_is_case_insensitive() {
        assert_eq!(classify_iana("\\Seen"), Some(IanaFlag::Seen));
        assert_eq!(classify_iana("$seen"), Some(IanaFlag::Seen));
        assert_eq!(classify_iana("SEEN"), Some(IanaFlag::Seen));
        assert_eq!(classify_iana("$MDNSent"), Some(IanaFlag::MdnSent));
        assert_eq!(classify_iana("foo"), None);
    }

    #[test]
    fn from_raw_populates_iana_when_recognised() {
        let f = Flag::from_raw("\\Seen");
        assert_eq!(f.raw(), "\\Seen");
        assert_eq!(f.iana(), Some(IanaFlag::Seen));

        let f = Flag::from_raw("custom-label");
        assert_eq!(f.raw(), "custom-label");
        assert_eq!(f.iana(), None);
    }

    #[test]
    fn iana_uses_canonical_spelling() {
        assert_eq!(Flag::from_iana(IanaFlag::Seen).raw(), "\\Seen");
        assert_eq!(Flag::from_iana(IanaFlag::Forwarded).raw(), "$Forwarded");
        assert_eq!(Flag::from_iana(IanaFlag::MdnSent).raw(), "$MDNSent");
    }

    #[test]
    fn equality_collapses_wire_variants() {
        assert_eq!(Flag::from_raw("\\Seen"), Flag::from_raw("$seen"));
        assert_eq!(Flag::from_raw("\\Seen"), Flag::from_iana(IanaFlag::Seen));
        assert_eq!(Flag::from_raw("FOO"), Flag::from_raw("foo"));
        assert_ne!(Flag::from_raw("foo"), Flag::from_iana(IanaFlag::Seen));
        assert_ne!(Flag::from_raw("foo"), Flag::from_raw("bar"));
    }

    #[test]
    fn btreeset_dedupes_across_spellings() {
        let mut set: BTreeSet<Flag> = BTreeSet::new();
        set.insert(Flag::from_raw("\\Seen"));
        set.insert(Flag::from_raw("$seen"));
        set.insert(Flag::from_iana(IanaFlag::Seen));
        set.insert(Flag::from_raw("custom"));
        set.insert(Flag::from_raw("CUSTOM"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn predicates_match_iana_only() {
        assert!(Flag::from_iana(IanaFlag::Seen).is_seen());
        assert!(!Flag::from_raw("seen-ish").is_seen());
        assert!(Flag::from_iana(IanaFlag::Draft).is_draft());
        assert!(Flag::from_iana(IanaFlag::Junk).is_junk());
    }
}
