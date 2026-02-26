use std::{borrow::Cow, num::NonZeroU32, str::from_utf8};

#[cfg(not(feature = "quirk_crlf_relaxed"))]
use abnf_core::streaming::crlf;
#[cfg(feature = "quirk_crlf_relaxed")]
use abnf_core::streaming::crlf_relaxed as crlf;
use abnf_core::{is_alpha, is_digit, streaming::dquote};
use base64::{Engine, engine::general_purpose::STANDARD as _base64};
use imap_types::{
    core::{
        AString, Atom, AtomExt, Charset, IString, Literal, LiteralMode, NString, Quoted,
        QuotedChar, Tag, Text,
    },
    utils::{
        indicators::{is_astring_char, is_atom_char, is_quoted_specials, is_text_char},
        unescape_quoted,
    },
};
#[cfg(feature = "fuzz")]
use nom::IResult;
use nom::{
    branch::alt,
    bytes::streaming::{escaped, tag, tag_no_case, take, take_while, take_while_m_n, take_while1},
    character::streaming::{char, digit1, one_of},
    combinator::{map, map_res, opt, recognize},
    sequence::{delimited, terminated, tuple},
};

use crate::decode::{IMAPErrorKind, IMAPParseError, IMAPResult};
#[cfg(feature = "ext_utf8")]
use crate::extensions::utf8::quoted_utf8;

// ----- number -----

/// `number = 1*DIGIT`
///
/// Unsigned 32-bit integer (0 <= n < 4,294,967,296)
pub(crate) fn number(input: &[u8]) -> IMAPResult<&[u8], u32> {
    map_res(
        // # Safety
        //
        // `unwrap` is safe because `1*DIGIT` contains ASCII-only characters.
        map(digit1, |val| from_utf8(val).unwrap()),
        str::parse::<u32>,
    )(input)
}

/// ```abnf
/// number64 = 1*DIGIT
/// ```
///
/// Unsigned 63-bit integer (0 <= n <= 9,223,372,036,854,775,807)
///
/// Defined in RFC 9051
pub(crate) fn number64(input: &[u8]) -> IMAPResult<&[u8], u64> {
    map_res(
        // # Safety
        //
        // `unwrap` is safe because `1*DIGIT` contains ASCII-only characters.
        map(digit1, |val| from_utf8(val).unwrap()),
        str::parse::<u64>,
    )(input)
}

/// `nz-number = digit-nz *DIGIT`
///
/// Non-zero unsigned 32-bit integer (0 < n < 4,294,967,296)
pub(crate) fn nz_number(input: &[u8]) -> IMAPResult<&[u8], NonZeroU32> {
    map_res(number, NonZeroU32::try_from)(input)
}

// ----- string -----

/// `string = quoted / literal`
pub(crate) fn string(input: &[u8]) -> IMAPResult<&[u8], IString> {
    alt((
        map(quoted, IString::Quoted),
        // quoted_utf8 must come after quoted but before literal.
        #[cfg(feature = "ext_utf8")]
        map(quoted_utf8, IString::QuotedUtf8),
        map(literal, IString::Literal),
    ))(input)
}

/// `quoted = DQUOTE *QUOTED-CHAR DQUOTE`
///
/// This function only allocates a new String, when needed, i.e. when
/// quoted chars need to be replaced.
pub(crate) fn quoted(input: &[u8]) -> IMAPResult<&[u8], Quoted> {
    let mut parser = tuple((
        dquote,
        map(
            escaped(
                take_while1(is_any_text_char_except_quoted_specials),
                '\\',
                one_of("\\\""),
            ),
            // # Safety
            //
            // `unwrap` is safe because val contains ASCII-only characters.
            |val| from_utf8(val).unwrap(),
        ),
        dquote,
    ));

    let (remaining, (_, quoted, _)) = parser(input)?;

    Ok((remaining, Quoted::unvalidated(unescape_quoted(quoted))))
}

