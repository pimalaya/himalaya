#[cfg(not(feature = "quirk_crlf_relaxed"))]
use abnf_core::streaming::crlf;
#[cfg(feature = "quirk_crlf_relaxed")]
use abnf_core::streaming::crlf_relaxed as crlf;
use abnf_core::streaming::sp;
use base64::{Engine, engine::general_purpose::STANDARD as _base64};
#[cfg(feature = "ext_condstore_qresync")]
use imap_types::sequence::SequenceSet;
use imap_types::{
    core::{Text, Vec1},
    fetch::MessageDataItem,
    response::{
        Bye, Capability, Code, CodeOther, CommandContinuationRequest, Data, Greeting, GreetingKind,
        Response, Status, StatusBody, StatusKind, Tagged,
    },
};
#[cfg(feature = "quirk_missing_text")]
use nom::combinator::peek;
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case, take_until, take_while},
    combinator::{map, map_res, opt, value},
    multi::separated_list1,
    sequence::{delimited, preceded, terminated, tuple},
};

#[cfg(feature = "ext_id")]
use crate::extensions::id::id_response;
#[cfg(feature = "ext_metadata")]
use crate::extensions::metadata::metadata_code;
use crate::{
    core::{atom, charset, nz_number, tag_imap, text},
    decode::IMAPResult,
    extensions::{
        enable::enable_data,
        uidplus::{resp_code_apnd, resp_code_copy},
    },
    fetch::msg_att,
    flag::flag_perm,
    mailbox::mailbox_data,
};
#[cfg(feature = "ext_condstore_qresync")]
use crate::{extensions::condstore_qresync::mod_sequence_value, sequence::sequence_set};

// ----- greeting -----

/// `greeting = "*" SP (resp-cond-auth / resp-cond-bye) CRLF`
pub(crate) fn greeting(input: &[u8]) -> IMAPResult<&[u8], Greeting> {
    let mut parser = delimited(
        tag(b"* "),
        alt((
            resp_cond_auth,
            map(resp_cond_bye, |resp_text| (GreetingKind::Bye, resp_text)),
        )),
        crlf,
    );

    let (remaining, (kind, (code, text))) = parser(input)?;

    Ok((remaining, Greeting { kind, code, text }))
}

/// `resp-cond-auth = ("OK" / "PREAUTH") SP resp-text`
///
/// Authentication condition
#[allow(clippy::type_complexity)]
pub(crate) fn resp_cond_auth(
    input: &[u8],
) -> IMAPResult<&[u8], (GreetingKind, (Option<Code>, Text))> {
    let mut parser = tuple((
        alt((
            value(GreetingKind::Ok, tag_no_case(b"OK ")),
            value(GreetingKind::PreAuth, tag_no_case(b"PREAUTH ")),
        )),
        resp_text,
    ));

    let (remaining, (kind, resp_text)) = parser(input)?;

    Ok((remaining, (kind, resp_text)))
}

/// `resp-text = ["[" resp-text-code "]" SP] text`
pub(crate) fn resp_text(input: &[u8]) -> IMAPResult<&[u8], (Option<Code>, Text)> {
    // When the text starts with "[", we insist on parsing a code.
    // Otherwise, a broken code could be interpreted as text.
    let (_, start) = opt(tag(b"["))(input)?;

    if start.is_some() {
        tuple((
            preceded(
                tag(b"["),
                map(
                    alt((
                        terminated(resp_text_code, tag(b"]")),
                        map(
                            terminated(
                                take_while(|b: u8| b != b']' && b != b'\r' && b != b'\n'),
                                tag(b"]"),
                            ),
                            |bytes: &[u8]| Code::Other(CodeOther::unvalidated(bytes)),
                        ),
                    )),
                    Some,
                ),
            ),
            #[cfg(not(feature = "quirk_missing_text"))]
            preceded(sp, text),
            #[cfg(feature = "quirk_missing_text")]
            alt((
                preceded(sp, text),
                map(peek(crlf), |_| {
                    log::warn!("Rectified missing `text` to \"...\"");

                    Text::unvalidated("...")
                }),
            )),
        ))(input)
    } else {
        map(text, |text| (None, text))(input)
    }
}

