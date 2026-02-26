use abnf_core::streaming::sp;
use imap_types::{
    body::{
        BasicFields, Body, BodyExtension, BodyStructure, Disposition, Language, Location,
        MultiPartExtensionData, SinglePartExtensionData, SpecificFields,
    },
    core::{IString, NString, Vec1},
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, opt},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, preceded, tuple},
};

use crate::{
    core::{nil, nstring, number, string},
    decode::{IMAPErrorKind, IMAPParseError, IMAPResult},
    envelope::envelope,
};

/// `body = "(" (body-type-1part / body-type-mpart) ")"`
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed. (8 should suffice).
pub(crate) fn body(
    remaining_recursions: usize,
) -> impl Fn(&[u8]) -> IMAPResult<&[u8], BodyStructure> {
    move |input: &[u8]| body_limited(input, remaining_recursions)
}

fn body_limited(input: &[u8], remaining_recursions: usize) -> IMAPResult<&[u8], BodyStructure> {
    if remaining_recursions == 0 {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::RecursionLimitExceeded,
        }));
    }

    let body_type_1part =
        |input| body_type_1part_limited(input, remaining_recursions.saturating_sub(1));
    let body_type_mpart =
        |input| body_type_mpart_limited(input, remaining_recursions.saturating_sub(1));

    delimited(
        tag(b"("),
        alt((body_type_1part, body_type_mpart)),
        tag(b")"),
    )(input)
}

/// `body-type-1part = (
///                     body-type-basic /
///                     body-type-msg /
///                     body-type-text
///                    )
///                    [SP body-ext-1part]`
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed.
fn body_type_1part_limited(
    input: &[u8],
    remaining_recursions: usize,
) -> IMAPResult<&[u8], BodyStructure> {
    if remaining_recursions == 0 {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::RecursionLimitExceeded,
        }));
    }

    let body_type_msg = |input| body_type_msg_limited(input, 8);

    let mut parser = tuple((
        alt((body_type_msg, body_type_text, body_type_basic)),
        opt(preceded(sp, body_ext_1part)),
    ));

    let (remaining, ((basic, specific), extension_data)) = parser(input)?;

    Ok((
        remaining,
        BodyStructure::Single {
            body: Body { basic, specific },
            extension_data,
        },
    ))
}

/// `body-type-basic = media-basic SP body-fields`
///
/// MESSAGE subtype MUST NOT be "RFC822"
pub(crate) fn body_type_basic(input: &[u8]) -> IMAPResult<&[u8], (BasicFields, SpecificFields)> {
    let mut parser = tuple((media_basic, sp, body_fields));

    let (remaining, ((type_, subtype), _, basic)) = parser(input)?;

    Ok((
        remaining,
        (
            basic,
            SpecificFields::Basic {
                r#type: type_,
                subtype,
            },
        ),
    ))
}

/// `body-type-msg = media-message SP
///                 body-fields SP
///                 envelope SP
///                 body SP
///                 body-fld-lines`
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed. (8 should suffice).
fn body_type_msg_limited(
    input: &[u8],
    remaining_recursions: usize,
) -> IMAPResult<&[u8], (BasicFields, SpecificFields)> {
    if remaining_recursions == 0 {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::RecursionLimitExceeded,
        }));
    }

    let body = |input| body_limited(input, remaining_recursions.saturating_sub(1));

    let mut parser = tuple((
        media_message,
        sp,
        body_fields,
        sp,
        envelope,
        sp,
        body,
        sp,
        body_fld_lines,
    ));

    let (remaining, (_, _, basic, _, envelope, _, body_structure, _, number_of_lines)) =
        parser(input)?;

    Ok((
        remaining,
        (
            basic,
            SpecificFields::Message {
                envelope: Box::new(envelope),
                body_structure: Box::new(body_structure),
                number_of_lines,
            },
        ),
    ))
}

/// `body-type-text = media-text SP
///                   body-fields SP
///                   body-fld-lines`
pub(crate) fn body_type_text(input: &[u8]) -> IMAPResult<&[u8], (BasicFields, SpecificFields)> {
    let mut parser = tuple((media_text, sp, body_fields, sp, body_fld_lines));

    let (remaining, (subtype, _, basic, _, number_of_lines)) = parser(input)?;

    Ok((
        remaining,
        (
            basic,
            SpecificFields::Text {
                subtype,
                number_of_lines,
            },
        ),
    ))
}

