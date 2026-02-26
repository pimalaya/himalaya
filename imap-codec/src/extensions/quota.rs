//! IMAP QUOTA Extension

use std::io::Write;

use abnf_core::streaming::sp;
use imap_types::{
    command::CommandBody,
    core::{AString, Vec1},
    extensions::quota::{QuotaGet, QuotaSet, Resource},
    response::Data,
};
#[cfg(feature = "quirk_excessive_space_quota_resource")]
use nom::multi::many1;
use nom::{
    bytes::streaming::{tag, tag_no_case},
    combinator::map,
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, preceded, tuple},
};

use crate::{
    core::{astring, atom, number64},
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext},
    mailbox::mailbox,
};

/// ```abnf
/// quota-root-name = astring
/// ```
#[inline]
pub(crate) fn quota_root_name(input: &[u8]) -> IMAPResult<&[u8], AString> {
    astring(input)
}

/// ```abnf
/// getquota = "GETQUOTA" SP quota-root-name
/// ```
#[inline]
pub(crate) fn getquota(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case("GETQUOTA "), quota_root_name));

    let (remaining, (_, root)) = parser(input)?;

    Ok((remaining, CommandBody::GetQuota { root }))
}

/// ```abnf
/// getquotaroot = "GETQUOTAROOT" SP mailbox
/// ```
pub(crate) fn getquotaroot(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case("GETQUOTAROOT "), mailbox));

    let (remaining, (_, mailbox)) = parser(input)?;

    Ok((remaining, CommandBody::GetQuotaRoot { mailbox }))
}

/// ```abnf
/// quota-resource = resource-name SP
///                  resource-usage SP
///                  resource-limit
///
/// resource-usage = number64
///
/// resource-limit = number64
/// ```
pub(crate) fn quota_resource(input: &[u8]) -> IMAPResult<&[u8], QuotaGet> {
    #[cfg(not(feature = "quirk_excessive_space_quota_resource"))]
    let mut parser = tuple((resource_name, sp, number64, sp, number64));

    #[cfg(feature = "quirk_excessive_space_quota_resource")]
    let mut parser = tuple((resource_name, many1(sp), number64, many1(sp), number64));

    let (remaining, (resource, _, usage, _, limit)) = parser(input)?;

    Ok((
        remaining,
        QuotaGet {
            resource,
            usage,
            limit,
        },
    ))
}

/// ```abnf
/// resource-name = "STORAGE" /
///                 "MESSAGE" /
///                 "MAILBOX" /
///                 "ANNOTATION-STORAGE" /
///                 resource-name-ext
///
/// resource-name-ext = atom
/// ```
pub(crate) fn resource_name(input: &[u8]) -> IMAPResult<&[u8], Resource> {
    map(atom, Resource::from)(input)
}

/// ```abnf
/// quota-response = "QUOTA" SP quota-root-name SP quota-list
///
/// quota-list = "(" quota-resource *(SP quota-resource) ")"
/// ```
pub(crate) fn quota_response(input: &[u8]) -> IMAPResult<&[u8], Data> {
    let mut parser = tuple((
        tag_no_case("QUOTA "),
        quota_root_name,
        delimited(tag(" ("), separated_list1(sp, quota_resource), tag(")")),
    ));

    let (remaining, (_, root, quotas)) = parser(input)?;

    Ok((
        remaining,
        Data::Quota {
            root,
            // Safety: Safe because we use `separated_list1` above.
            quotas: Vec1::try_from(quotas).unwrap(),
        },
    ))
}

/// ```abnf
/// quotaroot-response = "QUOTAROOT" SP mailbox *(SP quota-root-name)
/// ```
pub(crate) fn quotaroot_response(input: &[u8]) -> IMAPResult<&[u8], Data> {
    let mut parser = tuple((
        tag_no_case("QUOTAROOT "),
        mailbox,
        many0(preceded(sp, quota_root_name)),
    ));

    let (remaining, (_, mailbox, roots)) = parser(input)?;

    Ok((remaining, Data::QuotaRoot { mailbox, roots }))
}

/// ```abnf
/// setquota = "SETQUOTA" SP quota-root-name SP setquota-list
///
/// setquota-list = "(" [setquota-resource *(SP setquota-resource)] ")"
/// ```
pub(crate) fn setquota(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case("SETQUOTA "),
        quota_root_name,
        delimited(tag(" ("), separated_list0(sp, setquota_resource), tag(")")),
    ));

    let (remaining, (_, root, quotas)) = parser(input)?;

    Ok((remaining, CommandBody::SetQuota { root, quotas }))
}

/// ```abnf
/// setquota-resource = resource-name SP resource-limit
/// ```
pub(crate) fn setquota_resource(input: &[u8]) -> IMAPResult<&[u8], QuotaSet> {
    let mut parser = tuple((resource_name, sp, number64));

    let (remaining, (resource, _, limit)) = parser(input)?;

    Ok((remaining, QuotaSet { resource, limit }))
}