/// ```abnf
/// resp-text-code = "ALERT" /
///                  "BADCHARSET" [SP "(" charset *(SP charset) ")" ] /
///                  capability-data /
///                  "PARSE" /
///                  "PERMANENTFLAGS" SP "(" [flag-perm *(SP flag-perm)] ")" /
///                  "READ-ONLY" /
///                  "READ-WRITE" /
///                  "TRYCREATE" /
///                  "UIDNEXT" SP nz-number /
///                  "UIDVALIDITY" SP nz-number /
///                  "UNSEEN" SP nz-number /
///                  "COMPRESSIONACTIVE" / ; RFC 4978
///                  "OVERQUOTA" /         ; RFC 9208
///                  "TOOBIG" /            ; RFC 4469
///                  "METADATA" SP (       ; RFC 5464
///                    "LONGENTRIES" SP number /
///                    "MAXSIZE" SP number /
///                    "TOOMANY" /
///                    "NOPRIVATE"
///                  ) /
///                  "UNKNOWN-CTE" /       ; RFC 3516
///                  "HIGHESTMODSEQ" SP mod-sequence-value / ; RFC7162
///                  "NOMODSEQ"                            / ; RFC7162
///                  "MODIFIED" SP sequence-set            / ; RFC7162
///                  "CLOSED"                              / ; RFC7162
///                  atom [SP 1*<any TEXT-CHAR except "]">]
/// ```
///
/// Note: See errata id: 261
pub(crate) fn resp_text_code(input: &[u8]) -> IMAPResult<&[u8], Code> {
    alt((
        value(Code::Alert, tag_no_case(b"ALERT")),
        map(
            preceded(
                tag_no_case(b"BADCHARSET"),
                opt(delimited(
                    tag(b" ("),
                    separated_list1(sp, charset),
                    tag(b")"),
                )),
            ),
            |maybe_charsets| Code::BadCharset {
                allowed: maybe_charsets.unwrap_or_default(),
            },
        ),
        map(capability_data, Code::Capability),
        value(Code::Parse, tag_no_case(b"PARSE")),
        map(
            preceded(
                tag_no_case(b"PERMANENTFLAGS "),
                delimited(
                    tag(b"("),
                    map(opt(separated_list1(sp, flag_perm)), |maybe_flags| {
                        maybe_flags.unwrap_or_default()
                    }),
                    tag(b")"),
                ),
            ),
            Code::PermanentFlags,
        ),
        value(Code::ReadOnly, tag_no_case(b"READ-ONLY")),
        value(Code::ReadWrite, tag_no_case(b"READ-WRITE")),
        value(Code::TryCreate, tag_no_case(b"TRYCREATE")),
        map(preceded(tag_no_case(b"UIDNEXT "), nz_number), Code::UidNext),
        map(
            preceded(tag_no_case(b"UIDVALIDITY "), nz_number),
            Code::UidValidity,
        ),
        map(preceded(tag_no_case(b"UNSEEN "), nz_number), Code::Unseen),
        value(Code::CompressionActive, tag_no_case(b"COMPRESSIONACTIVE")),
        value(Code::OverQuota, tag_no_case(b"OVERQUOTA")),
        value(Code::TooBig, tag_no_case(b"TOOBIG")),
        #[cfg(feature = "ext_metadata")]
        map(
            preceded(tag_no_case("METADATA "), metadata_code),
            Code::Metadata,
        ),
        value(Code::UnknownCte, tag_no_case(b"UNKNOWN-CTE")),
        resp_code_apnd,
        resp_code_copy,
        value(Code::UidNotSticky, tag_no_case(b"UIDNOTSTICKY")),
        #[cfg(feature = "ext_condstore_qresync")]
        alt((
            map(
                preceded(tag_no_case(b"HIGHESTMODSEQ "), mod_sequence_value),
                Code::HighestModSeq,
            ),
            value(Code::NoModSeq, tag_no_case(b"NOMODSEQ")),
            map(
                preceded(tag_no_case(b"MODIFIED "), sequence_set),
                Code::Modified,
            ),
            value(Code::Closed, tag_no_case(b"CLOSED")),
        )),
    ))(input)
}