/// `body-fields = body-fld-param SP
///                body-fld-id SP
///                body-fld-desc SP
///                body-fld-enc SP
///                body-fld-octets`
pub(crate) fn body_fields(input: &[u8]) -> IMAPResult<&[u8], BasicFields> {
    let mut parser = tuple((
        body_fld_param,
        sp,
        body_fld_id,
        sp,
        body_fld_desc,
        sp,
        body_fld_enc,
        sp,
        body_fld_octets,
    ));

    let (remaining, (parameter_list, _, id, _, description, _, content_transfer_encoding, _, size)) =
        parser(input)?;

    Ok((
        remaining,
        BasicFields {
            parameter_list,
            id,
            description,
            content_transfer_encoding,
            size,
        },
    ))
}

/// ```abnf
/// body-fld-param = "("
///                    string SP string
///                    *(SP string SP string)
///                  ")" / nil
/// ```
pub(crate) fn body_fld_param(input: &[u8]) -> IMAPResult<&[u8], Vec<(IString, IString)>> {
    let mut parser = alt((
        delimited(
            tag(b"("),
            // Quirk: See https://github.com/emersion/go-imap/issues/557
            separated_list0(
                sp,
                map(tuple((string, sp, string)), |(key, _, value)| (key, value)),
            ),
            tag(b")"),
        ),
        map(nil, |_| vec![]),
    ));

    let (remaining, parsed_body_fld_param) = parser(input)?;

    Ok((remaining, parsed_body_fld_param))
}

#[inline]
/// `body-fld-id = nstring`
pub(crate) fn body_fld_id(input: &[u8]) -> IMAPResult<&[u8], NString> {
    nstring(input)
}

#[inline]
/// `body-fld-desc = nstring`
pub(crate) fn body_fld_desc(input: &[u8]) -> IMAPResult<&[u8], NString> {
    nstring(input)
}

#[inline]
/// `body-fld-enc = (
///                   DQUOTE (
///                     "7BIT" /
///                     "8BIT" /
///                     "BINARY" /
///                     "BASE64"/
///                     "QUOTED-PRINTABLE"
///                   ) DQUOTE
///                 ) / string`
///
/// Simplified...
///
/// `body-fld-enc = string`
///
/// TODO: why the special case?
///
/// # Quirks
///
/// The following erroneous content observed:
///
/// * A NIL value in maddy.
pub(crate) fn body_fld_enc(input: &[u8]) -> IMAPResult<&[u8], IString> {
    #[cfg(not(feature = "quirk_body_fld_enc_nil_to_empty"))]
    return string(input);

    #[cfg(feature = "quirk_body_fld_enc_nil_to_empty")]
    map(nstring, |enc| match enc.0 {
        Some(enc) => enc,
        None => IString::try_from("").unwrap(),
    })(input)
}

#[inline]
/// `body-fld-octets = number`
///
/// # Quirks
///
/// The following erroneous messages were observed:
///
/// * A negative number, specifically `-1`, in Dovecot.
#[allow(clippy::needless_return)]
pub(crate) fn body_fld_octets(input: &[u8]) -> IMAPResult<&[u8], u32> {
    #[cfg(not(feature = "quirk_rectify_numbers"))]
    return number(input);

    #[cfg(feature = "quirk_rectify_numbers")]
    {
        return alt((
            number,
            map(tuple((tag("-"), number)), |(_, _)| {
                log::warn!("Rectified negative number to 0");
                0
            }),
        ))(input);
    }
}

#[inline]
/// `body-fld-lines = number`
pub(crate) fn body_fld_lines(input: &[u8]) -> IMAPResult<&[u8], u32> {
    number(input)
}

/// ```abnf
/// body-ext-1part = body-fld-md5
///                   [SP body-fld-dsp
///                     [SP body-fld-lang
///                       [SP body-fld-loc *(SP body-extension)]
///                     ]
///                   ]
/// ```
///
/// Note: MUST NOT be returned on non-extensible "BODY" fetch.
pub(crate) fn body_ext_1part(input: &[u8]) -> IMAPResult<&[u8], SinglePartExtensionData> {
    map(
        tuple((
            body_fld_md5,
            opt(map(
                tuple((
                    preceded(sp, body_fld_dsp),
                    opt(map(
                        tuple((
                            preceded(sp, body_fld_lang),
                            opt(map(
                                tuple((
                                    preceded(sp, body_fld_loc),
                                    many0(preceded(sp, body_extension(8))),
                                )),
                                |(location, extensions)| Location {
                                    location,
                                    extensions,
                                },
                            )),
                        )),
                        |(language, tail)| Language { language, tail },
                    )),
                )),
                |(disposition, tail)| Disposition { disposition, tail },
            )),
        )),
        |(md5, tail)| SinglePartExtensionData { md5, tail },
    )(input)
}

