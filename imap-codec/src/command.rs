use std::borrow::Cow;

#[cfg(not(feature = "quirk_crlf_relaxed"))]
use abnf_core::streaming::crlf;
#[cfg(feature = "quirk_crlf_relaxed")]
use abnf_core::streaming::crlf_relaxed as crlf;
use abnf_core::streaming::sp;
#[cfg(feature = "ext_condstore_qresync")]
use imap_types::command::{FetchModifier, SelectParameter, StoreModifier};
use imap_types::{
    auth::AuthMechanism,
    command::{Command, CommandBody},
    core::AString,
    extensions::binary::LiteralOrLiteral8,
    fetch::{Macro, MacroOrMessageDataItemNames},
    flag::{Flag, StoreResponse, StoreType},
    secret::Secret,
};
#[cfg(feature = "ext_condstore_qresync")]
use nom::character::streaming::char;
#[cfg(feature = "ext_condstore_qresync")]
use nom::sequence::separated_pair;
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, opt, value},
    multi::{separated_list0, separated_list1},
    sequence::{delimited, preceded, terminated, tuple},
};

#[cfg(feature = "ext_condstore_qresync")]
use crate::core::nz_number;
#[cfg(feature = "ext_condstore_qresync")]
use crate::extensions::condstore_qresync::mod_sequence_value;
#[cfg(feature = "ext_condstore_qresync")]
use crate::extensions::condstore_qresync::mod_sequence_valzer;
#[cfg(feature = "ext_id")]
use crate::extensions::id::id;
#[cfg(feature = "ext_metadata")]
use crate::extensions::metadata::{getmetadata, setmetadata};
#[cfg(feature = "ext_namespace")]
use crate::extensions::namespace::namespace_command;
use crate::{
    auth::auth_type,
    core::{astring, base64, literal, tag_imap},
    datetime::date_time,
    decode::{IMAPErrorKind, IMAPResult},
    extensions::{
        binary::literal8,
        compress::compress,
        enable::enable,
        idle::idle,
        r#move::r#move,
        quota::{getquota, getquotaroot, setquota},
        sort::sort,
        thread::thread,
        uidplus::uid_expunge,
    },
    fetch::fetch_att,
    flag::{flag, flag_list},
    mailbox::{list_mailbox, mailbox},
    search::search,
    sequence::sequence_set,
    status::status_att,
};

/// `command = tag SP (
///                     command-any /
///                     command-auth /
///                     command-nonauth /
///                     command-select
///                   ) CRLF`
pub(crate) fn command(input: &[u8]) -> IMAPResult<&[u8], Command> {
    let mut parser_tag = terminated(tag_imap, sp);
    let mut parser_body = terminated(
        alt((command_any, command_auth, command_nonauth, command_select)),
        crlf,
    );

    let (remaining, obtained_tag) = parser_tag(input)?;

    match parser_body(remaining) {
        Ok((remaining, body)) => Ok((
            remaining,
            Command {
                tag: obtained_tag,
                body,
            },
        )),
        Err(mut error) => {
            // If we got an `IMAPErrorKind::Literal`, we fill in the missing `tag`.
            if let nom::Err::Error(ref mut err) | nom::Err::Failure(ref mut err) = error {
                if let IMAPErrorKind::Literal { ref mut tag, .. } = err.kind {
                    *tag = Some(obtained_tag);
                }
            }

            Err(error)
        }
    }
}

// # Command Any

/// ```abnf
/// command-any = "CAPABILITY" /
///               "LOGOUT" /
///               "NOOP" /
///               x-command /
///               id ; adds id command to command_any (See RFC 2971)
/// ```
///
/// Note: Valid in all states
pub(crate) fn command_any(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    alt((
        value(CommandBody::Capability, tag_no_case(b"CAPABILITY")),
        value(CommandBody::Logout, tag_no_case(b"LOGOUT")),
        value(CommandBody::Noop, tag_no_case(b"NOOP")),
        // x-command = "X" atom <experimental command arguments>
        #[cfg(feature = "ext_id")]
        map(id, |parameters| CommandBody::Id { parameters }),
    ))(input)
}

// # Command Auth

