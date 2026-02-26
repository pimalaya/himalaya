//! IMAP4 ID extension

// Additional changes:
//
// command_any ::= "CAPABILITY" / "LOGOUT" / "NOOP" / x_command / id
// response_data ::= "*" SPACE (resp_cond_state / resp_cond_bye / mailbox_data / message_data / capability_data / id_response)

use abnf_core::streaming::sp;
use imap_types::core::{IString, NString};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, value},
    multi::separated_list0,
    sequence::{delimited, preceded, separated_pair},
};
#[cfg(feature = "quirk_trailing_space_id")]
use nom::{combinator::opt, sequence::terminated};

use crate::{
    core::{nil, nstring, string},
    decode::IMAPResult,
};

/// ```abnf
/// id = "ID" SPACE id_params_list
/// ```
///
/// Note: Updated ABNF.
#[allow(clippy::type_complexity)]
pub(crate) fn id(input: &[u8]) -> IMAPResult<&[u8], Option<Vec<(IString, NString)>>> {
    preceded(tag_no_case("ID "), id_params_list)(input)
}

/// ```abnf
/// id_response = "ID" SPACE id_params_list
/// ```
///
/// Note: Updated ABNF.
#[inline]
#[allow(clippy::type_complexity)]
pub(crate) fn id_response(input: &[u8]) -> IMAPResult<&[u8], Option<Vec<(IString, NString)>>> {
    id(input)
}

/// ```abnf
/// id-params-list = "(" [string SP nstring *(SP string SP nstring)] ")" / nil
/// ```
///
/// Note: Updated ABNF. (See <https://github.com/modern-email/defects/issues/12>)
#[allow(clippy::type_complexity)]
pub(crate) fn id_params_list(input: &[u8]) -> IMAPResult<&[u8], Option<Vec<(IString, NString)>>> {
    alt((
        map(
            #[cfg(not(feature = "quirk_trailing_space_id"))]
            delimited(
                tag("("),
                separated_list0(sp, separated_pair(string, sp, nstring)),
                tag(")"),
            ),
            #[cfg(feature = "quirk_trailing_space_id")]
            delimited(
                tag("("),
                terminated(
                    separated_list0(sp, separated_pair(string, sp, nstring)),
                    opt(sp),
                ),
                tag(")"),
            ),
            Some,
        ),
        value(None, nil),
    ))(input)
}

#[cfg(test)]
mod tests {
    use imap_types::{
        command::{Command, CommandBody},
        core::{IString, NString},
        response::{Data, Response},
    };

    use super::*;
    use crate::testing::{kat_inverse_command, kat_inverse_response};

    #[test]
    fn test_parse_id() {
        let tests = [
            (
                b"id (\"name\" \"imap-codec\")\r\n".as_ref(),
                Some(vec![(
                    IString::try_from("name").unwrap(),
                    NString::try_from("imap-codec").unwrap(),
                )]),
            ),
            #[cfg(feature = "quirk_trailing_space_id")]
            (
                b"id (\"name\" \"imap-codec\" )\r\n".as_ref(),
                Some(vec![(
                    IString::try_from("name").unwrap(),
                    NString::try_from("imap-codec").unwrap(),
                )]),
            ),
        ];

        for (test, expected) in tests {
            let got = id(test).unwrap().1;
            assert_eq!(expected, got,);
        }
    }

    #[test]
    fn test_kat_inverse_command_id() {
        kat_inverse_command(&[
            (
                b"A ID nil\r\n".as_ref(),
                b"".as_ref(),
                Command::new("A", CommandBody::Id { parameters: None }).unwrap(),
            ),
            (
                b"A ID NIL\r\n".as_ref(),
                b"".as_ref(),
                Command::new("A", CommandBody::Id { parameters: None }).unwrap(),
            ),
            #[cfg(not(feature = "quirk_id_empty_to_nil"))]
            (
                b"A ID ()\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::Id {
                        parameters: Some(vec![]),
                    },
                )
                .unwrap(),
            ),
            (
                b"A ID (\"\" \"\")\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::Id {
                        parameters: Some(vec![(
                            IString::try_from("").unwrap(),
                            NString::try_from("").unwrap(),
                        )]),
                    },
                )
                .unwrap(),
            ),
            (
                b"A ID (\"name\" \"imap-codec\")\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::Id {
                        parameters: Some(vec![(
                            IString::try_from("name").unwrap(),
                            NString::try_from("imap-codec").unwrap(),
                        )]),
                    },
                )
                .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_response_id() {
        kat_inverse_response(&[(
            b"* ID nil\r\n".as_ref(),
            b"".as_ref(),
            Response::Data(Data::Id { parameters: None }),
        )]);
    }
}
