use abnf_core::streaming::{dquote, sp};
use imap_types::{
    core::QuotedChar,
    flag::FlagNameAttribute,
    mailbox::{ListCharString, ListMailbox, Mailbox},
    response::Data,
    utils::indicators::is_list_char,
};
#[cfg(feature = "ext_condstore_qresync")]
use nom::character::streaming::char;
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case, take_while1},
    combinator::{map, opt, value},
    multi::many0,
    sequence::{delimited, preceded, terminated, tuple},
};

#[cfg(feature = "ext_condstore_qresync")]
use crate::extensions::condstore_qresync::search_sort_mod_seq;
#[cfg(feature = "ext_metadata")]
use crate::extensions::metadata::metadata_resp;
#[cfg(feature = "ext_namespace")]
use crate::extensions::namespace::namespace_response;
use crate::{
    core::{astring, nil, number, nz_number, quoted_char, string},
    decode::IMAPResult,
    extensions::{
        quota::{quota_response, quotaroot_response},
        thread::thread_data,
    },
    flag::{flag_list, mbx_list_flags},
    status::status_att_list,
};

/// `list-mailbox = 1*list-char / string`
pub(crate) fn list_mailbox(input: &[u8]) -> IMAPResult<&[u8], ListMailbox> {
    alt((
        map(take_while1(is_list_char), |bytes: &[u8]| {
            // # Safety
            //
            // `unwrap` is safe here, because `is_list_char` enforces that the bytes ...
            //   * contain ASCII-only characters, i.e., `from_utf8` will return `Ok`.
            //   * are valid according to `ListCharString::verify()`, i.e., `unvalidated` is safe.
            ListMailbox::Token(ListCharString::unvalidated(
                std::str::from_utf8(bytes).unwrap(),
            ))
        }),
        map(string, ListMailbox::String),
    ))(input)
}

/// `mailbox = "INBOX" / astring`
///
/// INBOX is case-insensitive. All case variants of INBOX (e.g., "iNbOx")
/// MUST be interpreted as INBOX not as an astring.
///
/// An astring which consists of the case-insensitive sequence
/// "I" "N" "B" "O" "X" is considered to be INBOX and not an astring.
///
/// Refer to section 5.1 for further semantic details of mailbox names.
pub(crate) fn mailbox(input: &[u8]) -> IMAPResult<&[u8], Mailbox> {
    map(astring, Mailbox::from)(input)
}