/// ```abnf
/// command-auth = append /
///                create /
///                delete /
///                examine /
///                list /
///                lsub /
///                rename /
///                select /
///                status /
///                subscribe /
///                unsubscribe /
///                idle /         ; RFC 2177
///                enable /       ; RFC 5161
///                compress /     ; RFC 4978
///                getquota /     ; RFC 9208
///                getquotaroot / ; RFC 9208
///                setquota /     ; RFC 9208
///                setmetadata /  ; RFC 5464
///                getmetadata    ; RFC 5464
/// ```
///
/// Note: Valid only in Authenticated or Selected state
pub(crate) fn command_auth(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    alt((
        append,
        create,
        delete,
        examine,
        list,
        lsub,
        rename,
        select,
        status,
        subscribe,
        unsubscribe,
        idle,
        enable,
        compress,
        getquota,
        getquotaroot,
        setquota,
        #[cfg(feature = "ext_metadata")]
        setmetadata,
        #[cfg(feature = "ext_metadata")]
        getmetadata,
        #[cfg(feature = "ext_namespace")]
        namespace_command,
    ))(input)
}

/// `append = "APPEND" SP mailbox [SP flag-list] [SP date-time] SP literal`
pub(crate) fn append(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case(b"APPEND "),
        mailbox,
        opt(preceded(sp, flag_list)),
        opt(preceded(sp, date_time)),
        sp,
        alt((
            map(literal, LiteralOrLiteral8::Literal),
            map(literal8, LiteralOrLiteral8::Literal8),
        )),
    ));

    let (remaining, (_, mailbox, flags, date, _, message)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Append {
            mailbox,
            flags: flags.unwrap_or_default(),
            date,
            message,
        },
    ))
}

/// `create = "CREATE" SP mailbox`
///
/// Note: Use of INBOX gives a NO error
pub(crate) fn create(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = preceded(tag_no_case(b"CREATE "), mailbox);

    let (remaining, mailbox) = parser(input)?;

    Ok((remaining, CommandBody::Create { mailbox }))
}

/// `delete = "DELETE" SP mailbox`
///
/// Note: Use of INBOX gives a NO error
pub(crate) fn delete(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = preceded(tag_no_case(b"DELETE "), mailbox);

    let (remaining, mailbox) = parser(input)?;

    Ok((remaining, CommandBody::Delete { mailbox }))
}

/// ```abnf
/// examine = "EXAMINE" SP mailbox [select-params]
///                                ^^^^^^^^^^^^^^^
///                                |
///                                RFC 4466: modifies the original IMAP EXAMINE command to accept optional parameters
/// ```
pub(crate) fn examine(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = preceded(tag_no_case(b"EXAMINE "), mailbox);

    let (remaining, mailbox) = parser(input)?;

    #[cfg(feature = "ext_condstore_qresync")]
    let (remaining, parameters) =
        map(opt(select_params), |params| params.unwrap_or_default())(remaining)?;

    Ok((
        remaining,
        CommandBody::Examine {
            mailbox,
            #[cfg(feature = "ext_condstore_qresync")]
            parameters,
        },
    ))
}

/// `list = "LIST" SP mailbox SP list-mailbox`
pub(crate) fn list(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case(b"LIST "), mailbox, sp, list_mailbox));

    let (remaining, (_, reference, _, mailbox_wildcard)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::List {
            reference,
            mailbox_wildcard,
        },
    ))
}

/// `lsub = "LSUB" SP mailbox SP list-mailbox`
pub(crate) fn lsub(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case(b"LSUB "), mailbox, sp, list_mailbox));

    let (remaining, (_, reference, _, mailbox_wildcard)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Lsub {
            reference,
            mailbox_wildcard,
        },
    ))
}

/// `rename = "RENAME" SP mailbox SP mailbox`
///
/// Note: Use of INBOX as a destination gives a NO error
pub(crate) fn rename(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case(b"RENAME "), mailbox, sp, mailbox));

    let (remaining, (_, mailbox, _, new_mailbox)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Rename {
            from: mailbox,
            to: new_mailbox,
        },
    ))
}

/// ```abnf
/// select = "SELECT" SP mailbox [select-params]
///                              ^^^^^^^^^^^^^^^
///                              |
///                              RFC 4466: modifies the original IMAP SELECT command to accept optional parameters
/// ```
pub(crate) fn select(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = preceded(tag_no_case(b"SELECT "), mailbox);

    let (remaining, mailbox) = parser(input)?;

    #[cfg(feature = "ext_condstore_qresync")]
    let (remaining, parameters) =
        map(opt(select_params), |params| params.unwrap_or_default())(remaining)?;

    Ok((
        remaining,
        CommandBody::Select {
            mailbox,
            #[cfg(feature = "ext_condstore_qresync")]
            parameters,
        },
    ))
}

