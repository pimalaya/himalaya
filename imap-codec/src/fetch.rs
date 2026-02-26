use std::num::NonZeroU32;

use abnf_core::streaming::sp;
use imap_types::{
    core::{AString, NString8, Vec1},
    fetch::{MessageDataItem, MessageDataItemName, Part, PartSpecifier, Section},
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    character::streaming::char,
    combinator::{map, opt, value},
    multi::separated_list1,
    sequence::{delimited, preceded, tuple},
};

#[cfg(feature = "ext_condstore_qresync")]
use crate::extensions::condstore_qresync::mod_sequence_value;
use crate::{
    body::body,
    core::{astring, nstring, number, nz_number},
    datetime::date_time,
    decode::IMAPResult,
    envelope::envelope,
    extensions::binary::{literal8, partial, section_binary},
    flag::flag_fetch,
};

/// ```abnf
/// fetch-att = "ENVELOPE" /
///             "FLAGS" /
///             "INTERNALDATE" /
///             "RFC822" [".HEADER" / ".SIZE" / ".TEXT"] /
///             "BODY" ["STRUCTURE"] /
///             "UID" /
///             "BODY"      section ["<" number "." nz-number ">"] /
///             "BODY.PEEK" section ["<" number "." nz-number ">"] /
///             "BINARY"      section-binary [partial] / ; RFC 3516
///             "BINARY.PEEK" section-binary [partial] / ; RFC 3516
///             "BINARY.SIZE" section-binary           / ; RFC 3516
///             "MODSEQ"                                 ; RFC 7162
/// ```
pub(crate) fn fetch_att(input: &[u8]) -> IMAPResult<&[u8], MessageDataItemName> {
    alt((
        value(MessageDataItemName::Envelope, tag_no_case(b"ENVELOPE")),
        value(MessageDataItemName::Flags, tag_no_case(b"FLAGS")),
        value(
            MessageDataItemName::InternalDate,
            tag_no_case(b"INTERNALDATE"),
        ),
        value(
            MessageDataItemName::BodyStructure,
            tag_no_case(b"BODYSTRUCTURE"),
        ),
        map(
            tuple((
                tag_no_case(b"BODY.PEEK"),
                section,
                opt(delimited(
                    tag(b"<"),
                    tuple((number, tag(b"."), nz_number)),
                    tag(b">"),
                )),
            )),
            |(_, section, byterange)| MessageDataItemName::BodyExt {
                section,
                partial: byterange.map(|(start, _, end)| (start, end)),
                peek: true,
            },
        ),
        map(
            tuple((
                tag_no_case(b"BODY"),
                section,
                opt(delimited(
                    tag(b"<"),
                    tuple((number, tag(b"."), nz_number)),
                    tag(b">"),
                )),
            )),
            |(_, section, byterange)| MessageDataItemName::BodyExt {
                section,
                partial: byterange.map(|(start, _, end)| (start, end)),
                peek: false,
            },
        ),
        map(
            tuple((tag_no_case("BINARY.PEEK"), section_binary, opt(partial))),
            |(_, section, partial)| MessageDataItemName::Binary {
                section,
                partial,
                peek: true,
            },
        ),
        map(
            tuple((tag_no_case("BINARY"), section_binary, opt(partial))),
            |(_, section, partial)| MessageDataItemName::Binary {
                section,
                partial,
                peek: false,
            },
        ),
        map(
            preceded(tag_no_case("BINARY.SIZE"), section_binary),
            |section| MessageDataItemName::BinarySize { section },
        ),
        value(MessageDataItemName::Body, tag_no_case(b"BODY")),
        value(MessageDataItemName::Uid, tag_no_case(b"UID")),
        value(
            MessageDataItemName::Rfc822Header,
            tag_no_case(b"RFC822.HEADER"),
        ),
        value(MessageDataItemName::Rfc822Size, tag_no_case(b"RFC822.SIZE")),
        value(MessageDataItemName::Rfc822Text, tag_no_case(b"RFC822.TEXT")),
        value(MessageDataItemName::Rfc822, tag_no_case(b"RFC822")),
        #[cfg(feature = "ext_condstore_qresync")]
        value(MessageDataItemName::ModSeq, tag_no_case(b"MODSEQ")),
    ))(input)
}

/// ```abnf
/// msg-att = "("
///           (msg-att-dynamic / msg-att-static) *(SP (msg-att-dynamic / msg-att-static))
///           ")"
/// ```
pub(crate) fn msg_att(input: &[u8]) -> IMAPResult<&[u8], Vec1<MessageDataItem>> {
    delimited(
        tag(b"("),
        map(
            separated_list1(sp, alt((msg_att_dynamic, msg_att_static))),
            Vec1::unvalidated,
        ),
        tag(b")"),
    )(input)
}

