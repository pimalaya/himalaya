use abnf_core::streaming::sp;
use imap_types::{
    command::CommandBody,
    core::{Charset, Vec1},
    search::SearchKey,
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, map_opt, opt, value},
    multi::separated_list1,
    sequence::{delimited, separated_pair, tuple},
};

#[cfg(feature = "ext_condstore_qresync")]
use crate::extensions::condstore_qresync::search_modsequence;
use crate::{
    core::{astring, atom, charset, number},
    datetime::date,
    decode::{IMAPErrorKind, IMAPParseError, IMAPResult},
    fetch::header_fld_name,
    sequence::sequence_set,
};

/// `search = "SEARCH" [SP "CHARSET" SP charset] 1*(SP search-key)`
///
/// Note: CHARSET argument MUST be registered with IANA
///
/// errata id: 261
pub(crate) fn search(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case(b"SEARCH"),
        opt(map(
            tuple((sp, tag_no_case(b"CHARSET"), sp, charset)),
            |(_, _, _, charset)| charset,
        )),
        sp,
        map(separated_list1(sp, search_key(9)), Vec1::unvalidated),
    ));

    let (remaining, (_, charset, _, criteria)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Search {
            charset,
            criteria,
            uid: false,
        },
    ))
}

/// ```abnf
/// search-key = "ALL" /
///              "ANSWERED" /
///              "BCC" SP astring /
///              "BEFORE" SP date /
///              "BODY" SP astring /
///              "CC" SP astring /
///              "DELETED" /
///              "FLAGGED" /
///              "FROM" SP astring /
///              "KEYWORD" SP flag-keyword /
///              "NEW" /
///              "OLD" /
///              "ON" SP date /
///              "RECENT" /
///              "SEEN" /
///              "SINCE" SP date /
///              "SUBJECT" SP astring /
///              "TEXT" SP astring /
///              "TO" SP astring /
///              "UNANSWERED" /
///              "UNDELETED" /
///              "UNFLAGGED" /
///              "UNKEYWORD" SP flag-keyword /
///              "UNSEEN" /
///                ; Above this line were in [IMAP2]
///              "DRAFT" /
///              "HEADER" SP header-fld-name SP astring /
///              "LARGER" SP number /
///              "NOT" SP search-key /
///              "OR" SP search-key SP search-key /
///              "SENTBEFORE" SP date /
///              "SENTON" SP date /
///              "SENTSINCE" SP date /
///              "SMALLER" SP number /
///              "UID" SP sequence-set /
///              "UNDRAFT" /
///              search-modsequence / ; RFC 7162
///              sequence-set /
///              "(" search-key *(SP search-key) ")"
/// ```
///
/// This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed. (8 should suffice).
pub(crate) fn search_key(
    remaining_recursions: usize,
) -> impl Fn(&[u8]) -> IMAPResult<&[u8], SearchKey> {
    move |input: &[u8]| search_key_limited(input, remaining_recursions)
}