/// FROM RFC4466:
///
/// ```abnf
/// select-params = SP "(" select-param *(SP select-param) ")"
/// ```
#[cfg(feature = "ext_condstore_qresync")]
pub(crate) fn select_params(input: &[u8]) -> IMAPResult<&[u8], Vec<SelectParameter>> {
    delimited(tag(" ("), separated_list1(sp, select_param), tag(")"))(input)
}

/// FROM RFC4466:
///
/// ```abnf
/// select-param = select-param-name [SP select-param-value]
///                ;; a parameter to SELECT may contain one or more atoms and/or strings and/or lists.
///
/// select-param-name = tagged-ext-label
///
/// select-param-value = tagged-ext-val
///                      ;; This non-terminal shows recommended syntax for future extensions.
/// ```
///
/// FROM RFC 7162 (CONDSTORE/QRESYNC):
///
/// ```abnf
/// select-param =/ condstore-param
///              ;; Conforms to the generic "select-param" non-terminal syntax defined in [RFC4466].
///
/// condstore-param = "CONDSTORE"
///
/// select-param =/ "QRESYNC" SP "("
///                   uidvalidity SP
///                   mod-sequence-value [SP known-uids]
///                   [SP seq-match-data]
///                 ")"
///                 ;; Conforms to the generic select-param syntax defined in [RFC4466].
///
/// uidvalidity = nz-number
///
/// known-uids = sequence-set
///              ;; Sequence of UIDs; "*" is not allowed.
///
/// seq-match-data = "(" known-sequence-set SP known-uid-set ")"
///
/// known-sequence-set = sequence-set
///                    ;; Set of message numbers corresponding to
///                    ;; the UIDs in known-uid-set, in ascending order.
///                    ;; * is not allowed.
///
/// known-uid-set = sequence-set
///                 ;; Set of UIDs corresponding to the messages in
///                 ;; known-sequence-set, in ascending order.
///                 ;; * is not allowed.
/// ```
#[cfg(feature = "ext_condstore_qresync")]
pub(crate) fn select_param(input: &[u8]) -> IMAPResult<&[u8], SelectParameter> {
    alt((
        value(SelectParameter::CondStore, tag_no_case("CONDSTORE")),
        map(
            delimited(
                tag_no_case("QRESYNC ("),
                tuple((
                    terminated(nz_number, sp),
                    mod_sequence_value,
                    opt(preceded(sp, sequence_set)),
                    opt(preceded(
                        sp,
                        delimited(
                            char('('),
                            separated_pair(sequence_set, sp, sequence_set),
                            char(')'),
                        ),
                    )),
                )),
                char(')'),
            ),
            |(uid_validity, mod_sequence_value, known_uids, seq_match_data)| {
                SelectParameter::QResync {
                    uid_validity,
                    mod_sequence_value,
                    known_uids,
                    seq_match_data,
                }
            },
        ),
    ))(input)
}

/// `status = "STATUS" SP mailbox SP "(" status-att *(SP status-att) ")"`
pub(crate) fn status(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case(b"STATUS "),
        mailbox,
        delimited(tag(b" ("), separated_list0(sp, status_att), tag(b")")),
    ));

    let (remaining, (_, mailbox, item_names)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Status {
            mailbox,
            item_names: item_names.into(),
        },
    ))
}

/// `subscribe = "SUBSCRIBE" SP mailbox`
pub(crate) fn subscribe(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = preceded(tag_no_case(b"SUBSCRIBE "), mailbox);

    let (remaining, mailbox) = parser(input)?;

    Ok((remaining, CommandBody::Subscribe { mailbox }))
}

/// `unsubscribe = "UNSUBSCRIBE" SP mailbox`
pub(crate) fn unsubscribe(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = preceded(tag_no_case(b"UNSUBSCRIBE "), mailbox);

    let (remaining, mailbox) = parser(input)?;

    Ok((remaining, CommandBody::Unsubscribe { mailbox }))
}

// # Command NonAuth

/// `command-nonauth = login / authenticate / "STARTTLS"`
///
/// Note: Valid only when in Not Authenticated state
pub(crate) fn command_nonauth(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = alt((
        login,
        map(authenticate, |(mechanism, initial_response)| {
            CommandBody::Authenticate {
                mechanism,
                initial_response,
            }
        }),
        #[cfg(feature = "starttls")]
        value(CommandBody::StartTLS, tag_no_case(b"STARTTLS")),
    ));

    let (remaining, parsed_command_nonauth) = parser(input)?;

    Ok((remaining, parsed_command_nonauth))
}