#[inline]
/// `body-fld-md5 = nstring`
pub(crate) fn body_fld_md5(input: &[u8]) -> IMAPResult<&[u8], NString> {
    nstring(input)
}

/// `body-fld-dsp = "(" string SP body-fld-param ")" / nil`
#[allow(clippy::type_complexity)]
pub(crate) fn body_fld_dsp(
    input: &[u8],
) -> IMAPResult<&[u8], Option<(IString, Vec<(IString, IString)>)>> {
    alt((
        delimited(
            tag(b"("),
            map(
                tuple((string, sp, body_fld_param)),
                |(string, _, body_fld_param)| Some((string, body_fld_param)),
            ),
            tag(b")"),
        ),
        map(nil, |_| None),
    ))(input)
}

/// `body-fld-lang = nstring / "(" string *(SP string) ")"`
pub(crate) fn body_fld_lang(input: &[u8]) -> IMAPResult<&[u8], Vec<IString>> {
    alt((
        map(nstring, |nstring| match nstring.0 {
            Some(item) => vec![item],
            None => vec![],
        }),
        delimited(tag(b"("), separated_list1(sp, string), tag(b")")),
    ))(input)
}

#[inline]
/// `body-fld-loc = nstring`
pub(crate) fn body_fld_loc(input: &[u8]) -> IMAPResult<&[u8], NString> {
    nstring(input)
}

/// Future expansion.
///
/// Client implementations MUST accept body-extension fields.
/// Server implementations MUST NOT generate body-extension fields except as defined by
/// future standard or standards-track revisions of this specification.
///
/// ```abnf
/// body-extension = nstring /
///                  number /
///                  "(" body-extension *(SP body-extension) ")"
/// ```
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed. (8 should suffice).
pub(crate) fn body_extension(
    remaining_recursions: usize,
) -> impl Fn(&[u8]) -> IMAPResult<&[u8], BodyExtension> {
    move |input: &[u8]| body_extension_limited(input, remaining_recursions)
}

fn body_extension_limited(
    input: &[u8],
    remaining_recursion: usize,
) -> IMAPResult<&[u8], BodyExtension> {
    if remaining_recursion == 0 {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::RecursionLimitExceeded,
        }));
    }

    let body_extension =
        |input| body_extension_limited(input, remaining_recursion.saturating_sub(1));

    alt((
        map(nstring, BodyExtension::NString),
        map(number, BodyExtension::Number),
        map(
            delimited(tag(b"("), separated_list1(sp, body_extension), tag(b")")),
            |body_extensions| BodyExtension::List(Vec1::unvalidated(body_extensions)),
        ),
    ))(input)
}

// ---

/// `body-type-mpart = 1*body SP media-subtype [SP body-ext-mpart]`
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed.
///
/// # Quirk
///
/// Many IMAP servers send spaces between body parts in multipart structures
/// (e.g., `(body1) (body2) "alternative"` instead of `(body1)(body2) "alternative"`).
/// While RFC 3501's ABNF `1*body` implies no separator, we accept optional spaces
/// between body parts to handle real-world server behavior.
fn body_type_mpart_limited(
    input: &[u8],
    remaining_recursion: usize,
) -> IMAPResult<&[u8], BodyStructure> {
    if remaining_recursion == 0 {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::RecursionLimitExceeded,
        }));
    }

    // Parse the first body (required).
    let (mut remaining, first_body) = body(remaining_recursion)(input)?;
    let mut bodies = vec![first_body];

    // Parse additional bodies, allowing optional spaces between them.
    // We must not consume the SP before media-subtype, so we save/restore
    // on failure.
    loop {
        let saved = remaining;

        // Skip optional space before next body.
        let after_sp = match opt::<_, _, IMAPParseError<&[u8]>, _>(sp)(remaining) {
            Ok((r, _)) => r,
            Err(_) => remaining,
        };

        // Try to parse another body. Body always starts with '(' so this
        // won't accidentally consume the media-subtype string.
        match body(remaining_recursion)(after_sp) {
            Ok((r, b)) => {
                remaining = r;
                bodies.push(b);
            }
            Err(nom::Err::Error(_)) => {
                // No more bodies; restore position before optional space.
                remaining = saved;
                break;
            }
            Err(e) => return Err(e),
        }
    }

    let (remaining, (_, subtype, extension_data)) = tuple((
        sp,
        media_subtype,
        opt(preceded(sp, body_ext_mpart)),
    ))(remaining)?;

    Ok((
        remaining,
        BodyStructure::Multi {
            // Safety: `unwrap` can't panic because we always have at least `first_body`.
            bodies: Vec1::try_from(bodies).unwrap(),
            subtype,
            extension_data,
        },
    ))
}

