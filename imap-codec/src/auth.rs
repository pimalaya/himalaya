#[cfg(not(feature = "quirk_crlf_relaxed"))]
use abnf_core::streaming::crlf;
#[cfg(feature = "quirk_crlf_relaxed")]
use abnf_core::streaming::crlf_relaxed as crlf;
use imap_types::auth::{AuthMechanism, AuthenticateData};
use nom::{
    branch::alt,
    bytes::streaming::tag,
    combinator::{map, value},
    sequence::{terminated, tuple},
};

use crate::{
    core::{atom, base64},
    decode::IMAPResult,
};

// ----- Unsorted IMAP parsers -----

/// `auth-type = atom`
///
/// Note: Defined by \[SASL\]
pub(crate) fn auth_type(input: &[u8]) -> IMAPResult<&[u8], AuthMechanism> {
    let (rem, atom) = atom(input)?;

    Ok((rem, AuthMechanism::from(atom)))
}

/// `authenticate = "AUTHENTICATE" SP auth-type *(CRLF base64)` (edited)
///
/// ```text
/// authenticate = base64 CRLF
///                vvvvvvvvvvvv
///                |
///                This is parsed here.
///                CRLF is additionally parsed in this parser.
///                FIXME: Multiline base64 currently does not work.
/// ```
pub(crate) fn authenticate_data(input: &[u8]) -> IMAPResult<&[u8], AuthenticateData> {
    alt((
        map(terminated(base64, crlf), AuthenticateData::r#continue),
        value(AuthenticateData::Cancel, tuple((tag("*"), crlf))),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{known_answer_test_encode, known_answer_test_parse};

    #[test]
    fn test_encode_auth_mechanism() {
        let tests = [
            (AuthMechanism::Plain, b"PLAIN".as_ref()),
            (AuthMechanism::Login, b"LOGIN"),
            (AuthMechanism::OAuthBearer, b"OAUTHBEARER"),
            (AuthMechanism::try_from("PLAINX").unwrap(), b"PLAINX"),
            (AuthMechanism::try_from("LOGINX").unwrap(), b"LOGINX"),
            (AuthMechanism::try_from("XOAUTH2X").unwrap(), b"XOAUTH2X"),
        ];

        for test in tests {
            known_answer_test_encode(test);
        }
    }

    #[test]
    fn test_parse_auth_type() {
        let tests = [
            (b"plain ".as_ref(), b" ".as_ref(), AuthMechanism::Plain),
            (b"pLaiN ", b" ", AuthMechanism::Plain),
            (b"lOgiN ", b" ", AuthMechanism::Login),
            (b"login ", b" ", AuthMechanism::Login),
            (b"loginX ", b" ", AuthMechanism::try_from("loginX").unwrap()),
            (
                b"loginX ",
                b" ",
                AuthMechanism::try_from(b"loginX".as_ref()).unwrap(),
            ),
            (b"Xplain ", b" ", AuthMechanism::try_from("Xplain").unwrap()),
            (
                b"Xplain ",
                b" ",
                AuthMechanism::try_from(b"Xplain".as_ref()).unwrap(),
            ),
            (
                b"oauthbearer ".as_ref(),
                b" ".as_ref(),
                AuthMechanism::OAuthBearer,
            ),
            (b"oAuThbearEr ", b" ", AuthMechanism::OAuthBearer),
            (b"xoauth2 ".as_ref(), b" ".as_ref(), AuthMechanism::XOAuth2),
            (b"xOauTh2 ", b" ", AuthMechanism::XOAuth2),
        ];

        for test in tests {
            known_answer_test_parse(test, auth_type);
        }
    }

    #[test]
    fn test_authenticate_data() {
        let tests = [
            (b"*\r\n ".as_ref(), b" ".as_ref(), AuthenticateData::Cancel),
            (
                b"AA==\r\n ".as_ref(),
                b" ".as_ref(),
                AuthenticateData::r#continue(b"\x00".as_ref()),
            ),
            (
                b"aQ==\r\n ".as_ref(),
                b" ".as_ref(),
                AuthenticateData::r#continue(b"\x69".to_vec()),
            ),
        ];

        for test in tests {
            known_answer_test_parse(test, authenticate_data);
        }
    }
}