/// `login = "LOGIN" SP userid SP password`
pub(crate) fn login(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case(b"LOGIN"), sp, userid, sp, password));

    let (remaining, (_, _, username, _, password)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Login {
            username,
            password: Secret::new(password),
        },
    ))
}

#[inline]
/// `userid = astring`
pub(crate) fn userid(input: &[u8]) -> IMAPResult<&[u8], AString> {
    astring(input)
}

#[inline]
/// `password = astring`
pub(crate) fn password(input: &[u8]) -> IMAPResult<&[u8], AString> {
    astring(input)
}

/// `authenticate = "AUTHENTICATE" SP auth-type *(CRLF base64)` (edited)
///
/// ```text
///                                            Added by SASL-IR
///                                            |
///                                            vvvvvvvvvvvvvvvvvvv
/// authenticate = "AUTHENTICATE" SP auth-type [SP (base64 / "=")] *(CRLF base64)
///                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///                |
///                This is parsed here.
///                CRLF is parsed by upper command parser.
/// ```
#[allow(clippy::type_complexity)]
pub(crate) fn authenticate(
    input: &[u8],
) -> IMAPResult<&[u8], (AuthMechanism, Option<Secret<Cow<[u8]>>>)> {
    let mut parser = tuple((
        tag_no_case(b"AUTHENTICATE "),
        auth_type,
        opt(preceded(
            sp,
            alt((
                map(base64, |data| Secret::new(Cow::Owned(data))),
                value(Secret::new(Cow::Borrowed(&b""[..])), tag("=")),
            )),
        )),
    ));

    let (remaining, (_, auth_type, raw_data)) = parser(input)?;

    // Server must send continuation ("+ ") at this point...

    Ok((remaining, (auth_type, raw_data)))
}

// # Command Select

/// `command-select = "CHECK" /
///                   "CLOSE" /
///                   "EXPUNGE" /
///                   copy /
///                   fetch /
///                   store /
///                   uid /
///                   search`
///
/// Note: Valid only when in Selected state
pub(crate) fn command_select(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    alt((
        value(CommandBody::Check, tag_no_case(b"CHECK")),
        value(CommandBody::Close, tag_no_case(b"CLOSE")),
        value(CommandBody::Expunge, tag_no_case(b"EXPUNGE")),
        uid_expunge,
        copy,
        fetch,
        store,
        uid,
        search,
        sort,
        thread,
        value(CommandBody::Unselect, tag_no_case(b"UNSELECT")),
        r#move,
    ))(input)
}

/// `copy = "COPY" SP sequence-set SP mailbox`
pub(crate) fn copy(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case(b"COPY"), sp, sequence_set, sp, mailbox));

    let (remaining, (_, _, sequence_set, _, mailbox)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Copy {
            sequence_set,
            mailbox,
            uid: false,
        },
    ))
}

/// ```abnf
/// fetch = "FETCH" SP sequence-set SP ("ALL" /
///                                     "FULL" /
///                                     "FAST" /
///                                     fetch-att / "(" fetch-att *(SP fetch-att) ")")
///                                     [fetch-modifiers] ; FROM RFC 4466
/// ```
pub(crate) fn fetch(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case(b"FETCH"),
        preceded(sp, sequence_set),
        preceded(
            sp,
            alt((
                value(
                    MacroOrMessageDataItemNames::Macro(Macro::All),
                    tag_no_case(b"ALL"),
                ),
                value(
                    MacroOrMessageDataItemNames::Macro(Macro::Fast),
                    tag_no_case(b"FAST"),
                ),
                value(
                    MacroOrMessageDataItemNames::Macro(Macro::Full),
                    tag_no_case(b"FULL"),
                ),
                map(fetch_att, |fetch_att| {
                    MacroOrMessageDataItemNames::MessageDataItemNames(vec![fetch_att])
                }),
                map(
                    delimited(tag(b"("), separated_list0(sp, fetch_att), tag(b")")),
                    MacroOrMessageDataItemNames::MessageDataItemNames,
                ),
            )),
        ),
        #[cfg(feature = "ext_condstore_qresync")]
        map(opt(fetch_modifiers), Option::unwrap_or_default),
    ));

    #[cfg(not(feature = "ext_condstore_qresync"))]
    let (remaining, (_, sequence_set, macro_or_item_names)) = parser(input)?;

    #[cfg(feature = "ext_condstore_qresync")]
    let (remaining, (_, sequence_set, macro_or_item_names, modifiers)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Fetch {
            sequence_set,
            macro_or_item_names,
            uid: false,
            #[cfg(feature = "ext_condstore_qresync")]
            modifiers,
        },
    ))
}