fn search_key_limited(input: &[u8], remaining_recursion: usize) -> IMAPResult<&[u8], SearchKey> {
    if remaining_recursion == 0 {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::RecursionLimitExceeded,
        }));
    }

    let search_key = |input| search_key_limited(input, remaining_recursion.saturating_sub(1));

    alt((
        alt((
            value(SearchKey::All, tag_no_case(b"ALL")),
            value(SearchKey::Answered, tag_no_case(b"ANSWERED")),
            map(tuple((tag_no_case(b"BCC"), sp, astring)), |(_, _, val)| {
                SearchKey::Bcc(val)
            }),
            map(
                tuple((tag_no_case(b"BEFORE"), sp, map_opt(date, |date| date))),
                |(_, _, date)| SearchKey::Before(date),
            ),
            map(tuple((tag_no_case(b"BODY"), sp, astring)), |(_, _, val)| {
                SearchKey::Body(val)
            }),
            map(tuple((tag_no_case(b"CC"), sp, astring)), |(_, _, val)| {
                SearchKey::Cc(val)
            }),
            value(SearchKey::Deleted, tag_no_case(b"DELETED")),
            value(SearchKey::Flagged, tag_no_case(b"FLAGGED")),
            map(tuple((tag_no_case(b"FROM"), sp, astring)), |(_, _, val)| {
                SearchKey::From(val)
            }),
            map(
                // Note: `flag_keyword` parser returns `Flag`. Because Rust does not have first-class enum variants
                // it is not possible to fix SearchKey(Flag::Keyword), but only SearchKey(Flag).
                // Thus `SearchKey::Keyword(Atom)` is used instead. This is, why we use also `atom` parser here and not `flag_keyword` parser.
                tuple((tag_no_case(b"KEYWORD"), sp, atom)),
                |(_, _, val)| SearchKey::Keyword(val),
            ),
            value(SearchKey::New, tag_no_case(b"NEW")),
            value(SearchKey::Old, tag_no_case(b"OLD")),
            map(
                tuple((tag_no_case(b"ON"), sp, map_opt(date, |date| date))),
                |(_, _, date)| SearchKey::On(date),
            ),
            value(SearchKey::Recent, tag_no_case(b"RECENT")),
            value(SearchKey::Seen, tag_no_case(b"SEEN")),
            map(
                tuple((tag_no_case(b"SINCE"), sp, map_opt(date, |date| date))),
                |(_, _, date)| SearchKey::Since(date),
            ),
            map(
                tuple((tag_no_case(b"SUBJECT"), sp, astring)),
                |(_, _, val)| SearchKey::Subject(val),
            ),
            map(tuple((tag_no_case(b"TEXT"), sp, astring)), |(_, _, val)| {
                SearchKey::Text(val)
            }),
            map(tuple((tag_no_case(b"TO"), sp, astring)), |(_, _, val)| {
                SearchKey::To(val)
            }),
        )),
        alt((
            value(SearchKey::Unanswered, tag_no_case(b"UNANSWERED")),
            value(SearchKey::Undeleted, tag_no_case(b"UNDELETED")),
            value(SearchKey::Unflagged, tag_no_case(b"UNFLAGGED")),
            map(
                // Note: `flag_keyword` parser returns `Flag`. Because Rust does not have first-class enum variants
                // it is not possible to fix SearchKey(Flag::Keyword), but only SearchKey(Flag).
                // Thus `SearchKey::Keyword(Atom)` is used instead. This is, why we use also `atom` parser here and not `flag_keyword` parser.
                tuple((tag_no_case(b"UNKEYWORD"), sp, atom)),
                |(_, _, val)| SearchKey::Unkeyword(val),
            ),
            value(SearchKey::Unseen, tag_no_case(b"UNSEEN")),
            value(SearchKey::Draft, tag_no_case(b"DRAFT")),
            map(
                tuple((tag_no_case(b"HEADER"), sp, header_fld_name, sp, astring)),
                |(_, _, key, _, val)| SearchKey::Header(key, val),
            ),
            map(
                tuple((tag_no_case(b"LARGER"), sp, number)),
                |(_, _, val)| SearchKey::Larger(val),
            ),
            map(
                tuple((tag_no_case(b"NOT"), sp, search_key)),
                |(_, _, val)| SearchKey::Not(Box::new(val)),
            ),
            map(
                tuple((tag_no_case(b"OR"), sp, search_key, sp, search_key)),
                |(_, _, alt1, _, alt2)| SearchKey::Or(Box::new(alt1), Box::new(alt2)),
            ),
            map(
                tuple((tag_no_case(b"SENTBEFORE"), sp, map_opt(date, |date| date))),
                |(_, _, date)| SearchKey::SentBefore(date),
            ),
            map(
                tuple((tag_no_case(b"SENTON"), sp, map_opt(date, |date| date))),
                |(_, _, date)| SearchKey::SentOn(date),
            ),
            map(
                tuple((tag_no_case(b"SENTSINCE"), sp, map_opt(date, |date| date))),
                |(_, _, date)| SearchKey::SentSince(date),
            ),
            map(
                tuple((tag_no_case(b"SMALLER"), sp, number)),
                |(_, _, val)| SearchKey::Smaller(val),
            ),
            map(
                tuple((tag_no_case(b"UID"), sp, sequence_set)),
                |(_, _, val)| SearchKey::Uid(val),
            ),
            value(SearchKey::Undraft, tag_no_case(b"UNDRAFT")),
            #[cfg(feature = "ext_condstore_qresync")]
            map(search_modsequence, |(entry, modseq)| {
                SearchKey::ModSequence { entry, modseq }
            }),
            map(sequence_set, SearchKey::SequenceSet),
            map(
                delimited(tag(b"("), separated_list1(sp, search_key), tag(b")")),
                |val| SearchKey::And(Vec1::unvalidated(val)),
            ),
        )),
    ))(input)
}

