use std::io::Write;

use abnf_core::streaming::sp;
use imap_types::{
    command::CommandBody,
    core::Vec1,
    extensions::sort::{SortCriterion, SortKey},
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, opt, value},
    multi::separated_list1,
    sequence::{delimited, tuple},
};

use crate::{
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext},
    search::search_criteria,
};

/// ```abnf
/// sort = ["UID" SP] "SORT" SP sort-criteria SP search-criteria
/// ```
pub(crate) fn sort(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        map(opt(tag_no_case("UID ")), |thing| thing.is_some()),
        tag_no_case("SORT "),
        sort_criteria,
        sp,
        search_criteria,
    ));

    let (remaining, (uid, _, sort_criteria, _, (charset, search_key))) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Sort {
            sort_criteria,
            charset,
            search_criteria: search_key,
            uid,
        },
    ))
}

/// ```abnf
/// sort-criteria = "(" sort-criterion *(SP sort-criterion) ")"
/// ```
pub(crate) fn sort_criteria(input: &[u8]) -> IMAPResult<&[u8], Vec1<SortCriterion>> {
    delimited(
        tag("("),
        map(separated_list1(sp, sort_criterion), Vec1::unvalidated),
        tag(")"),
    )(input)
}

/// ```abnf
/// sort-criterion = ["REVERSE" SP] sort-key
/// ```
pub(crate) fn sort_criterion(input: &[u8]) -> IMAPResult<&[u8], SortCriterion> {
    let mut parser = tuple((
        map(opt(tag_no_case(b"REVERSE ")), |thing| thing.is_some()),
        sort_key,
    ));

    let (remaining, (reverse, key)) = parser(input)?;

    Ok((remaining, SortCriterion { reverse, key }))
}

/// ```abnf
/// sort-key = "ARRIVAL" / "CC" / "DATE" / "FROM" / "SIZE" / "SUBJECT" / "TO"
/// ```
pub(crate) fn sort_key(input: &[u8]) -> IMAPResult<&[u8], SortKey> {
    alt((
        value(SortKey::Arrival, tag_no_case("ARRIVAL")),
        value(SortKey::Cc, tag_no_case("CC")),
        value(SortKey::Date, tag_no_case("DATE")),
        value(SortKey::From, tag_no_case("FROM")),
        value(SortKey::Size, tag_no_case("SIZE")),
        value(SortKey::Subject, tag_no_case("SUBJECT")),
        value(SortKey::To, tag_no_case("TO")),
        value(SortKey::DisplayFrom, tag_no_case("DISPLAYFROM")),
        value(SortKey::DisplayTo, tag_no_case("DISPLAYTO")),
    ))(input)
}

impl EncodeIntoContext for SortCriterion {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        if self.reverse {
            ctx.write_all(b"REVERSE ")?;
        }

        ctx.write_all(self.key.as_ref().as_bytes())
    }
}
