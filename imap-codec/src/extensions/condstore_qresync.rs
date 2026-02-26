use std::num::NonZeroU64;

use abnf_core::streaming::sp;
#[cfg(feature = "ext_condstore_qresync")]
use imap_types::extensions::condstore_qresync::{AttributeFlag, EntryTypeReq};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    character::streaming::char,
    combinator::{map, map_res, opt, value},
    sequence::{delimited, preceded, tuple},
};

use crate::{
    core::{atom, number64},
    decode::IMAPResult,
};

/// ```abnf
/// mod-sequence-valzer = "0" / mod-sequence-value
/// ```
pub(crate) fn mod_sequence_valzer(input: &[u8]) -> IMAPResult<&[u8], u64> {
    number64(input)
}

/// Positive unsigned 64-bit integer (mod-sequence) (1 <= n < 18,446,744,073,709,551,615)
///
/// ```abnf
/// mod-sequence-value  = 1*DIGIT
/// ```
pub(crate) fn mod_sequence_value(input: &[u8]) -> IMAPResult<&[u8], NonZeroU64> {
    map_res(number64, NonZeroU64::try_from)(input)
}

/// ```abnf
/// search-sort-mod-seq = "(" "MODSEQ" SP mod-sequence-value ")"
/// ```
pub(crate) fn search_sort_mod_seq(input: &[u8]) -> IMAPResult<&[u8], NonZeroU64> {
    delimited(
        char('('),
        preceded(tag_no_case("MODSEQ "), mod_sequence_value),
        char(')'),
    )(input)
}

/// ```abnf
/// search-modsequence = "MODSEQ" [search-modseq-ext] SP mod-sequence-valzer
/// ```
#[allow(clippy::type_complexity)]
pub(crate) fn search_modsequence(
    input: &[u8],
) -> IMAPResult<&[u8], (Option<(AttributeFlag, EntryTypeReq)>, u64)> {
    preceded(
        tag_no_case("MODSEQ"),
        tuple((opt(search_modseq_ext), preceded(sp, mod_sequence_valzer))),
    )(input)
}

/// ```abnf
/// search-modseq-ext = SP entry-name SP entry-type-req
/// ```
pub(crate) fn search_modseq_ext(input: &[u8]) -> IMAPResult<&[u8], (AttributeFlag, EntryTypeReq)> {
    tuple((preceded(sp, entry_name), preceded(sp, entry_type_req)))(input)
}

/// ```abnf
/// entry-name = entry-flag-name
/// ```
#[inline]
pub(crate) fn entry_name(input: &[u8]) -> IMAPResult<&[u8], AttributeFlag> {
    entry_flag_name(input)
}

/// Each system or user-defined flag \<flag\> is mapped to "/flags/\<flag\>".
///
/// \<entry-flag-name\> follows the escape rules used by "quoted" string as described in
/// Section 4.3 of \[RFC3501\]; e.g., for the flag \Seen, the corresponding \<entry-name\>
/// is "/flags/\\seen", and for the flag $MDNSent, the corresponding \<entry-name\>
/// is "/flags/$mdnsent".
///
/// ```abnf
/// entry-flag-name = DQUOTE "/flags/" attr-flag DQUOTE
/// ```
pub(crate) fn entry_flag_name(input: &[u8]) -> IMAPResult<&[u8], AttributeFlag> {
    delimited(tag_no_case("\"/flags/"), attr_flag, char('"'))(input)
}

/// ```abnf
/// attr-flag = "\\Answered" /
///             "\\Flagged" /
///             "\\Deleted" /
///             "\\Seen" /
///             "\\Draft" /
///             attr-flag-keyword /
///             attr-flag-extension
///             ;; Does not include "\\Recent".
/// ```
pub(crate) fn attr_flag(input: &[u8]) -> IMAPResult<&[u8], AttributeFlag> {
    alt((
        map(preceded(tag("\\\\"), atom), AttributeFlag::system),
        map(atom, AttributeFlag::Keyword),
    ))(input)
}

// /// ```abnf
// /// attr-flag-keyword = atom
// /// ```
// #[inline]
// pub(crate) fn attr_flag_keyword(input: &[u8]) -> IMAPResult<&[u8], Atom> {
//     atom(input)
// }

// /// Future expansion.
// /// Client implementations MUST accept flag-extension flags.
// /// Server implementations MUST NOT generate flag-extension flags, except as defined by future
// /// standards or Standards Track revisions of [RFC3501].
// ///
// /// ```abnf
// /// attr-flag-extension = "\\" atom
// /// ```
// pub(crate) fn attr_flag_extension(input: &[u8]) -> IMAPResult<&[u8], Atom> {
//     preceded(tag("\\\\"), atom)(input)
// }

/// ```abnf
/// ;; Perform SEARCH operation on a private metadata item,
/// ;; shared metadata item, or both.
/// entry-type-req = entry-type-resp / "all"
///
/// ;; Metadata item type.
/// entry-type-resp = "priv" / "shared"
/// ```
pub(crate) fn entry_type_req(input: &[u8]) -> IMAPResult<&[u8], EntryTypeReq> {
    alt((
        value(EntryTypeReq::Private, tag_no_case("priv")),
        value(EntryTypeReq::Shared, tag_no_case("shared")),
        value(EntryTypeReq::All, tag_no_case("all")),
    ))(input)
}

#[cfg(test)]
mod tests {
    use crate::response::resp_text;

    #[test]
    fn test_condstore_qresync_codes() {
        assert!(resp_text(b"[MODIFIED 7,9] Conditional STORE failed\r\n").is_ok());
        assert!(
            resp_text(b"[NOMODSEQ] Sorry, this mailbox format doesn't support modsequences\r\n")
                .is_ok()
        );
        assert!(resp_text(b"[HIGHESTMODSEQ 715194045007] Highest\r\n").is_ok());
    }
}