/// `QUOTED-CHAR = <any TEXT-CHAR except quoted-specials> / "\" quoted-specials`
pub(crate) fn quoted_char(input: &[u8]) -> IMAPResult<&[u8], QuotedChar> {
    map(
        alt((
            map(
                take_while_m_n(1, 1, is_any_text_char_except_quoted_specials),
                |bytes: &[u8]| {
                    assert_eq!(bytes.len(), 1);
                    bytes[0] as char
                },
            ),
            map(
                tuple((tag("\\"), take_while_m_n(1, 1, is_quoted_specials))),
                |(_, bytes): (_, &[u8])| {
                    assert_eq!(bytes.len(), 1);
                    bytes[0] as char
                },
            ),
        )),
        QuotedChar::unvalidated,
    )(input)
}

pub(crate) fn is_any_text_char_except_quoted_specials(byte: u8) -> bool {
    is_text_char(byte) && !is_quoted_specials(byte)
}

/// `literal = "{" number "}" CRLF *CHAR8`
///
/// Number represents the number of CHAR8s
///
/// # IMAP4 Non-synchronizing Literals
///
/// ```abnf
/// literal  = "{" number ["+"] "}" CRLF *CHAR8
///              ; Number represents the number of CHAR8 octets
///
/// CHAR8    = <defined in RFC 3501>
///
/// literal8 = <defined in RFC 4466>
/// ```
/// -- <https://datatracker.ietf.org/doc/html/rfc7888#section-8>
pub(crate) fn literal(input: &[u8]) -> IMAPResult<&[u8], Literal> {
    let (remaining, (length, mode)) = terminated(
        delimited(
            tag(b"{"),
            tuple((
                number,
                map(opt(char('+')), |i| {
                    i.map(|_| LiteralMode::NonSync).unwrap_or(LiteralMode::Sync)
                }),
            )),
            tag(b"}"),
        ),
        crlf,
    )(input)?;

    // Signal that an continuation request could be required.
    // Note: This doesn't trigger when there is data following the literal prefix.
    if remaining.is_empty() {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::Literal {
                // We don't know the tag here and rely on an upper parser, e.g., `command` to fill this in.
                tag: None,
                length,
                mode,
            },
        }));
    }

    let (remaining, data) = take(length)(remaining)?;

    match Literal::try_from(data) {
        Ok(mut literal) => {
            literal.set_mode(mode);

            Ok((remaining, literal))
        }
        Err(_) => Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::LiteralContainsNull,
        })),
    }
}

// ----- astring ----- atom (roughly) or string

/// `astring = 1*ASTRING-CHAR / string`
pub(crate) fn astring(input: &[u8]) -> IMAPResult<&[u8], AString> {
    alt((
        map(take_while1(is_astring_char), |bytes: &[u8]| {
            // # Safety
            //
            // `unwrap` is safe, because `is_astring_char` enforces that the bytes ...
            //   * contain ASCII-only characters, i.e., `from_utf8` will return `Ok`.
            //   * are valid according to `AtomExt::verify(), i.e., `unvalidated` is safe.
            AString::Atom(AtomExt::unvalidated(Cow::Borrowed(
                std::str::from_utf8(bytes).unwrap(),
            )))
        }),
        map(string, AString::String),
    ))(input)
}

/// `atom = 1*ATOM-CHAR`
pub(crate) fn atom(input: &[u8]) -> IMAPResult<&[u8], Atom> {
    let parser = take_while1(is_atom_char);

    let (remaining, parsed_atom) = parser(input)?;

    // # Safety
    //
    // `unwrap` is safe, because `is_atom_char` enforces ...
    // * that the string is always UTF8, and ...
    // * contains only the allowed characters.
    Ok((
        remaining,
        Atom::unvalidated(from_utf8(parsed_atom).unwrap()),
    ))
}

// ----- nstring ----- nil or string

/// `nstring = string / nil`
pub(crate) fn nstring(input: &[u8]) -> IMAPResult<&[u8], NString> {
    alt((
        map(string, |item| NString(Some(item))),
        map(nil, |_| NString(None)),
    ))(input)
}