/// `capability-data = "CAPABILITY" *(SP capability) SP "IMAP4rev1" *(SP capability)`
///
/// Servers MUST implement the STARTTLS, AUTH=PLAIN, and LOGINDISABLED capabilities
/// Servers which offer RFC 1730 compatibility MUST list "IMAP4" as the first capability.
pub(crate) fn capability_data(input: &[u8]) -> IMAPResult<&[u8], Vec1<Capability>> {
    map(
        #[cfg(not(feature = "quirk_trailing_space_capability"))]
        preceded(tag_no_case("CAPABILITY "), separated_list1(sp, capability)),
        #[cfg(feature = "quirk_trailing_space_capability")]
        terminated(
            preceded(tag_no_case("CAPABILITY "), separated_list1(sp, capability)),
            opt(sp),
        ),
        Vec1::unvalidated,
    )(input)
}

/// `capability = ("AUTH=" auth-type) /
///               "COMPRESS=" algorithm / ; RFC 4978
///               atom`
pub(crate) fn capability(input: &[u8]) -> IMAPResult<&[u8], Capability> {
    map(atom, Capability::from)(input)
}

/// `resp-cond-bye = "BYE" SP resp-text`
pub(crate) fn resp_cond_bye(input: &[u8]) -> IMAPResult<&[u8], (Option<Code>, Text)> {
    preceded(tag_no_case(b"BYE "), resp_text)(input)
}

// ----- response -----

/// `response = *(continue-req / response-data) response-done`
pub(crate) fn response(input: &[u8]) -> IMAPResult<&[u8], Response> {
    // Divert from standard here for better usability.
    // response_data already contains the bye response, thus
    // response_done could also be response_tagged.
    //
    // However, I will keep it as it is for now.
    alt((
        #[cfg(feature = "quirk_empty_continue_req")]
        map(empty_continue_req, Response::CommandContinuationRequest),
        map(continue_req, Response::CommandContinuationRequest),
        response_data,
        map(response_done, Response::Status),
    ))(input)
}

/// Parser that allows a spaceless, empty continuation request `+\r\n`.
#[cfg(feature = "quirk_empty_continue_req")]
pub(crate) fn empty_continue_req(input: &[u8]) -> IMAPResult<&[u8], CommandContinuationRequest> {
    let mut parser = tuple((tag(b"+"), crlf));
    let (remaining, _) = parser(input)?;
    log::warn!("Rectified faulty continuation request `+\r\n` to `+ ...\r\n`");
    let req = CommandContinuationRequest::basic(None, "...").unwrap();

    Ok((remaining, req))
}

/// `continue-req = "+" SP (resp-text / base64) CRLF`
pub(crate) fn continue_req(input: &[u8]) -> IMAPResult<&[u8], CommandContinuationRequest> {
    // We can't map the output of `resp_text` directly to `Continue::basic()` because we might end
    // up with a subset of `Text` that is valid base64 and will panic on `unwrap()`. Thus, we first
    // let the parsing finish and only later map to `Continue`.

    // A helper struct to postpone the unification to `Continue` in the `alt` combinator below.
    enum Either<A, B> {
        Base64(A),
        Basic(B),
    }

    let mut parser = tuple((
        tag(b"+ "),
        alt((
            #[cfg(not(feature = "quirk_crlf_relaxed"))]
            map(
                map_res(take_until("\r\n"), |input| _base64.decode(input)),
                Either::Base64,
            ),
            #[cfg(feature = "quirk_crlf_relaxed")]
            map(
                map_res(take_until("\n"), |input: &[u8]| {
                    if !input.is_empty() && input[input.len().saturating_sub(1)] == b'\r' {
                        _base64.decode(&input[..input.len().saturating_sub(1)])
                    } else {
                        _base64.decode(input)
                    }
                }),
                Either::Base64,
            ),
            map(resp_text, Either::Basic),
        )),
        crlf,
    ));

    let (remaining, (_, either, _)) = parser(input)?;

    let continue_request = match either {
        Either::Base64(data) => CommandContinuationRequest::base64(data),
        Either::Basic((code, text)) => CommandContinuationRequest::basic(code, text).unwrap(),
    };

    Ok((remaining, continue_request))
}