/// ```abnf
/// search-criteria = charset 1*(SP search-key)
/// ```
pub(crate) fn search_criteria(input: &[u8]) -> IMAPResult<&[u8], (Charset, Vec1<SearchKey>)> {
    let mut parser = separated_pair(
        charset,
        sp,
        map(separated_list1(sp, search_key(9)), Vec1::unvalidated),
    );

    let (remaining, (charset, search_keys)) = parser(input)?;

    Ok((remaining, (charset, search_keys)))
}

#[cfg(test)]
mod tests {
    use imap_types::{
        core::{AString, Atom},
        datetime::NaiveDate,
        sequence::{Sequence, SequenceSet},
    };

    use super::*;
    use crate::testing::known_answer_test_encode;

    #[test]
    fn test_parse_search() {
        use imap_types::{
            search::SearchKey::*,
            sequence::{SeqOrUid::Value, Sequence::*, SequenceSet as SequenceSetData},
        };

        let (_rem, val) = search(b"search (uid 5)???").unwrap();
        assert_eq!(
            val,
            CommandBody::Search {
                charset: None,
                criteria: Vec1::from(And(Vec1::from(Uid(SequenceSetData(
                    vec![Single(Value(5.try_into().unwrap()))]
                        .try_into()
                        .unwrap()
                ))))),
                uid: false,
            }
        );

        let (_rem, val) = search(b"search (uid 5 or uid 5 (uid 1 uid 2) not uid 5)???").unwrap();
        let expected = CommandBody::Search {
            charset: None,
            criteria: Vec1::from(And(vec![
                Uid(SequenceSetData(
                    vec![Single(Value(5.try_into().unwrap()))]
                        .try_into()
                        .unwrap(),
                )),
                Or(
                    Box::new(Uid(SequenceSetData(
                        vec![Single(Value(5.try_into().unwrap()))]
                            .try_into()
                            .unwrap(),
                    ))),
                    Box::new(And(vec![
                        Uid(SequenceSetData(
                            vec![Single(Value(1.try_into().unwrap()))]
                                .try_into()
                                .unwrap(),
                        )),
                        Uid(SequenceSetData(
                            vec![Single(Value(2.try_into().unwrap()))]
                                .try_into()
                                .unwrap(),
                        )),
                    ]
                    .try_into()
                    .unwrap())),
                ),
                Not(Box::new(Uid(SequenceSetData(
                    vec![Single(Value(5.try_into().unwrap()))]
                        .try_into()
                        .unwrap(),
                )))),
            ]
            .try_into()
            .unwrap())),
            uid: false,
        };
        assert_eq!(val, expected);
    }

    #[test]
    fn test_parse_search_key() {
        assert!(search_key(1)(b"1:5|").is_ok());
        assert!(search_key(1)(b"(1:5)|").is_err());
        assert!(search_key(2)(b"(1:5)|").is_ok());
        assert!(search_key(2)(b"((1:5))|").is_err());
    }