/// ```abnf
/// body-ext-mpart = body-fld-param
///                   [SP body-fld-dsp
///                     [SP body-fld-lang
///                       [SP body-fld-loc *(SP body-extension)]
///                     ]
///                   ]
/// ```
///
/// Note: MUST NOT be returned on non-extensible "BODY" fetch.
pub(crate) fn body_ext_mpart(input: &[u8]) -> IMAPResult<&[u8], MultiPartExtensionData> {
    map(
        tuple((
            body_fld_param,
            opt(map(
                tuple((
                    preceded(sp, body_fld_dsp),
                    opt(map(
                        tuple((
                            preceded(sp, body_fld_lang),
                            opt(map(
                                tuple((
                                    preceded(sp, body_fld_loc),
                                    many0(preceded(sp, body_extension(8))),
                                )),
                                |(location, extensions)| Location {
                                    location,
                                    extensions,
                                },
                            )),
                        )),
                        |(language, tail)| Language { language, tail },
                    )),
                )),
                |(disposition, tail)| Disposition { disposition, tail },
            )),
        )),
        |(parameter_list, tail)| MultiPartExtensionData {
            parameter_list,
            tail,
        },
    )(input)
}

// ---

/// `media-basic = (
///                  ( DQUOTE
///                    (
///                      "APPLICATION" /
///                      "AUDIO" /
///                      "IMAGE" /
///                      "MESSAGE" /
///                      "VIDEO"
///                    ) DQUOTE
///                  ) / string
///                ) SP media-subtype`
///
/// Simplified...
///
/// `media-basic = string SP media-subtype`
///
/// TODO: Why the special case?
///
/// Defined in [MIME-IMT]
pub(crate) fn media_basic(input: &[u8]) -> IMAPResult<&[u8], (IString, IString)> {
    let mut parser = tuple((string, sp, media_subtype));

    let (remaining, (type_, _, subtype)) = parser(input)?;

    Ok((remaining, (type_, subtype)))
}

#[inline]
/// `media-subtype = string`
///
/// Defined in [MIME-IMT]
pub(crate) fn media_subtype(input: &[u8]) -> IMAPResult<&[u8], IString> {
    string(input)
}

#[inline]
/// `media-message = DQUOTE "MESSAGE" DQUOTE SP
///                  DQUOTE "RFC822" DQUOTE`
///
/// Simplified:
///
/// `media-message = "\"MESSAGE\" \"RFC822\""`
///
/// Defined in [MIME-IMT]
///
/// "message" "rfc822" basic specific-for-message-rfc822 extension
pub(crate) fn media_message(input: &[u8]) -> IMAPResult<&[u8], &[u8]> {
    tag_no_case(b"\"MESSAGE\" \"RFC822\"")(input)
}