/// ```abnf
/// msg-att-dynamic = "FLAGS" SP "(" [flag-fetch *(SP flag-fetch)] ")"
/// ```
///
/// Note: MAY change for a message
pub(crate) fn msg_att_dynamic(input: &[u8]) -> IMAPResult<&[u8], MessageDataItem> {
    let flags = map(
        preceded(
            tag_no_case(b"FLAGS "),
            delimited(char('('), opt(separated_list1(sp, flag_fetch)), char(')')),
        ),
        |flags| MessageDataItem::Flags(flags.unwrap_or_default()),
    );
    #[cfg(feature = "ext_condstore_qresync")]
    let modseq = map(
        preceded(
            tag_no_case("MODSEQ "),
            delimited(char('('), mod_sequence_value, char(')')),
        ),
        MessageDataItem::ModSeq,
    );

    #[cfg(feature = "ext_condstore_qresync")]
    let mut parser = alt((flags, modseq));

    #[cfg(not(feature = "ext_condstore_qresync"))]
    let mut parser = flags;

    let (remaining, item) = parser(input)?;

    Ok((remaining, item))
}

/// ```abnf
/// msg-att-static = "ENVELOPE" SP envelope /
///                  "INTERNALDATE" SP date-time /
///                  "RFC822" [".HEADER" / ".TEXT"] SP nstring /
///                  "RFC822.SIZE" SP number /
///                  "BODY" ["STRUCTURE"] SP body /
///                  "BODY" section ["<" number ">"] SP nstring /
///                  "UID" SP uniqueid /
///                  "BINARY" section-binary SP (nstring / literal8) / ; RFC 3516
///                  "BINARY.SIZE" section-binary SP number            ; RFC 3516
/// ```
///
/// Note: MUST NOT change for a message
pub(crate) fn msg_att_static(input: &[u8]) -> IMAPResult<&[u8], MessageDataItem> {
    alt((
        map(
            preceded(tag_no_case(b"ENVELOPE "), envelope),
            MessageDataItem::Envelope,
        ),
        map(
            preceded(tag_no_case(b"INTERNALDATE "), date_time),
            MessageDataItem::InternalDate,
        ),
        map(
            preceded(tag_no_case(b"RFC822.HEADER "), nstring),
            MessageDataItem::Rfc822Header,
        ),
        map(
            preceded(tag_no_case(b"RFC822.TEXT "), nstring),
            MessageDataItem::Rfc822Text,
        ),
        map(
            preceded(tag_no_case(b"RFC822.SIZE "), number),
            MessageDataItem::Rfc822Size,
        ),
        map(
            preceded(tag_no_case(b"RFC822 "), nstring),
            MessageDataItem::Rfc822,
        ),
        map(
            preceded(tag_no_case(b"BODYSTRUCTURE "), body(8)),
            MessageDataItem::BodyStructure,
        ),
        map(
            preceded(tag_no_case(b"BODY "), body(8)),
            MessageDataItem::Body,
        ),
        map(
            tuple((
                tag_no_case(b"BODY"),
                section,
                opt(delimited(tag(b"<"), number, tag(b">"))),
                sp,
                nstring,
            )),
            |(_, section, origin, _, data)| MessageDataItem::BodyExt {
                section,
                origin,
                data,
            },
        ),
        map(
            preceded(tag_no_case(b"UID "), uniqueid),
            MessageDataItem::Uid,
        ),
        map(
            tuple((
                tag_no_case(b"BINARY"),
                section_binary,
                sp,
                alt((
                    map(nstring, NString8::NString),
                    map(literal8, NString8::Literal8),
                )),
            )),
            |(_, section, _, value)| MessageDataItem::Binary { section, value },
        ),
        map(
            tuple((tag_no_case(b"BINARY.SIZE"), section_binary, sp, number)),
            |(_, section, _, size)| MessageDataItem::BinarySize { section, size },
        ),
    ))(input)
}

#[inline]
/// `uniqueid = nz-number`
///
/// Note: Strictly ascending
pub(crate) fn uniqueid(input: &[u8]) -> IMAPResult<&[u8], NonZeroU32> {
    nz_number(input)
}

/// `section = "[" [section-spec] "]"`
pub(crate) fn section(input: &[u8]) -> IMAPResult<&[u8], Option<Section>> {
    delimited(tag(b"["), opt(section_spec), tag(b"]"))(input)
}