    #[test]
    fn test_encode_search_key() {
        let tests = [
            (
                SearchKey::And(Vec1::try_from(vec![SearchKey::Answered]).unwrap()),
                b"(ANSWERED)".as_ref(),
            ),
            (
                SearchKey::And(Vec1::try_from(vec![SearchKey::Answered, SearchKey::Seen]).unwrap()),
                b"(ANSWERED SEEN)".as_ref(),
            ),
            (
                SearchKey::SequenceSet(SequenceSet::try_from(1).unwrap()),
                b"1",
            ),
            (SearchKey::All, b"ALL"),
            (SearchKey::Answered, b"ANSWERED"),
            (SearchKey::Bcc(AString::try_from("A").unwrap()), b"BCC A"),
            (
                SearchKey::Before(
                    NaiveDate::try_from(chrono::NaiveDate::from_ymd_opt(2023, 4, 12).unwrap())
                        .unwrap(),
                ),
                b"BEFORE \"12-Apr-2023\"",
            ),
            (SearchKey::Body(AString::try_from("A").unwrap()), b"BODY A"),
            (SearchKey::Cc(AString::try_from("A").unwrap()), b"CC A"),
            (SearchKey::Deleted, b"DELETED"),
            (SearchKey::Draft, b"DRAFT"),
            (SearchKey::Flagged, b"FLAGGED"),
            (SearchKey::From(AString::try_from("A").unwrap()), b"FROM A"),
            (
                SearchKey::Header(
                    AString::try_from("A").unwrap(),
                    AString::try_from("B").unwrap(),
                ),
                b"HEADER A B",
            ),
            (
                SearchKey::Keyword(Atom::try_from("A").unwrap()),
                b"KEYWORD A",
            ),
            (SearchKey::Larger(42), b"LARGER 42"),
            (SearchKey::New, b"NEW"),
            (SearchKey::Not(Box::new(SearchKey::New)), b"NOT NEW"),
            (SearchKey::Old, b"OLD"),
            (
                SearchKey::On(
                    NaiveDate::try_from(chrono::NaiveDate::from_ymd_opt(2023, 4, 12).unwrap())
                        .unwrap(),
                ),
                b"ON \"12-Apr-2023\"",
            ),
            (
                SearchKey::Or(Box::new(SearchKey::New), Box::new(SearchKey::Recent)),
                b"OR NEW RECENT",
            ),
            (SearchKey::Recent, b"RECENT"),
            (SearchKey::Seen, b"SEEN"),
            (
                SearchKey::SentBefore(
                    NaiveDate::try_from(chrono::NaiveDate::from_ymd_opt(2023, 4, 12).unwrap())
                        .unwrap(),
                ),
                b"SENTBEFORE \"12-Apr-2023\"",
            ),
            (
                SearchKey::SentOn(
                    NaiveDate::try_from(chrono::NaiveDate::from_ymd_opt(2023, 4, 12).unwrap())
                        .unwrap(),
                ),
                b"SENTON \"12-Apr-2023\"",
            ),
            (
                SearchKey::SentSince(
                    NaiveDate::try_from(chrono::NaiveDate::from_ymd_opt(2023, 4, 12).unwrap())
                        .unwrap(),
                ),
                b"SENTSINCE \"12-Apr-2023\"",
            ),
            (
                SearchKey::Since(
                    NaiveDate::try_from(chrono::NaiveDate::from_ymd_opt(2023, 4, 12).unwrap())
                        .unwrap(),
                ),
                b"SINCE \"12-Apr-2023\"",
            ),
            (SearchKey::Smaller(1337), b"SMALLER 1337"),
            (
                SearchKey::Subject(AString::try_from("A").unwrap()),
                b"SUBJECT A",
            ),
            (SearchKey::Text(AString::try_from("A").unwrap()), b"TEXT A"),
            (SearchKey::To(AString::try_from("A").unwrap()), b"TO A"),
            (
                SearchKey::Uid(SequenceSet::from(Sequence::try_from(1..).unwrap())),
                b"UID 1:*",
            ),
            (SearchKey::Unanswered, b"UNANSWERED"),
            (SearchKey::Undeleted, b"UNDELETED"),
            (SearchKey::Undraft, b"UNDRAFT"),
            (SearchKey::Unflagged, b"UNFLAGGED"),
            (
                SearchKey::Unkeyword(Atom::try_from("A").unwrap()),
                b"UNKEYWORD A",
            ),
            (SearchKey::Unseen, b"UNSEEN"),
        ];

        for test in tests {
            known_answer_test_encode(test);
        }
    }
}