#[cfg(feature = "ext_condstore_qresync")]
/// From RFC 4466:
///
/// ```abnf
/// fetch-modifiers = SP "(" fetch-modifier *(SP fetch-modifier) ")"
/// ```
pub(crate) fn fetch_modifiers(input: &[u8]) -> IMAPResult<&[u8], Vec<FetchModifier>> {
    delimited(tag(" ("), separated_list1(sp, fetch_modifier), char(')'))(input)
}

#[cfg(feature = "ext_condstore_qresync")]
/// From RFC 4466:
///
/// ```abnf
/// fetch-modifier = fetch-modifier-name [ SP fetch-modif-params ]
///
/// fetch-modif-params = tagged-ext-val
///                      ;; This non-terminal shows recommended syntax
///                      ;; for future extensions.
///
/// fetch-modifier-name = tagged-ext-label
/// ```
///
/// From RFC 7162 (CONDSTORE/QRESYNC):
///
/// ```abnf
/// fetch-modifier =/ chgsince-fetch-mod
///                   ;; Conforms to the generic "fetch-modifier" syntax defined in [RFC4466].
///
/// chgsince-fetch-mod = "CHANGEDSINCE" SP mod-sequence-value
///                      ;; CHANGEDSINCE FETCH modifier conforms to the fetch-modifier syntax.
///
/// rexpunges-fetch-mod = "VANISHED"
///                     ;; VANISHED UID FETCH modifier conforms to the fetch-modifier syntax defined in [RFC4466].
///                     ;; It is only allowed in the UID FETCH command.
/// ```
pub(crate) fn fetch_modifier(input: &[u8]) -> IMAPResult<&[u8], FetchModifier> {
    alt((
        map(
            preceded(tag_no_case("CHANGEDSINCE "), mod_sequence_value),
            FetchModifier::ChangedSince,
        ),
        value(FetchModifier::Vanished, tag_no_case("VANISHED")),
    ))(input)
}

/// ```abnf
/// store = "STORE" SP sequence-set [store-modifiers] SP store-att-flags
///                                 ^^^^^^^^^^^^^^^^^
///                                 |
///                                 RFC 4466
/// ```
pub(crate) fn store(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case(b"STORE"),
        preceded(sp, sequence_set),
        #[cfg(feature = "ext_condstore_qresync")]
        map(opt(store_modifiers), Option::unwrap_or_default),
        preceded(sp, store_att_flags),
    ));

    #[cfg(not(feature = "ext_condstore_qresync"))]
    let (remaining, (_, sequence_set, (kind, response, flags))) = parser(input)?;

    #[cfg(feature = "ext_condstore_qresync")]
    let (remaining, (_, sequence_set, modifiers, (kind, response, flags))) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Store {
            sequence_set,
            kind,
            response,
            flags,
            uid: false,
            #[cfg(feature = "ext_condstore_qresync")]
            modifiers,
        },
    ))
}

#[cfg(feature = "ext_condstore_qresync")]
/// From RFC 4466:
///
/// ```abnf
/// store-modifiers = SP "(" store-modifier *(SP store-modifier) ")"
/// ```
pub(crate) fn store_modifiers(input: &[u8]) -> IMAPResult<&[u8], Vec<StoreModifier>> {
    delimited(tag(" ("), separated_list1(sp, store_modifier), char(')'))(input)
}

#[cfg(feature = "ext_condstore_qresync")]
/// From RFC 4466:
///
/// ```abnf
/// store-modifier = store-modifier-name [SP store-modif-params]
///
/// store-modif-params = tagged-ext-val
///
/// store-modifier-name = tagged-ext-label
/// ```
///
/// From RFC 7162 (CONDSTORE/QRESYNC):
///
/// ```abnf
/// store-modifier =/ "UNCHANGEDSINCE" SP mod-sequence-valzer
///                ;; Only a single "UNCHANGEDSINCE" may be specified in a STORE operation.
/// ```
pub(crate) fn store_modifier(input: &[u8]) -> IMAPResult<&[u8], StoreModifier> {
    map(
        preceded(tag_no_case(b"UNCHANGEDSINCE "), mod_sequence_valzer),
        StoreModifier::UnchangedSince,
    )(input)
}