/// ```abnf
/// response-data = "*" SP (
///                    resp-cond-state /
///                    resp-cond-bye /
///                    mailbox-data /
///                    message-data /
///                    capability-data /
///                    id_response ; (See RFC 2971)
///                  ) CRLF
/// ```
pub(crate) fn response_data(input: &[u8]) -> IMAPResult<&[u8], Response> {
    delimited(
        tag(b"* "),
        alt((
            map(resp_cond_state, |(kind, code, text)| {
                Response::Status(Status::Untagged(StatusBody { kind, code, text }))
            }),
            map(resp_cond_bye, |(code, text)| {
                Response::Status(Status::Bye(Bye { code, text }))
            }),
            map(mailbox_data, Response::Data),
            map(message_data, Response::Data),
            map(capability_data, |caps| {
                Response::Data(Data::Capability(caps))
            }),
            map(enable_data, Response::Data),
            #[cfg(feature = "ext_id")]
            map(id_response, |parameters| {
                Response::Data(Data::Id { parameters })
            }),
        )),
        crlf,
    )(input)
}

/// `resp-cond-state = ("OK" / "NO" / "BAD") SP resp-text`
///
/// Status condition
pub(crate) fn resp_cond_state(input: &[u8]) -> IMAPResult<&[u8], (StatusKind, Option<Code>, Text)> {
    let mut parser = tuple((
        alt((
            value(StatusKind::Ok, tag_no_case("OK ")),
            value(StatusKind::No, tag_no_case("NO ")),
            value(StatusKind::Bad, tag_no_case("BAD ")),
        )),
        resp_text,
    ));

    let (remaining, (kind, (maybe_code, text))) = parser(input)?;

    Ok((remaining, (kind, maybe_code, text)))
}

/// `response-done = response-tagged / response-fatal`
pub(crate) fn response_done(input: &[u8]) -> IMAPResult<&[u8], Status> {
    alt((response_tagged, response_fatal))(input)
}

/// `response-tagged = tag SP resp-cond-state CRLF`
pub(crate) fn response_tagged(input: &[u8]) -> IMAPResult<&[u8], Status> {
    let mut parser = tuple((tag_imap, sp, resp_cond_state, crlf));

    let (remaining, (tag, _, (kind, code, text), _)) = parser(input)?;

    Ok((
        remaining,
        Status::Tagged(Tagged {
            tag,
            body: StatusBody { kind, code, text },
        }),
    ))
}

/// `response-fatal = "*" SP resp-cond-bye CRLF`
///
/// Server closes connection immediately
pub(crate) fn response_fatal(input: &[u8]) -> IMAPResult<&[u8], Status> {
    let mut parser = delimited(tag(b"* "), resp_cond_bye, crlf);

    let (remaining, (code, text)) = parser(input)?;

    Ok((remaining, Status::Bye(Bye { code, text })))
}