#[inline]
/// `nil = "NIL"`
pub(crate) fn nil(input: &[u8]) -> IMAPResult<&[u8], &[u8]> {
    tag_no_case(b"NIL")(input)
}

// ----- text -----

/// `text = 1*TEXT-CHAR`
pub(crate) fn text(input: &[u8]) -> IMAPResult<&[u8], Text> {
    map(take_while1(is_text_char), |bytes|
        // # Safety
        // 
        // `is_text_char` makes sure that the sequence of bytes
        // is always valid ASCII. Thus, it is also valid UTF-8.
        Text::unvalidated(from_utf8(bytes).unwrap()))(input)
}

// ----- base64 -----

/// `base64 = *(4base64-char) [base64-terminal]`
pub(crate) fn base64(input: &[u8]) -> IMAPResult<&[u8], Vec<u8>> {
    map_res(
        recognize(tuple((
            take_while(is_base64_char),
            opt(alt((tag("=="), tag("=")))),
        ))),
        |input| _base64.decode(input),
    )(input)
}

/// `base64-char = ALPHA / DIGIT / "+" / "/" ; Case-sensitive`
pub(crate) fn is_base64_char(i: u8) -> bool {
    is_alpha(i) || is_digit(i) || i == b'+' || i == b'/'
}

// base64-terminal = (2base64-char "==") / (3base64-char "=")

// ----- charset -----

/// `charset = atom / quoted`
///
/// Note: see errata id: 261
pub(crate) fn charset(input: &[u8]) -> IMAPResult<&[u8], Charset> {
    alt((map(atom, Charset::Atom), map(quoted, Charset::Quoted)))(input)
}

// ----- tag -----

/// `tag = 1*<any ASTRING-CHAR except "+">`
pub(crate) fn tag_imap(input: &[u8]) -> IMAPResult<&[u8], Tag> {
    map(take_while1(|b| is_astring_char(b) && b != b'+'), |val| {
        // # Safety
        //
        // `is_astring_char` ensures that `val` is UTF-8.
        Tag::unvalidated(from_utf8(val).unwrap())
    })(input)
}