// This had to be inlined into the `capability` parser because `CapabilityOther("QUOTAFOO")` would
// be parsed as `Capability::Quota` plus an erroneous remainder. The `capability` parser eagerly consumes
// an `atom` and tries to detect the variants later.
// /// ```abnf
// /// capability-quota = "QUOTASET" / capa-quota-res
// /// ```
// ///
// /// Note: Extended to ...
// ///
// /// ```abnf
// /// capability-quota = "QUOTASET" / capa-quota-res / "QUOTA"
// /// ```
// pub(crate) fn capability_quota(input: &[u8]) -> IMAPResult<&[u8], Capability> {
//     alt((
//         value(Capability::QuotaSet, tag_no_case("QUOTASET")),
//         capa_quota_res,
//         value(Capability::Quota, tag_no_case("QUOTA")),
//     ))(input)
// }

// /// ```abnf
// /// capa-quota-res = "QUOTA=RES-" resource-name
// /// ```
// pub(crate) fn capa_quota_res(input: &[u8]) -> IMAPResult<&[u8], Capability> {
//     let mut parser = preceded(tag_no_case("QUOTA=RES-"), resource_name);
//
//     let (remaining, resource) = parser(input)?;
//
//     Ok((remaining, Capability::QuotaRes(resource)))
// }

impl EncodeIntoContext for Resource<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.to_string().as_bytes())
    }
}

impl EncodeIntoContext for QuotaGet<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.resource.encode_ctx(ctx)?;
        write!(ctx, " {} {}", self.usage, self.limit)
    }
}

impl EncodeIntoContext for QuotaSet<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.resource.encode_ctx(ctx)?;
        write!(ctx, " {}", self.limit)
    }
}

#[cfg(test)]
mod tests {
    use imap_types::{
        command::{Command, CommandBody},
        core::{IString, Tag},
        extensions::quota::{QuotaGet, QuotaSet, Resource},
        mailbox::Mailbox,
        response::{Capability, Code, Response, Status},
        status::{StatusDataItem, StatusDataItemName},
    };

    use super::*;
    use crate::testing::{kat_inverse_command, kat_inverse_response};

    #[test]
    fn test_parse_resource_name() {
        let tests = [
            (b"stOragE ".as_ref(), Resource::Storage),
            (b"mesSaGe ".as_ref(), Resource::Message),
            (b"maIlbOx ".as_ref(), Resource::Mailbox),
            (b"anNotatIon-stoRage ".as_ref(), Resource::AnnotationStorage),
            (
                b"anNotatIon-stoRageX ".as_ref(),
                Resource::try_from(b"anNotatIon-stoRageX".as_ref()).unwrap(),
            ),
            (
                b"anNotatIon-stoRagee ".as_ref(),
                Resource::try_from(b"anNotatIon-stoRagee".as_ref()).unwrap(),
            ),
        ];

        for (test, expected) in tests.iter() {
            let (rem, got) = resource_name(test).unwrap();
            assert_eq!(*expected, got);
            assert_eq!(rem, b" ");
        }
    }