/// `media-text = DQUOTE "TEXT" DQUOTE SP media-subtype`
///
/// Defined in [MIME-IMT]
///
/// "text" "?????" basic specific-for-text extension
pub(crate) fn media_text(input: &[u8]) -> IMAPResult<&[u8], IString> {
    let mut parser = preceded(tag_no_case(b"\"TEXT\" "), media_subtype);

    let (remaining, media_subtype) = parser(input)?;

    Ok((remaining, media_subtype))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use imap_types::{
        core::{Literal, Quoted},
        fetch::MessageDataItem,
        response::{Data, Response},
    };

    use super::*;
    use crate::testing::{kat_inverse_response, known_answer_test_encode};

    #[test]
    fn test_parse_media_basic() {
        media_basic(b"\"application\" \"xxx\"").unwrap();
        media_basic(b"\"unknown\" \"test\"").unwrap();
        media_basic(b"\"x\" \"xxx\"").unwrap();
    }

    #[test]
    fn test_parse_media_message() {
        media_message(b"\"message\" \"rfc822\"").unwrap();
    }

    #[test]
    fn test_parse_media_text() {
        media_text(b"\"text\" \"html\"").unwrap();
    }

    #[test]
    fn test_parse_body_ext_1part() {
        for test in [
            b"nil|xxx".as_ref(),
            b"\"md5\"|xxx".as_ref(),
            b"\"md5\" nil|xxx".as_ref(),
            b"\"md5\" (\"dsp\" nil)|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) nil|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) \"swedish\"|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\")|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") nil|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\"|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\" (1 \"2\" (nil 4))|xxx".as_ref(),
            b"\"AABB\" NIL NIL NIL 1337|xxx",
            b"\"AABB\" NIL NIL NIL (1337)|xxx",
            b"\"AABB\" NIL NIL NIL (1337 1337)|xxx",
            b"\"AABB\" NIL NIL NIL (1337 (1337 (1337 \"FOO\" {0}\r\n)))|xxx",
        ]
        .iter()
        {
            let (rem, out) = body_ext_1part(test).unwrap();
            println!("{out:?}");
            assert_eq!(rem, b"|xxx");
        }
    }

    #[test]
    fn test_body_rec() {
        let _ = body(8)(str::repeat("(", 1_000_000).as_bytes());
    }

    #[test]
    fn test_parse_body_ext_mpart() {
        for test in [
            b"nil|xxx".as_ref(),
            b"(\"key\" \"value\")|xxx".as_ref(),
            b"(\"key\" \"value\") nil|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" nil)|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) nil|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) \"swedish\"|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\")|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") nil|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\"|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\" (1 \"2\" (nil 4))|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\" (1 \"2\" (nil 4) {0}\r\n)|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\" {0}\r\n {0}\r\n|xxx".as_ref(),
        ]
            .iter()
        {
            let (rem, out) = body_ext_mpart(test).unwrap();
            println!("{out:?}");
            assert_eq!(rem, b"|xxx");
        }
    }

    #[test]
    fn test_parse_body() {
        dbg!(body(9)(b"((((((({0}\r\n {0}\r\n NIL NIL NIL {0}\r\n 0 \"FOO\" NIL NIL \"LOCATION\" 1337) \"mixed\") \"mixed\") \"mixed\") \"mixed\") \"mixed\") \"mixed\")|xxx").unwrap());
    }

    #[test]
    fn test_parse_body_mpart_with_spaces_between_parts() {
        // Many IMAP servers (e.g. Stalwart, Exchange) send spaces between body
        // parts in multipart BODYSTRUCTURE responses.

        // Without spaces (strict RFC interpretation): bodies directly adjacent
        let (rem, _) = body(8)(
            b"((\"text\" \"plain\" NIL NIL NIL \"7bit\" 100 5)(\"text\" \"html\" NIL NIL NIL \"7bit\" 200 10) \"alternative\")|end"
        ).unwrap();
        assert_eq!(rem, b"|end");

        // With spaces between body parts (common server behavior)
        let (rem, _) = body(8)(
            b"((\"text\" \"plain\" NIL NIL NIL \"7bit\" 100 5) (\"text\" \"html\" NIL NIL NIL \"7bit\" 200 10) \"alternative\")|end"
        ).unwrap();
        assert_eq!(rem, b"|end");

        // With spaces and multipart extension data (NIL NIL NIL)
        let (rem, _) = body(8)(
            b"((\"text\" \"plain\" (\"charset\" \"utf-8\") NIL NIL \"base64\" 4620 60 NIL NIL NIL NIL) (\"text\" \"html\" (\"charset\" \"utf-8\") NIL NIL \"base64\" 21304 274 NIL NIL NIL NIL) \"alternative\" (\"boundary\" \"000000000000abcdef\") NIL NIL NIL)|end"
        ).unwrap();
        assert_eq!(rem, b"|end");

        // Three body parts with spaces
        let (rem, _) = body(8)(
            b"((\"text\" \"plain\" NIL NIL NIL \"7bit\" 100 5) (\"text\" \"html\" NIL NIL NIL \"7bit\" 200 10) (\"application\" \"pdf\" NIL NIL NIL \"base64\" 5000) \"mixed\")|end"
        ).unwrap();
        assert_eq!(rem, b"|end");
    }

    #[test]
    fn test_kat_inverse_response_data() {
        kat_inverse_response(&[(
            b"* 3372220415 FETCH (BODYSTRUCTURE ((((((({0}\r\n {0}\r\n NIL NIL NIL {0}\r\n 0 \"FOO\" NIL NIL \"LOCATION\" 1337) \"mixed\") \"mixed\") \"mixed\") \"mixed\") \"mixed\") \"mixed\"))\r\n".as_ref(),
            b"".as_ref(),
            Response::Data(Data::Fetch {
                seq: NonZeroU32::try_from(3372220415).unwrap(),
                items: Vec1::from(MessageDataItem::BodyStructure(
                    BodyStructure::Multi {
                        bodies: Vec1::from(BodyStructure::Multi {
                            bodies: Vec1::from(BodyStructure::Multi {
                                bodies: Vec1::from(BodyStructure::Multi {
                                    bodies: Vec1::from(BodyStructure::Multi {
                                        bodies: Vec1::from(BodyStructure::Multi {
                                            bodies: Vec1::from(BodyStructure::Single {
                                                body: Body {
                                                    basic: BasicFields {
                                                        parameter_list: vec![],
                                                        id: NString(None),
                                                        description: NString(None),
                                                        content_transfer_encoding: IString::from(
                                                            Literal::try_from(b"".as_ref())
                                                                .unwrap(),
                                                        ),
                                                        size: 0,
                                                    },
                                                    specific: SpecificFields::Basic {
                                                        r#type: IString::from(
                                                            Literal::try_from(b"".as_ref())
                                                                .unwrap(),
                                                        ),
                                                        subtype: IString::from(
                                                            Literal::try_from(b"".as_ref())
                                                                .unwrap(),
                                                        ),
                                                    },
                                                },
                                                extension_data: Some(SinglePartExtensionData {
                                                    md5: NString::try_from("FOO").unwrap(),
                                                    tail: Some(Disposition{
                                                        disposition: None,
                                                        tail: Some(Language {
                                                            language: vec![],
                                                            tail: Some(Location{
                                                                location: NString::try_from("LOCATION").unwrap(),
                                                                extensions: vec![BodyExtension::Number(1337)],
                                                            })
                                                        })
                                                    })
                                                }),
                                            }),
                                            subtype: IString::try_from("mixed").unwrap(),
                                            extension_data: None,
                                        }),
                                        subtype: IString::try_from("mixed").unwrap(),
                                        extension_data: None,
                                    }),
                                    subtype: IString::try_from("mixed").unwrap(),
                                    extension_data: None,
                                }),
                                subtype: IString::try_from("mixed").unwrap(),
                                extension_data: None,
                            }),
                            subtype: IString::try_from("mixed").unwrap(),
                            extension_data: None,
                        }),
                        subtype: IString::try_from("mixed").unwrap(),
                        extension_data: None,
                    },
                )),
            }),
        )]);
    }

    #[test]
    fn test_encode_single_part_extension_data() {
        let tests = [(
            SinglePartExtensionData {
                md5: NString(None),
                tail: Some(Disposition {
                    disposition: None,
                    tail: Some(Language {
                        language: vec![],
                        tail: Some(Location {
                            location: NString::from(Quoted::try_from("").unwrap()),
                            extensions: vec![],
                        }),
                    }),
                }),
            },
            b"NIL NIL NIL \"\"".as_ref(),
        )];

        for test in tests {
            known_answer_test_encode(test);
        }
    }

    #[test]
    fn test_number_quirk() {
        assert_eq!(body_fld_octets(b"0)").unwrap().1, 0);
        assert_eq!(body_fld_octets(b"1)").unwrap().1, 1);

        #[cfg(not(feature = "quirk_rectify_numbers"))]
        {
            assert!(dbg!(body_fld_octets(b"-0)")).is_err());
            assert!(body_fld_octets(b"-1)").is_err());
            assert!(body_fld_octets(b"-999999)").is_err());
        }

        #[cfg(feature = "quirk_rectify_numbers")]
        {
            assert_eq!(body_fld_octets(b"-0)").unwrap().1, 0);
            assert_eq!(body_fld_octets(b"-1)").unwrap().1, 0);
            assert_eq!(body_fld_octets(b"-999999)").unwrap().1, 0);
        }
    }
}