/// ```abnf
/// mailbox-data = "FLAGS" SP flag-list /
///                "LIST" SP mailbox-list /
///                "LSUB" SP mailbox-list /
///                "SEARCH" *(SP nz-number) [SP search-sort-mod-seq] /
///                                         ^^^^^^^^^^^^^^^^^^^^^^^^
///                                         |
///                                         RFC 7162 (edited)
///                "STATUS" SP mailbox SP "(" [status-att-list] ")" /
///                "METADATA" SP mailbox SP (entry-values / entry-list) / ; RFC 5464
///                number SP "EXISTS" /
///                number SP "RECENT"
/// ```
///
/// FROM RFC 7162 (CONDSTORE/QRESYNC):
///
/// ```abnf
/// mailbox-data =/ "SEARCH" [1*(SP nz-number) SP search-sort-mod-seq]
///
/// search-sort-mod-seq = "(" "MODSEQ" SP mod-sequence-value ")"
/// ```
pub(crate) fn mailbox_data(input: &[u8]) -> IMAPResult<&[u8], Data> {
    alt((
        map(preceded(tag_no_case(b"FLAGS "), flag_list), Data::Flags),
        map(
            preceded(tag_no_case(b"LIST "), mailbox_list),
            |(items, delimiter, mailbox)| Data::List {
                items: items.unwrap_or_default(),
                mailbox,
                delimiter,
            },
        ),
        map(
            preceded(tag_no_case(b"LSUB "), mailbox_list),
            |(items, delimiter, mailbox)| Data::Lsub {
                items: items.unwrap_or_default(),
                mailbox,
                delimiter,
            },
        ),
        #[cfg(not(feature = "ext_condstore_qresync"))]
        map(
            #[cfg(not(feature = "quirk_trailing_space_search"))]
            tuple((tag_no_case(b"SEARCH"), many0(preceded(sp, nz_number)))),
            #[cfg(feature = "quirk_trailing_space_search")]
            tuple((
                tag_no_case(b"SEARCH"),
                many0(preceded(sp, nz_number)),
                opt(sp),
            )),
            #[cfg(not(feature = "quirk_trailing_space_search"))]
            |(_, nums)| Data::Search(nums),
            #[cfg(feature = "quirk_trailing_space_search")]
            |(_, nums, _)| Data::Search(nums),
        ),
        #[cfg(feature = "ext_condstore_qresync")]
        map(
            #[cfg(not(feature = "quirk_trailing_space_search"))]
            tuple((
                tag_no_case(b"SEARCH"),
                many0(preceded(sp, nz_number)),
                opt(preceded(char(' '), search_sort_mod_seq)),
            )),
            #[cfg(feature = "quirk_trailing_space_search")]
            tuple((
                tag_no_case(b"SEARCH"),
                many0(preceded(sp, nz_number)),
                opt(preceded(char(' '), search_sort_mod_seq)),
                opt(sp),
            )),
            #[cfg(not(feature = "quirk_trailing_space_search"))]
            |(_, nums, modseq)| Data::Search(nums, modseq),
            #[cfg(feature = "quirk_trailing_space_search")]
            |(_, nums, modseq, _)| Data::Search(nums, modseq),
        ),
        #[cfg(not(feature = "ext_condstore_qresync"))]
        map(
            preceded(tag_no_case(b"SORT"), many0(preceded(sp, nz_number))),
            Data::Sort,
        ),
        #[cfg(feature = "ext_condstore_qresync")]
        map(
            tuple((
                tag_no_case(b"SORT"),
                many0(preceded(sp, nz_number)),
                opt(preceded(char(' '), search_sort_mod_seq)),
            )),
            |(_, nums, modseq)| Data::Sort(nums, modseq),
        ),
        thread_data,
        map(
            tuple((
                tag_no_case(b"STATUS "),
                mailbox,
                delimited(tag(b" ("), opt(status_att_list), tag(b")")),
                #[cfg(feature = "quirk_trailing_space_status")]
                opt(sp),
                #[cfg(not(feature = "quirk_trailing_space_status"))]
                nom::combinator::success(()),
            )),
            |(_, mailbox, items, _)| Data::Status {
                mailbox,
                items: items.unwrap_or_default().into(),
            },
        ),
        #[cfg(feature = "ext_metadata")]
        metadata_resp,
        #[cfg(feature = "ext_namespace")]
        namespace_response,
        map(terminated(number, tag_no_case(b" EXISTS")), Data::Exists),
        map(terminated(number, tag_no_case(b" RECENT")), Data::Recent),
        quotaroot_response,
        quota_response,
    ))(input)
}

/// `mailbox-list = "(" [mbx-list-flags] ")" SP
///                 (DQUOTE QUOTED-CHAR DQUOTE / nil) SP
///                 mailbox`
#[allow(clippy::type_complexity)]
pub(crate) fn mailbox_list(
    input: &[u8],
) -> IMAPResult<&[u8], (Option<Vec<FlagNameAttribute>>, Option<QuotedChar>, Mailbox)> {
    let mut parser = tuple((
        delimited(tag(b"("), opt(mbx_list_flags), tag(b")")),
        sp,
        alt((
            map(delimited(dquote, quoted_char, dquote), Option::Some),
            value(None, nil),
        )),
        sp,
        mailbox,
    ));

    let (remaining, (mbx_list_flags, _, maybe_delimiter, _, mailbox)) = parser(input)?;

    Ok((remaining, (mbx_list_flags, maybe_delimiter, mailbox)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailbox() {
        assert!(mailbox(b"\"iNbOx\"").is_ok());
        assert!(mailbox(b"{3}\r\naaa\r\n").is_ok());
        assert!(mailbox(b"inbox ").is_ok());
        assert!(mailbox(b"inbox.sent ").is_ok());
        assert!(mailbox(b"aaa").is_err());
    }
}
