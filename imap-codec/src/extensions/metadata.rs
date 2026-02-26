//! The IMAP METADATA Extension

use std::io::Write;

use abnf_core::streaming::sp;
use imap_types::{
    command::CommandBody,
    core::{NString8, Vec1},
    extensions::metadata::{
        Depth, Entry, EntryValue, GetMetadataOption, MetadataCode, MetadataResponse,
    },
    response::Data,
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, opt, value},
    error::ErrorKind,
    multi::separated_list1,
    sequence::{delimited, preceded, separated_pair, tuple},
};

use crate::{
    core::{astring, nstring, number},
    decode::{IMAPErrorKind, IMAPParseError, IMAPResult},
    encode::{EncodeContext, EncodeIntoContext, utils::join_serializable},
    extensions::binary::literal8,
    mailbox::mailbox,
};

// ----- Command -----

/// ```abnf
/// ; empty string for mailbox implies server annotation.
/// setmetadata  = "SETMETADATA" SP mailbox SP entry-values
/// ```
pub(crate) fn setmetadata(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case("SETMETADATA"),
        preceded(sp, mailbox),
        preceded(sp, entry_values),
    ));

    let (rem, (_, mailbox, entry_values)) = parser(input)?;

    Ok((
        rem,
        CommandBody::SetMetadata {
            mailbox,
            entry_values,
        },
    ))
}

/// ```abnf
/// entry-values = "(" entry-value *(SP entry-value) ")"
/// ```
pub(crate) fn entry_values(input: &[u8]) -> IMAPResult<&[u8], Vec1<EntryValue>> {
    map(
        delimited(tag("("), separated_list1(sp, entry_value), tag(")")),
        Vec1::unvalidated,
    )(input)
}

/// ```abnf
/// entry-value = entry SP value
/// ```
#[inline]
pub(crate) fn entry_value(input: &[u8]) -> IMAPResult<&[u8], EntryValue> {
    map(separated_pair(entry, sp, imap_value), |(entry, value)| {
        EntryValue { entry, value }
    })(input)
}

/// Slash-separated path to entry.
///
/// Note: MUST NOT contain "*" or "%".
///
/// ```abnf
/// entry = astring
/// ```
pub(crate) fn entry(input: &[u8]) -> IMAPResult<&[u8], Entry> {
    let (rem, astring) = astring(input)?;

    if let Ok(entry) = Entry::try_from(astring) {
        Ok((rem, entry))
    } else {
        Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::Nom(ErrorKind::Verify),
        }))
    }
}

/// ```abnf
/// value = nstring / literal8
/// ```
#[inline]
pub(crate) fn imap_value(input: &[u8]) -> IMAPResult<&[u8], NString8> {
    alt((
        map(nstring, NString8::NString),
        map(literal8, NString8::Literal8),
    ))(input)
}

/// ```abnf
/// getmetadata = "GETMETADATA" [SP getmetadata-options] SP mailbox SP entries
/// ```
///
/// Note: Empty string for mailbox implies server annotation.
pub(crate) fn getmetadata(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case("GETMETADATA"),
        opt(preceded(sp, getmetadata_options)),
        preceded(sp, mailbox),
        preceded(sp, entries),
    ));

    let (rem, (_, options, mailbox, entries)) = parser(input)?;

    Ok((
        rem,
        CommandBody::GetMetadata {
            options: options.map(|x| x.into_inner()).unwrap_or_default(),
            mailbox,
            entries,
        },
    ))
}

/// ```abnf
/// getmetadata-options = "(" getmetadata-option *(SP getmetadata-option) ")"
/// ```
pub(crate) fn getmetadata_options(input: &[u8]) -> IMAPResult<&[u8], Vec1<GetMetadataOption>> {
    map(
        delimited(tag("("), separated_list1(sp, getmetadata_option), tag(")")),
        Vec1::unvalidated,
    )(input)
}

/// ```abnf
/// ; tagged-ext-label and tagged-ext-val are defined in [RFC4466].
/// getmetadata-option = tagged-ext-label [SP tagged-ext-val]
///
/// ; Used as a getmetadata-option
/// maxsize-opt = "MAXSIZE" SP number
///
/// ; Used as a getmetadata-option
/// scope-opt = "DEPTH" SP ("0" / "1" / "infinity")
/// ```
pub(crate) fn getmetadata_option(input: &[u8]) -> IMAPResult<&[u8], GetMetadataOption> {
    // TODO: Generic parser required?
    alt((
        map(
            preceded(tag_no_case("MAXSIZE "), number),
            GetMetadataOption::MaxSize,
        ),
        map(
            preceded(
                tag_no_case("DEPTH "),
                alt((
                    value(Depth::Null, tag("0")),
                    value(Depth::One, tag("1")),
                    value(Depth::Infinity, tag_no_case("infinity")),
                )),
            ),
            GetMetadataOption::Depth,
        ),
    ))(input)
}