// TODO: This could be exposed in a more elegant way...
#[cfg(feature = "fuzz")]
/// `tag = 1*<any ASTRING-CHAR except "+">`
pub fn fuzz_tag_imap(input: &[u8]) -> IResult<&[u8], Tag> {
    match tag_imap(input) {
        Ok((rem, out)) => Ok((rem, out)),
        Err(e) => match e {
            nom::Err::Incomplete(needed) => Err(nom::Err::Incomplete(needed)),
            nom::Err::Error(e) => Err(nom::Err::Error(nom::error::Error::new(
                e.input,
                nom::error::ErrorKind::Verify,
            ))),
            nom::Err::Failure(e) => Err(nom::Err::Failure(nom::error::Error::new(
                e.input,
                nom::error::ErrorKind::Verify,
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{EncodeContext, EncodeIntoContext};

    #[test]
    fn test_atom() {
        assert!(atom(b" ").is_err());
        assert!(atom(b"").is_err());

        let (rem, val) = atom(b"a(").unwrap();
        assert_eq!(val, "a".try_into().unwrap());
        assert_eq!(rem, b"(");

        let (rem, val) = atom(b"xxx yyy").unwrap();
        assert_eq!(val, "xxx".try_into().unwrap());
        assert_eq!(rem, b" yyy");
    }

    #[test]
    fn test_quoted() {
        let (rem, val) = quoted(br#""Hello"???"#).unwrap();
        assert_eq!(rem, b"???");
        assert_eq!(val, Quoted::try_from("Hello").unwrap());

        // Allowed escapes...
        assert!(quoted(br#""Hello \" "???"#).is_ok());
        assert!(quoted(br#""Hello \\ "???"#).is_ok());

        // Not allowed escapes...
        assert!(quoted(br#""Hello \a "???"#).is_err());
        assert!(quoted(br#""Hello \z "???"#).is_err());
        assert!(quoted(br#""Hello \? "???"#).is_err());

        let (rem, val) = quoted(br#""Hello \"World\""???"#).unwrap();
        assert_eq!(rem, br#"???"#);
        // Should it be this (Hello \"World\") ...
        //assert_eq!(val, r#"Hello \"World\""#);
        // ... or this (Hello "World")?
        assert_eq!(val, Quoted::try_from("Hello \"World\"").unwrap());

        // Test Incomplete
        assert!(matches!(quoted(br#""#), Err(nom::Err::Incomplete(_))));
        assert!(matches!(quoted(br#""\"#), Err(nom::Err::Incomplete(_))));
        assert!(matches!(
            quoted(br#""Hello "#),
            Err(nom::Err::Incomplete(_))
        ));

        // Test Error
        assert!(matches!(quoted(br#"\"#), Err(nom::Err::Error(_))));
    }

    #[test]
    fn test_quoted_char() {
        let (rem, val) = quoted_char(b"\\\"xxx").unwrap();
        assert_eq!(rem, b"xxx");
        assert_eq!(val, QuotedChar::try_from('"').unwrap());
    }

    #[test]
    fn test_number() {
        assert!(number(b"").is_err());
        assert!(number(b"?").is_err());

        assert!(number(b"0?").is_ok());
        assert!(number(b"55?").is_ok());
        assert!(number(b"999?").is_ok());
    }

    #[test]
    fn test_nz_number() {
        assert!(number(b"").is_err());
        assert!(number(b"?").is_err());

        assert!(nz_number(b"0?").is_err());
        assert!(nz_number(b"55?").is_ok());
        assert!(nz_number(b"999?").is_ok());
    }

    #[test]
    fn test_literal() {
        assert!(literal(b"{3}\r\n123").is_ok());
        assert!(literal(b"{3}\r\n1\x003").is_err());

        let (rem, val) = literal(b"{3}\r\n123xxx").unwrap();
        assert_eq!(rem, b"xxx");
        assert_eq!(val, Literal::try_from(b"123".as_slice()).unwrap());
    }

    #[test]
    fn test_nil() {
        assert!(nil(b"nil").is_ok());
        assert!(nil(b"nil ").is_ok());
        assert!(nil(b" nil").is_err());
        assert!(nil(b"null").is_err());

        let (rem, _) = nil(b"nilxxx").unwrap();
        assert_eq!(rem, b"xxx");
    }

    #[test]
    fn test_encode_charset() {
        let tests = [
            ("bengali", "bengali"),
            ("\"simple\" english", r#""\"simple\" english""#),
            ("", "\"\""),
            ("\"", "\"\\\"\""),
            ("\\", "\"\\\\\""),
        ];

        for (from, expected) in tests.iter() {
            let cs = Charset::try_from(*from).unwrap();
            println!("{cs:?}");

            let mut ctx = EncodeContext::new();
            cs.encode_ctx(&mut ctx).unwrap();

            let out = ctx.dump();
            assert_eq!(from_utf8(&out).unwrap(), *expected);
        }

        assert!(Charset::try_from("\r").is_err());
        assert!(Charset::try_from("\n").is_err());
        assert!(Charset::try_from("¹").is_err());
        assert!(Charset::try_from("²").is_err());
        assert!(Charset::try_from("\x00").is_err());
    }

    #[test]
    fn test_is_base64_char() {
        assert!(is_base64_char(b'a'));
        assert!(is_base64_char(b'z'));
        assert!(is_base64_char(b'A'));
        assert!(is_base64_char(b'Z'));
    }

    #[test]
    fn test_base64() {
        _base64.decode(b"AA==").unwrap();
        // Note: "pad bits MUST be set to zero by conforming encoders" [RFC 4648, sec. 3.5].
        //_base64.decode(b"aa==").unwrap();
        _base64.decode(b"aQ==").unwrap();
    }
}