/// ```abnf
/// message-data = nz-number SP ("EXPUNGE" / ("FETCH" SP msg-att))
/// ```
///
/// From RFC 7162:
///
/// ```abnf
/// message-data =/ expunged-resp
///
/// expunged-resp = "VANISHED" [SP "(EARLIER)"] SP known-uids
/// ```
pub(crate) fn message_data(input: &[u8]) -> IMAPResult<&[u8], Data> {
    #[derive(Clone)]
    enum TmpData<'a> {
        Expunge,
        Fetch(Vec1<MessageDataItem<'a>>),
        #[cfg(feature = "ext_condstore_qresync")]
        Vanished(bool, SequenceSet),
    }

    let (remaining, (seq, tmp)) = tuple((
        terminated(nz_number, sp),
        alt((
            value(TmpData::Expunge, tag_no_case(b"EXPUNGE")),
            map(preceded(tag_no_case(b"FETCH "), msg_att), TmpData::Fetch),
            #[cfg(feature = "ext_condstore_qresync")]
            map(
                tuple((
                    tag_no_case("VANISHED"),
                    opt(tag_no_case(" (EARLIER)")),
                    preceded(sp, sequence_set),
                )),
                |(_, earlier, known_uids)| TmpData::Vanished(earlier.is_some(), known_uids),
            ),
        )),
    ))(input)?;

    Ok((
        remaining,
        match tmp {
            TmpData::Expunge => Data::Expunge(seq),
            TmpData::Fetch(items) => Data::Fetch { seq, items },
            #[cfg(feature = "ext_condstore_qresync")]
            TmpData::Vanished(earlier, known_uids) => Data::Vanished {
                earlier,
                known_uids,
            },
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use imap_types::{
        body::{
            BasicFields, Body, BodyExtension, BodyStructure, Disposition, Language, Location,
            SinglePartExtensionData, SpecificFields,
        },
        core::{IString, NString, QuotedChar, Tag},
        flag::FlagNameAttribute,
    };

    use super::*;
    use crate::testing::{kat_inverse_greeting, kat_inverse_response, known_answer_test_encode};

    #[test]
    fn test_kat_inverse_greeting() {
        kat_inverse_greeting(&[
            (
                b"* OK [badcharset] ...\r\n".as_slice(),
                b"".as_slice(),
                Greeting::ok(Some(Code::BadCharset { allowed: vec![] }), "...").unwrap(),
            ),
            (
                b"* OK [UnSEEN 12345] ...\r\naaa".as_slice(),
                b"aaa".as_slice(),
                Greeting::ok(
                    Some(Code::Unseen(NonZeroU32::try_from(12345).unwrap())),
                    "...",
                )
                .unwrap(),
            ),
            (
                b"* OK [unseen 12345]  \r\n ".as_slice(),
                b" ".as_slice(),
                Greeting::ok(
                    Some(Code::Unseen(NonZeroU32::try_from(12345).unwrap())),
                    " ",
                )
                .unwrap(),
            ),
            (
                b"* PREAUTH [ALERT] hello\r\n".as_ref(),
                b"".as_ref(),
                Greeting::new(GreetingKind::PreAuth, Some(Code::Alert), "hello").unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_response_data() {
        kat_inverse_response(&[
            (
                b"* CAPABILITY IMAP4REV1\r\n".as_ref(),
                b"".as_ref(),
                Response::Data(Data::Capability(Vec1::from(Capability::Imap4Rev1))),
            ),
            (
                b"* LIST (\\Noselect) \"/\" bbb\r\n",
                b"",
                Response::Data(Data::List {
                    items: vec![FlagNameAttribute::Noselect],
                    delimiter: Some(QuotedChar::try_from('/').unwrap()),
                    mailbox: "bbb".try_into().unwrap(),
                }),
            ),
            (
                b"* SEARCH 1 2 3 42\r\n",
                b"",
                Response::Data(Data::Search(
                    vec![
                        1.try_into().unwrap(),
                        2.try_into().unwrap(),
                        3.try_into().unwrap(),
                        42.try_into().unwrap(),
                    ],
                    #[cfg(feature = "ext_condstore_qresync")]
                    None,
                )),
            ),
            (b"* 42 EXISTS\r\n", b"", Response::Data(Data::Exists(42))),
            (
                b"* 12345 RECENT\r\n",
                b"",
                Response::Data(Data::Recent(12345)),
            ),
            (
                b"* 123 EXPUNGE\r\n",
                b"",
                Response::Data(Data::Expunge(123.try_into().unwrap())),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_response_status() {
        kat_inverse_response(&[
            // tagged; Ok, No, Bad
            (
                b"A1 OK [ALERT] hello\r\n".as_ref(),
                b"".as_ref(),
                Response::Status(
                    Status::ok(
                        Some(Tag::try_from("A1").unwrap()),
                        Some(Code::Alert),
                        "hello",
                    )
                    .unwrap(),
                ),
            ),
            (
                b"A1 NO [ALERT] hello\r\n",
                b"".as_ref(),
                Response::Status(
                    Status::no(
                        Some(Tag::try_from("A1").unwrap()),
                        Some(Code::Alert),
                        "hello",
                    )
                    .unwrap(),
                ),
            ),
            (
                b"A1 BAD [ALERT] hello\r\n",
                b"".as_ref(),
                Response::Status(
                    Status::bad(
                        Some(Tag::try_from("A1").unwrap()),
                        Some(Code::Alert),
                        "hello",
                    )
                    .unwrap(),
                ),
            ),
            (
                b"A1 OK hello\r\n",
                b"".as_ref(),
                Response::Status(
                    Status::ok(Some(Tag::try_from("A1").unwrap()), None, "hello").unwrap(),
                ),
            ),
            (
                b"A1 NO hello\r\n",
                b"".as_ref(),
                Response::Status(
                    Status::no(Some(Tag::try_from("A1").unwrap()), None, "hello").unwrap(),
                ),
            ),
            (
                b"A1 BAD hello\r\n",
                b"".as_ref(),
                Response::Status(
                    Status::bad(Some(Tag::try_from("A1").unwrap()), None, "hello").unwrap(),
                ),
            ),
            // untagged; Ok, No, Bad
            (
                b"* OK [ALERT] hello\r\n",
                b"".as_ref(),
                Response::Status(Status::ok(None, Some(Code::Alert), "hello").unwrap()),
            ),
            (
                b"* NO [ALERT] hello\r\n",
                b"".as_ref(),
                Response::Status(Status::no(None, Some(Code::Alert), "hello").unwrap()),
            ),
            (
                b"* BAD [ALERT] hello\r\n",
                b"".as_ref(),
                Response::Status(Status::bad(None, Some(Code::Alert), "hello").unwrap()),
            ),
            (
                b"* OK hello\r\n",
                b"".as_ref(),
                Response::Status(Status::ok(None, None, "hello").unwrap()),
            ),
            (
                b"* NO hello\r\n",
                b"".as_ref(),
                Response::Status(Status::no(None, None, "hello").unwrap()),
            ),
            (
                b"* BAD hello\r\n",
                b"".as_ref(),
                Response::Status(Status::bad(None, None, "hello").unwrap()),
            ),
            // bye
            (
                b"* BYE [ALERT] hello\r\n",
                b"".as_ref(),
                Response::Status(Status::bye(Some(Code::Alert), "hello").unwrap()),
            ),
        ]);
    }

    /*
    // TODO(#184)
    #[test]
    fn test_kat_inverse_continue() {
        kat_inverse_continue(&[
            (
                b"+ \x01\r\n".as_ref(),
                b"".as_ref(),
                Continue::basic(None, "\x01").unwrap(),
            ),
            (
                b"+ hello\r\n".as_ref(),
                b"".as_ref(),
                Continue::basic(None, "hello").unwrap(),
            ),
            (
                b"+ [READ-WRITE] hello\r\n",
                b"",
                Continue::basic(Some(Code::ReadWrite), "hello").unwrap(),
            ),
        ]);
    }
    */

    #[test]
    fn test_encode_body_structure() {
        let tests = [
            (
                BodyStructure::Single {
                    body: Body {
                        basic: BasicFields {
                            parameter_list: vec![],
                            id: NString(None),
                            description: NString::try_from("description").unwrap(),
                            content_transfer_encoding: IString::try_from("cte").unwrap(),
                            size: 123,
                        },
                        specific: SpecificFields::Basic {
                            r#type: IString::try_from("application").unwrap(),
                            subtype: IString::try_from("voodoo").unwrap(),
                        },
                    },
                    extension_data: None,
                },
                b"(\"application\" \"voodoo\" NIL NIL \"description\" \"cte\" 123)".as_ref(),
            ),
            (
                BodyStructure::Single {
                    body: Body {
                        basic: BasicFields {
                            parameter_list: vec![],
                            id: NString(None),
                            description: NString::try_from("description").unwrap(),
                            content_transfer_encoding: IString::try_from("cte").unwrap(),
                            size: 123,
                        },
                        specific: SpecificFields::Text {
                            subtype: IString::try_from("plain").unwrap(),
                            number_of_lines: 14,
                        },
                    },
                    extension_data: None,
                },
                b"(\"TEXT\" \"plain\" NIL NIL \"description\" \"cte\" 123 14)",
            ),
            (
                BodyStructure::Single {
                    body: Body {
                        basic: BasicFields {
                            parameter_list: vec![],
                            id: NString(None),
                            description: NString::try_from("description").unwrap(),
                            content_transfer_encoding: IString::try_from("cte").unwrap(),
                            size: 123,
                        },
                        specific: SpecificFields::Text {
                            subtype: IString::try_from("plain").unwrap(),
                            number_of_lines: 14,
                        },
                    },
                    extension_data: Some(SinglePartExtensionData {
                        md5: NString::try_from("AABB").unwrap(),
                        tail: Some(Disposition {
                            disposition: None,
                            tail: Some(Language {
                                language: vec![],
                                tail: Some(Location {
                                    location: NString(None),
                                    extensions: vec![BodyExtension::List(Vec1::from(BodyExtension::Number(1337)))],
                                }),
                            }),
                        }),
                    }),
                },
                b"(\"TEXT\" \"plain\" NIL NIL \"description\" \"cte\" 123 14 \"AABB\" NIL NIL NIL (1337))",
            ),
        ];

        for test in tests {
            known_answer_test_encode(test);
        }
    }

    #[test]
    fn test_parse_response_negative() {
        let tests = [
            // TODO(#301,#184)
            // b"+ Nose[CAY a\r\n".as_ref()
        ];

        for test in tests {
            assert!(response(test).is_err());
        }
    }

    #[test]
    fn test_parse_resp_text_quirk() {
        #[cfg(not(feature = "quirk_missing_text"))]
        {
            assert!(resp_text(b"[IMAP4rev1]\r\n").is_err());
            assert!(resp_text(b"[IMAP4rev1]\r\n").is_err());
            assert!(resp_text(b"[IMAP4rev1] \r\n").is_err());
            assert!(resp_text(b"[IMAP4rev1]  \r\n").is_ok());
        }

        #[cfg(feature = "quirk_missing_text")]
        {
            assert!(resp_text(b"[IMAP4rev1]\r\n").is_ok());
            assert!(resp_text(b"[IMAP4rev1] \r\n").is_err());
            assert!(resp_text(b"[IMAP4rev1]  \r\n").is_ok());
        }
    }

    #[test]
    fn test_parse_resp_space_quirk() {
        assert!(response_data(b"* STATUS INBOX (MESSAGES 100 UNSEEN 0)\r\n").is_ok());
        assert!(response_data(b"* STATUS INBOX (MESSAGES 100 UNSEEN 0)  \r\n").is_err());

        #[cfg(not(feature = "quirk_trailing_space_status"))]
        assert!(response_data(b"* STATUS INBOX (MESSAGES 100 UNSEEN 0) \r\n").is_err());

        #[cfg(feature = "quirk_trailing_space_status")]
        assert!(response_data(b"* STATUS INBOX (MESSAGES 100 UNSEEN 0) \r\n").is_ok());
    }

    #[test]
    fn test_quirk_trailing_space_capability() {
        assert!(response_data(b"* CAPABILITY IMAP4REV1\r\n").is_ok());
        assert!(response_data(b"* CAPABILITY IMAP4REV1  \r\n").is_err());

        #[cfg(not(feature = "quirk_trailing_space_capability"))]
        assert!(response_data(b"* CAPABILITY IMAP4REV1 \r\n").is_err());

        #[cfg(feature = "quirk_trailing_space_capability")]
        assert!(response_data(b"* CAPABILITY IMAP4REV1 \r\n").is_ok());
    }
}