/// `section-spec = section-msgtext / (section-part ["." section-text])`
pub(crate) fn section_spec(input: &[u8]) -> IMAPResult<&[u8], Section> {
    alt((
        map(section_msgtext, |part_specifier| match part_specifier {
            PartSpecifier::PartNumber(_) => unreachable!(),
            PartSpecifier::Header => Section::Header(None),
            PartSpecifier::HeaderFields(fields) => Section::HeaderFields(None, fields),
            PartSpecifier::HeaderFieldsNot(fields) => Section::HeaderFieldsNot(None, fields),
            PartSpecifier::Text => Section::Text(None),
            PartSpecifier::Mime => unreachable!(),
        }),
        map(
            tuple((section_part, opt(tuple((tag(b"."), section_text))))),
            |(part_number, maybe_part_specifier)| {
                if let Some((_, part_specifier)) = maybe_part_specifier {
                    match part_specifier {
                        PartSpecifier::PartNumber(_) => unreachable!(),
                        PartSpecifier::Header => Section::Header(Some(Part(part_number))),
                        PartSpecifier::HeaderFields(fields) => {
                            Section::HeaderFields(Some(Part(part_number)), fields)
                        }
                        PartSpecifier::HeaderFieldsNot(fields) => {
                            Section::HeaderFieldsNot(Some(Part(part_number)), fields)
                        }
                        PartSpecifier::Text => Section::Text(Some(Part(part_number))),
                        PartSpecifier::Mime => Section::Mime(Part(part_number)),
                    }
                } else {
                    Section::Part(Part(part_number))
                }
            },
        ),
    ))(input)
}

/// `section-msgtext = "HEADER" / "HEADER.FIELDS" [".NOT"] SP header-list / "TEXT"`
///
/// Top-level or MESSAGE/RFC822 part
pub(crate) fn section_msgtext(input: &[u8]) -> IMAPResult<&[u8], PartSpecifier> {
    alt((
        map(
            tuple((tag_no_case(b"HEADER.FIELDS.NOT"), sp, header_list)),
            |(_, _, header_list)| PartSpecifier::HeaderFieldsNot(header_list),
        ),
        map(
            tuple((tag_no_case(b"HEADER.FIELDS"), sp, header_list)),
            |(_, _, header_list)| PartSpecifier::HeaderFields(header_list),
        ),
        value(PartSpecifier::Header, tag_no_case(b"HEADER")),
        value(PartSpecifier::Text, tag_no_case(b"TEXT")),
    ))(input)
}

#[inline]
/// `section-part = nz-number *("." nz-number)`
///
/// Body part nesting
pub(crate) fn section_part(input: &[u8]) -> IMAPResult<&[u8], Vec1<NonZeroU32>> {
    map(separated_list1(tag(b"."), nz_number), Vec1::unvalidated)(input)
}

/// `section-text = section-msgtext / "MIME"`
///
/// Text other than actual body part (headers, etc.)
pub(crate) fn section_text(input: &[u8]) -> IMAPResult<&[u8], PartSpecifier> {
    alt((
        section_msgtext,
        value(PartSpecifier::Mime, tag_no_case(b"MIME")),
    ))(input)
}

/// `header-list = "(" header-fld-name *(SP header-fld-name) ")"`
pub(crate) fn header_list(input: &[u8]) -> IMAPResult<&[u8], Vec1<AString>> {
    map(
        delimited(tag(b"("), separated_list1(sp, header_fld_name), tag(b")")),
        Vec1::unvalidated,
    )(input)
}

#[inline]
/// `header-fld-name = astring`
pub(crate) fn header_fld_name(input: &[u8]) -> IMAPResult<&[u8], AString> {
    astring(input)
}

#[cfg(test)]
mod tests {
    use imap_types::{
        body::{BasicFields, Body, BodyStructure, SpecificFields},
        core::{IString, NString},
        datetime::DateTime,
        envelope::Envelope,
    };

    use super::*;
    use crate::testing::known_answer_test_encode;

    #[test]
    fn test_encode_message_data_item_name() {
        let tests = [
            (MessageDataItemName::Body, b"BODY".as_ref()),
            (
                MessageDataItemName::BodyExt {
                    section: None,
                    partial: None,
                    peek: false,
                },
                b"BODY[]",
            ),
            (MessageDataItemName::BodyStructure, b"BODYSTRUCTURE"),
            (MessageDataItemName::Envelope, b"ENVELOPE"),
            (MessageDataItemName::Flags, b"FLAGS"),
            (MessageDataItemName::InternalDate, b"INTERNALDATE"),
            (MessageDataItemName::Rfc822, b"RFC822"),
            (MessageDataItemName::Rfc822Header, b"RFC822.HEADER"),
            (MessageDataItemName::Rfc822Size, b"RFC822.SIZE"),
            (MessageDataItemName::Rfc822Text, b"RFC822.TEXT"),
            (MessageDataItemName::Uid, b"UID"),
        ];

        for test in tests {
            known_answer_test_encode(test);
        }
    }