/// ```abnf
/// entries = entry / "(" entry *(SP entry) ")"
/// ```
pub(crate) fn entries(input: &[u8]) -> IMAPResult<&[u8], Vec1<Entry>> {
    alt((
        map(entry, Vec1::from),
        map(
            delimited(tag("("), separated_list1(sp, entry), tag(")")),
            Vec1::unvalidated,
        ),
    ))(input)
}

// ----- Response -----

/// ```abnf
/// response-payload =/ metadata-resp
///
/// ; empty string for mailbox implies server annotation.
/// metadata-resp = "METADATA" SP mailbox SP (entry-values / entry-list)
///
/// entry-list = entry *(SP entry)
/// ```
pub(crate) fn metadata_resp(input: &[u8]) -> IMAPResult<&[u8], Data> {
    let mut parser = tuple((
        tag_no_case("METADATA"),
        preceded(sp, mailbox),
        preceded(
            sp,
            alt((
                map(entry_values, MetadataResponse::WithValues),
                map(entry_list, MetadataResponse::WithoutValues),
            )),
        ),
    ));

    let (rem, (_, mailbox, items)) = parser(input)?;

    Ok((rem, Data::Metadata { mailbox, items }))
}

/// ```abnf
/// entry-list = entry *(SP entry)
/// ```
pub(crate) fn entry_list(input: &[u8]) -> IMAPResult<&[u8], Vec1<Entry>> {
    map(separated_list1(sp, entry), Vec1::unvalidated)(input)
}

impl EncodeIntoContext for MetadataResponse<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            MetadataResponse::WithValues(list) => {
                ctx.write_all(b"(")?;
                join_serializable(list.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")
            }
            MetadataResponse::WithoutValues(list) => join_serializable(list.as_ref(), b" ", ctx),
        }
    }
}

/// ```abnf
/// "LONGENTRIES" SP number /
/// "MAXSIZE" SP number /
/// "TOOMANY" /
/// "NOPRIVATE"
/// ```
pub(crate) fn metadata_code(input: &[u8]) -> IMAPResult<&[u8], MetadataCode> {
    alt((
        map(
            preceded(tag_no_case("LONGENTRIES "), number),
            MetadataCode::LongEntries,
        ),
        map(
            preceded(tag_no_case("MAXSIZE "), number),
            MetadataCode::MaxSize,
        ),
        value(MetadataCode::TooMany, tag_no_case("TOOMANY")),
        value(MetadataCode::NoPrivate, tag_no_case("NOPRIVATE")),
    ))(input)
}

impl EncodeIntoContext for MetadataCode {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            MetadataCode::LongEntries(number) => {
                ctx.write_all(b"LONGENTRIES ")?;
                number.encode_ctx(ctx)
            }
            MetadataCode::MaxSize(number) => {
                ctx.write_all(b"MAXSIZE ")?;
                number.encode_ctx(ctx)
            }
            MetadataCode::TooMany => ctx.write_all(b"TOOMANY"),
            MetadataCode::NoPrivate => ctx.write_all(b"NOPRIVATE"),
        }
    }
}

impl EncodeIntoContext for Entry<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.inner().encode_ctx(ctx)
    }
}

impl EncodeIntoContext for GetMetadataOption {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            GetMetadataOption::MaxSize(number) => {
                ctx.write_all(b"MAXSIZE ")?;
                number.encode_ctx(ctx)
            }
            GetMetadataOption::Depth(depth) => {
                ctx.write_all(b"DEPTH ")?;
                depth.encode_ctx(ctx)
            }
        }
    }
}

impl EncodeIntoContext for Depth {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(match self {
            Depth::Null => b"0",
            Depth::One => b"1",
            Depth::Infinity => b"INFINITY",
        })
    }
}

impl EncodeIntoContext for EntryValue<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.entry.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.value.encode_ctx(ctx)
    }
}

#[cfg(test)]
mod tests {
    use imap_types::{
        command::{Command, CommandBody},
        core::{AString, IString, Literal, LiteralMode, NString, NString8, Text, Vec1},
        extensions::{
            binary::Literal8,
            metadata::{
                Depth, Entry, EntryValue, GetMetadataOption, MetadataCode, MetadataResponse,
            },
        },
        mailbox::{Mailbox, MailboxOther},
        response::{Code, Data, Response, Status, StatusBody, StatusKind},
    };

    use crate::testing::{kat_inverse_command, kat_inverse_response};