/// `store-att-flags = (["+" / "-"] "FLAGS" [".SILENT"]) SP (flag-list / (flag *(SP flag)))`
pub(crate) fn store_att_flags(
    input: &[u8],
) -> IMAPResult<&[u8], (StoreType, StoreResponse, Vec<Flag>)> {
    let mut parser = tuple((
        tuple((
            map(
                opt(alt((
                    value(StoreType::Add, tag(b"+")),
                    value(StoreType::Remove, tag(b"-")),
                ))),
                |type_| match type_ {
                    Some(type_) => type_,
                    None => StoreType::Replace,
                },
            ),
            tag_no_case(b"FLAGS"),
            map(opt(tag_no_case(b".SILENT")), |x| match x {
                Some(_) => StoreResponse::Silent,
                None => StoreResponse::Answer,
            }),
        )),
        sp,
        alt((flag_list, separated_list1(sp, flag))),
    ));

    let (remaining, ((store_type, _, store_response), _, flag_list)) = parser(input)?;

    Ok((remaining, (store_type, store_response, flag_list)))
}

/// `uid = "UID" SP (copy / fetch / search / store)`
///
/// Note: Unique identifiers used instead of message sequence numbers
pub(crate) fn uid(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case(b"UID"),
        sp,
        alt((copy, fetch, search, store, r#move)),
    ));

    let (remaining, (_, _, mut cmd)) = parser(input)?;

    match cmd {
        CommandBody::Copy { ref mut uid, .. }
        | CommandBody::Fetch { ref mut uid, .. }
        | CommandBody::Search { ref mut uid, .. }
        | CommandBody::Store { ref mut uid, .. }
        | CommandBody::Move { ref mut uid, .. } => *uid = true,
        _ => unreachable!(),
    }

    Ok((remaining, cmd))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use imap_types::{
        core::Tag,
        fetch::{MessageDataItemName, Section},
    };

    use super::*;
    use crate::{CommandCodec, encode::Encoder};

    #[test]
    fn test_parse_fetch() {
        println!("{:#?}", fetch(b"fetch 1:1 (flags)???"));
    }

    #[test]
    fn test_parse_fetch_att() {
        let tests = [
            (MessageDataItemName::Envelope, "ENVELOPE???"),
            (MessageDataItemName::Flags, "FLAGS???"),
            (MessageDataItemName::InternalDate, "INTERNALDATE???"),
            (MessageDataItemName::Rfc822, "RFC822???"),
            (MessageDataItemName::Rfc822Header, "RFC822.HEADER???"),
            (MessageDataItemName::Rfc822Size, "RFC822.SIZE???"),
            (MessageDataItemName::Rfc822Text, "RFC822.TEXT???"),
            (MessageDataItemName::Body, "BODY???"),
            (MessageDataItemName::BodyStructure, "BODYSTRUCTURE???"),
            (MessageDataItemName::Uid, "UID???"),
            (
                MessageDataItemName::BodyExt {
                    partial: None,
                    peek: false,
                    section: None,
                },
                "BODY[]???",
            ),
            (
                MessageDataItemName::BodyExt {
                    partial: None,
                    peek: true,
                    section: None,
                },
                "BODY.PEEK[]???",
            ),
            (
                MessageDataItemName::BodyExt {
                    partial: None,
                    peek: true,
                    section: Some(Section::Text(None)),
                },
                "BODY.PEEK[TEXT]???",
            ),
            (
                MessageDataItemName::BodyExt {
                    partial: Some((42, NonZeroU32::try_from(1337).unwrap())),
                    peek: true,
                    section: Some(Section::Text(None)),
                },
                "BODY.PEEK[TEXT]<42.1337>???",
            ),
        ];

        let expected_remainder = "???".as_bytes();

        for (expected, test) in tests {
            let (got_remainder, got) = fetch_att(test.as_bytes()).unwrap();

            assert_eq!(expected, got);
            assert_eq!(expected_remainder, got_remainder);
        }
    }

    #[test]
    fn test_that_empty_ir_is_encoded_correctly() {
        let command = Command::new(
            Tag::try_from("A").unwrap(),
            CommandBody::Authenticate {
                mechanism: AuthMechanism::Plain,
                initial_response: Some(Secret::new(Cow::Borrowed(&b""[..]))),
            },
        )
        .unwrap();

        let buffer = CommandCodec::default().encode(&command).dump();

        assert_eq!(buffer, b"A AUTHENTICATE PLAIN =\r\n")
    }
}