    #[test]
    fn test_encode_message_data_item() {
        let tests = [
            (
                MessageDataItem::Body(BodyStructure::Single {
                    body: Body {
                        basic: BasicFields {
                            parameter_list: vec![],
                            id: NString(None),
                            description: NString(None),
                            content_transfer_encoding: IString::try_from("base64").unwrap(),
                            size: 42,
                        },
                        specific: SpecificFields::Text {
                            subtype: IString::try_from("foo").unwrap(),
                            number_of_lines: 1337,
                        },
                    },
                    extension_data: None,
                }),
                b"BODY (\"TEXT\" \"foo\" NIL NIL NIL \"base64\" 42 1337)".as_ref(),
            ),
            (
                MessageDataItem::BodyExt {
                    section: None,
                    origin: None,
                    data: NString(None),
                },
                b"BODY[] NIL",
            ),
            (
                MessageDataItem::BodyExt {
                    section: None,
                    origin: Some(123),
                    data: NString(None),
                },
                b"BODY[]<123> NIL",
            ),
            (
                MessageDataItem::BodyStructure(BodyStructure::Single {
                    body: Body {
                        basic: BasicFields {
                            parameter_list: vec![],
                            id: NString(None),
                            description: NString(None),
                            content_transfer_encoding: IString::try_from("base64").unwrap(),
                            size: 213,
                        },
                        specific: SpecificFields::Text {
                            subtype: IString::try_from("").unwrap(),
                            number_of_lines: 224,
                        },
                    },
                    extension_data: None,
                }),
                b"BODYSTRUCTURE (\"TEXT\" \"\" NIL NIL NIL \"base64\" 213 224)",
            ),
            (
                MessageDataItem::Envelope(Envelope {
                    date: NString(None),
                    subject: NString(None),
                    from: vec![],
                    sender: vec![],
                    reply_to: vec![],
                    to: vec![],
                    cc: vec![],
                    bcc: vec![],
                    in_reply_to: NString(None),
                    message_id: NString(None),
                }),
                b"ENVELOPE (NIL NIL NIL NIL NIL NIL NIL NIL NIL NIL)",
            ),
            (MessageDataItem::Flags(vec![]), b"FLAGS ()"),
            (
                MessageDataItem::InternalDate(
                    DateTime::try_from(
                        chrono::DateTime::parse_from_rfc2822("Tue, 1 Jul 2003 10:52:37 +0200")
                            .unwrap(),
                    )
                    .unwrap(),
                ),
                b"INTERNALDATE \"01-Jul-2003 10:52:37 +0200\"",
            ),
            (MessageDataItem::Rfc822(NString(None)), b"RFC822 NIL"),
            (
                MessageDataItem::Rfc822Header(NString(None)),
                b"RFC822.HEADER NIL",
            ),
            (MessageDataItem::Rfc822Size(3456), b"RFC822.SIZE 3456"),
            (
                MessageDataItem::Rfc822Text(NString(None)),
                b"RFC822.TEXT NIL",
            ),
            (
                MessageDataItem::Uid(NonZeroU32::try_from(u32::MAX).unwrap()),
                b"UID 4294967295",
            ),
        ];

        for test in tests {
            known_answer_test_encode(test);
        }
    }

    #[test]
    fn test_encode_section() {
        let tests = [
            (
                Section::Part(Part(Vec1::from(NonZeroU32::try_from(1).unwrap()))),
                b"1".as_ref(),
            ),
            (Section::Header(None), b"HEADER"),
            (
                Section::Header(Some(Part(Vec1::from(NonZeroU32::try_from(1).unwrap())))),
                b"1.HEADER",
            ),
            (
                Section::HeaderFields(None, Vec1::from(AString::try_from("").unwrap())),
                b"HEADER.FIELDS (\"\")",
            ),
            (
                Section::HeaderFields(
                    Some(Part(Vec1::from(NonZeroU32::try_from(1).unwrap()))),
                    Vec1::from(AString::try_from("").unwrap()),
                ),
                b"1.HEADER.FIELDS (\"\")",
            ),
            (
                Section::HeaderFieldsNot(None, Vec1::from(AString::try_from("").unwrap())),
                b"HEADER.FIELDS.NOT (\"\")",
            ),
            (
                Section::HeaderFieldsNot(
                    Some(Part(Vec1::from(NonZeroU32::try_from(1).unwrap()))),
                    Vec1::from(AString::try_from("").unwrap()),
                ),
                b"1.HEADER.FIELDS.NOT (\"\")",
            ),
            (Section::Text(None), b"TEXT"),
            (
                Section::Text(Some(Part(Vec1::from(NonZeroU32::try_from(1).unwrap())))),
                b"1.TEXT",
            ),
            (
                Section::Mime(Part(Vec1::from(NonZeroU32::try_from(1).unwrap()))),
                b"1.MIME",
            ),
        ];

        for test in tests {
            known_answer_test_encode(test)
        }
    }
}