    #[test]
    fn test_kat_inverse_command_setmetadata() {
        kat_inverse_command(&[
            (
                b"A SETMETADATA \"\" (/test nil)\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::SetMetadata {
                        mailbox: Mailbox::Other(MailboxOther::try_from("").unwrap()),
                        entry_values: Vec1::try_from(vec![EntryValue {
                            entry: Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                            value: NString8::NString(NString(None)),
                        }])
                        .unwrap(),
                    },
                )
                .unwrap(),
            ),
            (
                b"A SETMETADATA \"\" (/test \"test\")\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::SetMetadata {
                        mailbox: Mailbox::Other(MailboxOther::try_from("").unwrap()),
                        entry_values: Vec1::try_from(vec![EntryValue {
                            entry: Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                            value: NString8::NString(NString(Some(
                                IString::try_from("test").unwrap(),
                            ))),
                        }])
                        .unwrap(),
                    },
                )
                .unwrap(),
            ),
            (
                b"A SETMETADATA \"\" (/test ~{4+}\r\nt\x00st)\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::SetMetadata {
                        mailbox: Mailbox::Other(MailboxOther::try_from("").unwrap()),
                        entry_values: Vec1::try_from(vec![EntryValue {
                            entry: Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                            value: NString8::Literal8(Literal8 {
                                data: b"t\x00st".as_ref().into(),
                                mode: LiteralMode::NonSync,
                            }),
                        }])
                        .unwrap(),
                    },
                )
                .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_command_getmetadata() {
        kat_inverse_command(&[
            (
                b"A GETMETADATA \"\" /test\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::GetMetadata {
                        options: vec![],
                        mailbox: Mailbox::Other(MailboxOther::try_from("").unwrap()),
                        entries: Vec1::from(
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ),
            (
                b"A GETMETADATA INBOX /test\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::GetMetadata {
                        options: vec![],
                        mailbox: Mailbox::Inbox,
                        entries: Vec1::from(
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ),
            (
                b"A GETMETADATA (MAXSIZE 0) INBOX /test\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::GetMetadata {
                        options: vec![GetMetadataOption::MaxSize(0)],
                        mailbox: Mailbox::Inbox,
                        entries: Vec1::from(
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ),
            (
                b"A GETMETADATA (MAXSIZE 1337) INBOX /test\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::GetMetadata {
                        options: vec![GetMetadataOption::MaxSize(1337)],
                        mailbox: Mailbox::Inbox,
                        entries: Vec1::from(
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ),
            (
                b"A GETMETADATA (DEPTH 0) INBOX /test\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::GetMetadata {
                        options: vec![GetMetadataOption::Depth(Depth::Null)],
                        mailbox: Mailbox::Inbox,
                        entries: Vec1::from(
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ),
            (
                b"A GETMETADATA (DEPTH 1) INBOX /test\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::GetMetadata {
                        options: vec![GetMetadataOption::Depth(Depth::One)],
                        mailbox: Mailbox::Inbox,
                        entries: Vec1::from(
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ),
            (
                b"A GETMETADATA (DEPTH infinity) INBOX /test\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::GetMetadata {
                        options: vec![GetMetadataOption::Depth(Depth::Infinity)],
                        mailbox: Mailbox::Inbox,
                        entries: Vec1::from(
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_response_metadata() {
        kat_inverse_response(&[
            (
                b"* metadata INBOX /test /xxx\r\n".as_ref(),
                b"".as_ref(),
                Response::Data(Data::Metadata {
                    mailbox: Mailbox::Inbox,
                    items: MetadataResponse::WithoutValues(
                        Vec1::try_from(vec![
                            Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                            Entry::try_from(AString::try_from("/xxx").unwrap()).unwrap(),
                        ])
                        .unwrap(),
                    ),
                }),
            ),
            (
                b"* metadata INBOX (/test {4}\r\nABCD)\r\n".as_ref(),
                b"".as_ref(),
                Response::Data(Data::Metadata {
                    mailbox: Mailbox::Inbox,
                    items: MetadataResponse::WithValues(
                        Vec1::try_from(vec![EntryValue {
                            entry: Entry::try_from(AString::try_from("/test").unwrap()).unwrap(),
                            value: NString8::NString(NString(Some(IString::Literal(
                                Literal::try_from("ABCD").unwrap(),
                            )))),
                        }])
                        .unwrap(),
                    ),
                }),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_response_metadata_code() {
        kat_inverse_response(&[
            (
                b"* OK [metadata longentries 0] ...\r\n".as_ref(),
                b"".as_ref(),
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::Metadata(MetadataCode::LongEntries(0))),
                    text: Text::try_from("...").unwrap(),
                })),
            ),
            (
                b"* OK [metadata longentries 4294967295] ...\r\n".as_ref(),
                b"".as_ref(),
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::Metadata(MetadataCode::LongEntries(u32::MAX))),
                    text: Text::try_from("...").unwrap(),
                })),
            ),
            (
                b"* OK [metadatA maxSIZE 0] ...\r\n".as_ref(),
                b"".as_ref(),
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::Metadata(MetadataCode::MaxSize(0))),
                    text: Text::try_from("...").unwrap(),
                })),
            ),
            (
                b"* OK [metadata maxSizE 4294967295] ...\r\n".as_ref(),
                b"".as_ref(),
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::Metadata(MetadataCode::MaxSize(u32::MAX))),
                    text: Text::try_from("...").unwrap(),
                })),
            ),
            (
                b"* OK [metadata toOmaNy] ...\r\n".as_ref(),
                b"".as_ref(),
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::Metadata(MetadataCode::TooMany)),
                    text: Text::try_from("...").unwrap(),
                })),
            ),
            (
                b"* OK [mEtadata noPRIvaTE] ...\r\n".as_ref(),
                b"".as_ref(),
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::Metadata(MetadataCode::NoPrivate)),
                    text: Text::try_from("...").unwrap(),
                })),
            ),
        ]);
    }
}