    #[test]
    fn test_kat_inverse_command_get_quota() {
        kat_inverse_command(&[
            (
                b"A GETQUOTA INBOX\r\n".as_ref(),
                b"".as_ref(),
                Command::new("A", CommandBody::get_quota("INBOX").unwrap()).unwrap(),
            ),
            (
                b"A GETQUOTA \"\"\r\n?",
                b"?",
                Command::new("A", CommandBody::get_quota("").unwrap()).unwrap(),
            ),
            (
                b"A003 GETQUOTA \"\"\r\n",
                b"",
                CommandBody::get_quota("").unwrap().tag("A003").unwrap(),
            ),
            (
                b"G0001 GETQUOTA \"!partition/sda4\"\r\n",
                b"",
                CommandBody::get_quota(AString::String(
                    IString::try_from("!partition/sda4").unwrap(),
                ))
                .unwrap()
                .tag("G0001")
                .unwrap(),
            ),
            (
                b"S0000 GETQUOTA \"#user/alice\"\r\n",
                b"",
                CommandBody::get_quota(AString::String(IString::try_from("#user/alice").unwrap()))
                    .unwrap()
                    .tag("S0000")
                    .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_command_get_quota_root() {
        kat_inverse_command(&[
            (
                b"A003 GETQUOTAROOT INBOX\r\n".as_ref(),
                b"".as_ref(),
                CommandBody::get_quota_root(Mailbox::Inbox)
                    .unwrap()
                    .tag("A003")
                    .unwrap(),
            ),
            (
                b"A GETQUOTAROOT MAILBOX\r\n??",
                b"??",
                Command::new("A", CommandBody::get_quota_root("MAILBOX").unwrap()).unwrap(),
            ),
            (
                b"G0002 GETQUOTAROOT INBOX\r\n",
                b"",
                CommandBody::get_quota_root("inbox")
                    .unwrap()
                    .tag("G0002")
                    .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_command_set_quota() {
        kat_inverse_command(&[
            (
                b"A SETQUOTA INBOX ()\r\n".as_ref(),
                b"".as_ref(),
                Command::new("A", CommandBody::set_quota("INBOX", vec![]).unwrap()).unwrap(),
            ),
            (
                b"A SETQUOTA INBOX (STORAGE 256)\r\n",
                b"",
                Command::new("A", CommandBody::set_quota(
                    "INBOX",
                    vec![QuotaSet {
                        resource: Resource::Storage,
                        limit: 256,
                    }],
                )
                    .unwrap()).unwrap(),
            ),
            (
                b"A SETQUOTA INBOX (STORAGE 0 MESSAGE 512 MAILBOX 512 ANNOTATION-STORAGE 123 Foo 18446744073709551615)\r\n",
                b"",
                Command::new("A", CommandBody::set_quota(
                    "INBOX",
                    vec![
                        QuotaSet {
                            resource: Resource::Storage,
                            limit: 0,
                        },
                        QuotaSet {
                            resource: Resource::Message,
                            limit: 512,
                        },
                        QuotaSet {
                            resource: Resource::Mailbox,
                            limit: 512,
                        },
                        QuotaSet {
                            resource: Resource::AnnotationStorage,
                            limit: 123,
                        },
                        QuotaSet {
                            resource: Resource::try_from("Foo").unwrap(),
                            limit: u64::MAX,
                        },
                    ],
                )
                    .unwrap()).unwrap(),
            ),
            (
                b"S0001 SETQUOTA \"#user/alice\" (STORAGE 510)\r\n",
                b"",
                CommandBody::set_quota(
                    AString::String(IString::try_from("#user/alice").unwrap()),
                    vec![QuotaSet::new(Resource::Storage, 510)],
                )
                    .unwrap()
                    .tag("S0001")
                    .unwrap(),
            ),
            (
                b"S0002 SETQUOTA \"!partition/sda4\" (STORAGE 99999999)\r\n",
                b"",
                CommandBody::set_quota(
                    AString::String(IString::try_from("!partition/sda4").unwrap()),
                    vec![QuotaSet::new(Resource::Storage, 99999999)],
                )
                    .unwrap()
                    .tag("S0002")
                    .unwrap(),
            ),
            (
                b"A001 SETQUOTA \"\" (STORAGE 512)\r\n",
                b"",
                CommandBody::set_quota("", vec![QuotaSet::new(Resource::Storage, 512)])
                    .unwrap()
                    .tag("A001")
                    .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_command_status_quota() {
        kat_inverse_command(&[(
            b"S0003 STATUS INBOX (MESSAGES DELETED DELETED-STORAGE)\r\n",
            b"",
            CommandBody::status(
                "inbox",
                vec![
                    StatusDataItemName::Messages,
                    StatusDataItemName::Deleted,
                    StatusDataItemName::DeletedStorage,
                ],
            )
            .unwrap()
            .tag("S0003")
            .unwrap(),
        )]);
    }

    #[test]
    fn test_kat_inverse_response_data_quota() {
        kat_inverse_response(&[
            (
                b"* QUOTA INBOX (MESSAGE 1024 2048)\r\n".as_ref(),
                b"".as_ref(),
                Response::Data(
                    Data::quota(
                        "INBOX",
                        vec![QuotaGet {
                            resource: Resource::Message,
                            usage: 1024,
                            limit: 2048,
                        }],
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* QUOTAROOT INBOX\r\n",
                b"",
                Response::Data(Data::quota_root("INBOX", vec![]).unwrap()),
            ),
            (
                b"* QUOTAROOT INBOX ROOT1 ROOT2\r\n",
                b"",
                Response::Data(
                    Data::quota_root(
                        "INBOX",
                        vec!["ROOT1".try_into().unwrap(), "ROOT2".try_into().unwrap()],
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* CAPABILITY QUOTA QUOTA=RES-STORAGE\r\n",
                b"",
                Response::Data(
                    Data::capability(vec![
                        Capability::Quota,
                        Capability::QuotaRes(Resource::Storage),
                    ])
                    .unwrap(),
                ),
            ),
            (
                b"* CAPABILITY QUOTA QUOTA=RES-STORAGE QUOTA=RES-MESSAGE\r\n",
                b"",
                Response::Data(
                    Data::capability(vec![
                        Capability::Quota,
                        Capability::QuotaRes(Resource::Storage),
                        Capability::QuotaRes(Resource::Message),
                    ])
                    .unwrap(),
                ),
            ),
            (
                b"* CAPABILITY QUOTA QUOTASET QUOTA=RES-STORAGE QUOTA=RES-MESSAGE\r\n",
                b"",
                Response::Data(
                    Data::capability(vec![
                        Capability::Quota,
                        Capability::QuotaSet,
                        Capability::QuotaRes(Resource::Storage),
                        Capability::QuotaRes(Resource::Message),
                    ])
                    .unwrap(),
                ),
            ),
            (
                b"* QUOTA \"!partition/sda4\" (STORAGE 104 10923847)\r\n",
                b"",
                Response::Data(
                    Data::quota(
                        AString::String(IString::try_from("!partition/sda4").unwrap()),
                        vec![QuotaGet::new(Resource::Storage, 104, 10923847)],
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* QUOTA \"\" (STORAGE 10 512)\r\n",
                b"",
                Response::Data(Data::Quota {
                    root: "".try_into().unwrap(),
                    quotas: vec![QuotaGet::new(Resource::Storage, 10, 512)]
                        .try_into()
                        .unwrap(),
                }),
            ),
            (
                b"* QUOTA \"#user/alice\" (MESSAGE 42 1000)\r\n",
                b"",
                Response::Data(Data::Quota {
                    root: AString::String(IString::try_from("#user/alice").unwrap()),
                    quotas: vec![QuotaGet::new(Resource::Message, 42, 1000)]
                        .try_into()
                        .unwrap(),
                }),
            ),
            (
                b"* QUOTA \"#user/alice\" (STORAGE 54 111 MESSAGE 42 1000)\r\n",
                b"",
                Response::Data(
                    Data::quota(
                        AString::String(IString::try_from("#user/alice").unwrap()),
                        vec![
                            QuotaGet::new(Resource::Storage, 54, 111),
                            QuotaGet::new(Resource::Message, 42, 1000),
                        ],
                    )
                    .unwrap(),
                ),
            ),
            #[cfg(feature = "quirk_excessive_space_quota_resource")]
            (
                b"* QUOTA \"#user/alice\" (STORAGE 54 111 MESSAGE 42  1000)\r\n",
                b"",
                Response::Data(
                    Data::quota(
                        AString::String(IString::try_from("#user/alice").unwrap()),
                        vec![
                            QuotaGet::new(Resource::Storage, 54, 111),
                            QuotaGet::new(Resource::Message, 42, 1000),
                        ],
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* QUOTA \"#user/alice\" (STORAGE 58 512)\r\n",
                b"",
                Response::Data(
                    Data::quota(
                        AString::String(IString::try_from("#user/alice").unwrap()),
                        vec![QuotaGet::new(Resource::Storage, 58, 512)],
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* QUOTAROOT INBOX \"\"\r\n",
                b"",
                Response::Data(Data::QuotaRoot {
                    mailbox: Mailbox::Inbox,
                    roots: vec!["".try_into().unwrap()],
                }),
            ),
            (
                b"* QUOTAROOT comp.mail.mime\r\n",
                b"",
                Response::Data(Data::QuotaRoot {
                    mailbox: Mailbox::try_from("comp.mail.mime").unwrap(),
                    roots: vec![],
                }),
            ),
            (
                b"* QUOTAROOT INBOX \"#user/alice\" \"!partition/sda4\"\r\n",
                b"",
                Response::Data(Data::QuotaRoot {
                    mailbox: Mailbox::try_from("inbox").unwrap(),
                    roots: vec![
                        AString::String(IString::try_from("#user/alice").unwrap()),
                        AString::String(IString::try_from("!partition/sda4").unwrap()),
                    ],
                }),
            ),
            (
                b"* STATUS INBOX (MESSAGES 12 DELETED 4 DELETED-STORAGE 8)\r\n",
                b"",
                Response::Data(Data::Status {
                    mailbox: Mailbox::Inbox,
                    items: vec![
                        StatusDataItem::Messages(12),
                        StatusDataItem::Deleted(4),
                        StatusDataItem::DeletedStorage(8),
                    ]
                    .into(),
                }),
            ),
            (
                b"* NO [OVERQUOTA] Soft quota has been exceeded\r\n",
                b"",
                Response::Status(
                    Status::no(None, Some(Code::OverQuota), "Soft quota has been exceeded")
                        .unwrap(),
                ),
            ),
            (
                b"A003 NO [OVERQUOTA] APPEND Failed\r\n",
                b"".as_ref(),
                Response::Status(
                    Status::no(
                        Some(Tag::try_from("A003").unwrap()),
                        Some(Code::OverQuota),
                        "APPEND Failed",
                    )
                    .unwrap(),
                ),
            ),
        ]);
    }
}
