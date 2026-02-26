use std::{io::Write, num::NonZeroU32};

use abnf_core::streaming::sp;
use imap_types::{
    command::CommandBody,
    core::Vec1,
    extensions::uidplus::{UidElement, UidElement::Range, UidSet},
    response::Code,
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::map,
    multi::separated_list1,
    sequence::{preceded, separated_pair, tuple},
};

use crate::{
    core::nz_number,
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext, utils::join_serializable},
    sequence::sequence_set,
};

/// ```abnf
/// uid-expunge = "UID" SP "EXPUNGE" SP sequence-set
/// ```
pub(crate) fn uid_expunge(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    map(
        preceded(tag_no_case("UID EXPUNGE "), sequence_set),
        |sequence_set| CommandBody::ExpungeUid { sequence_set },
    )(input)
}

/// ```abnf
/// resp-code-apnd = "APPENDUID" SP nz-number SP append-uid
///
/// append-uid = uniqueid
///
/// uniqueid = nz-number
/// ```
pub(crate) fn resp_code_apnd(input: &[u8]) -> IMAPResult<&[u8], Code> {
    let (rem, (_, uid_validity, _, uid)) =
        tuple((tag_no_case("APPENDUID "), nz_number, sp, nz_number))(input)?;

    Ok((rem, Code::AppendUid { uid_validity, uid }))
}

/// ```abnf
/// resp-code-copy = "COPYUID" SP nz-number SP uid-set SP uid-set
/// ```
pub(crate) fn resp_code_copy(input: &[u8]) -> IMAPResult<&[u8], Code> {
    let (rem, (_, uid_validity, _, source, _, destination)) =
        tuple((tag_no_case("COPYUID "), nz_number, sp, uid_set, sp, uid_set))(input)?;

    Ok((
        rem,
        Code::CopyUid {
            uid_validity,
            source,
            destination,
        },
    ))
}

/// ```abnf
/// uid-set = (uniqueid / uid-range) *("," uid-set)
/// ```
///
/// Modified ...
///
/// ```abnf
/// uid-set = (uniqueid / uid-range) *("," uniqueid / uid-range)
/// ```
pub(crate) fn uid_set(input: &[u8]) -> IMAPResult<&[u8], UidSet> {
    map(
        separated_list1(
            tag(b","),
            alt((
                map(uid_range, |(a, b)| UidElement::Range(a, b)),
                map(nz_number, UidElement::Single),
            )),
        ),
        // `unvalidated` is fine due to `separated_list1`
        |set| UidSet(Vec1::unvalidated(set)),
    )(input)
}

/// ```abnf
/// ; two uniqueid values and all values between these two regards of order.
/// ; Example: 2:4 and 4:2 are equivalent.
/// uid-range = (uniqueid ":" uniqueid)
/// ```
pub(crate) fn uid_range(input: &[u8]) -> IMAPResult<&[u8], (NonZeroU32, NonZeroU32)> {
    separated_pair(nz_number, tag(b":"), nz_number)(input)
}

impl EncodeIntoContext for UidSet {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        join_serializable(self.0.as_ref(), b",", ctx)
    }
}

impl EncodeIntoContext for UidElement {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            UidElement::Single(uid) => uid.encode_ctx(ctx),
            Range(start, end) => {
                start.encode_ctx(ctx)?;
                ctx.write_all(b":")?;
                end.encode_ctx(ctx)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use imap_types::{
        command::{Command, CommandBody},
        core::{Text, Vec1},
        extensions::uidplus::{UidElement, UidSet},
        response::{Code, Response, Status, StatusBody, StatusKind},
    };

    use crate::{
        extensions::uidplus::uid_set,
        testing::{kat_inverse_command, kat_inverse_response, known_answer_test_parse},
    };

    #[test]
    fn test_kat_inverse_command_uid_expunge() {
        kat_inverse_command(&[
            (
                b"A UID EXPUNGE 1\r\n?".as_ref(),
                b"?".as_ref(),
                Command::new(
                    "A",
                    CommandBody::ExpungeUid {
                        sequence_set: 1.try_into().unwrap(),
                    },
                )
                .unwrap(),
            ),
            (
                b"A UID EXPUNGE *\r\n?".as_ref(),
                b"?".as_ref(),
                Command::new(
                    "A",
                    CommandBody::ExpungeUid {
                        sequence_set: "*".try_into().unwrap(),
                    },
                )
                .unwrap(),
            ),
            (
                b"A UID EXPUNGE 1:1337\r\n?".as_ref(),
                b"?".as_ref(),
                Command::new(
                    "A",
                    CommandBody::ExpungeUid {
                        sequence_set: "1:1337".try_into().unwrap(),
                    },
                )
                .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_response() {
        kat_inverse_response(&[
            (
                b"* OK [UIDNOTSTICKY] ...\r\n???",
                b"???",
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::UidNotSticky),
                    text: Text::unvalidated("..."),
                })),
            ),
            (
                b"* OK [APPENDUID 12345 1337] ...\r\n???",
                b"???",
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::AppendUid {
                        uid_validity: 12345.try_into().unwrap(),
                        uid: 1337.try_into().unwrap(),
                    }),
                    text: Text::unvalidated("..."),
                })),
            ),
            (
                b"* OK [COPYUID 12345 1001:1005 1:5] ...\r\n???",
                b"???",
                Response::Status(Status::Untagged(StatusBody {
                    kind: StatusKind::Ok,
                    code: Some(Code::CopyUid {
                        uid_validity: 12345.try_into().unwrap(),
                        source: UidSet(Vec1::from(UidElement::Range(
                            1001.try_into().unwrap(),
                            1005.try_into().unwrap(),
                        ))),
                        destination: UidSet(Vec1::from(UidElement::Range(
                            1.try_into().unwrap(),
                            5.try_into().unwrap(),
                        ))),
                    }),
                    text: Text::unvalidated("..."),
                })),
            ),
        ]);
    }

    #[test]
    fn test_uid_set() {
        let tests = [
            // (
            //     b"1 ".as_ref(),
            //     b" ".as_ref(),
            //     UidSet(Vec1::from(UidElement::Single(1.try_into().unwrap()))),
            // ),
            (
                b"1:5 ".as_ref(),
                b" ".as_ref(),
                UidSet(Vec1::from(UidElement::Range(
                    1.try_into().unwrap(),
                    5.try_into().unwrap(),
                ))),
            ),
        ];

        for test in tests {
            known_answer_test_parse(test, uid_set);
        }
    }
}
