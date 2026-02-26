//! The IMAP ENABLE Extension

// Additional changes:
//
// capability    =/ "ENABLE"
// command-any   =/ "ENABLE" 1*(SP capability)
// response-data =/ "*" SP enable-data CRLF

use std::io::Write;

use abnf_core::streaming::sp;
use imap_types::{command::CommandBody, extensions::enable::CapabilityEnable, response::Data};
use nom::{
    bytes::streaming::tag_no_case,
    combinator::map,
    multi::{many0, many1},
    sequence::{preceded, tuple},
};

use crate::{
    core::atom,
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext},
};

/// `command-any =/ "ENABLE" 1*(SP capability)`
///
/// Note:
///
/// Introduced into imap-codec as ...
///
/// ```text
/// enable = "ENABLE" 1*(SP capability)
///
/// command-any =/ enable
/// ```
pub(crate) fn enable(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        tag_no_case("ENABLE"),
        many1(preceded(sp, capability_enable)),
    ));

    let (remaining, (_, capabilities)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Enable {
            capabilities: capabilities.try_into().unwrap(),
        },
    ))
}

pub(crate) fn capability_enable(input: &[u8]) -> IMAPResult<&[u8], CapabilityEnable> {
    map(atom, CapabilityEnable::from)(input)
}

/// `enable-data = "ENABLED" *(SP capability)`
pub(crate) fn enable_data(input: &[u8]) -> IMAPResult<&[u8], Data> {
    let mut parser = preceded(
        tag_no_case(b"ENABLED"),
        many0(preceded(sp, capability_enable)),
    );

    let (remaining, capabilities) = parser(input)?;

    Ok((remaining, { Data::Enabled { capabilities } }))
}

impl EncodeIntoContext for CapabilityEnable<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ext_utf8")]
    use imap_types::extensions::utf8::Utf8Kind;
    use imap_types::{command::Command, core::Atom, extensions::enable::CapabilityEnable};

    use super::*;
    use crate::testing::kat_inverse_command;

    #[cfg(feature = "ext_utf8")]
    #[test]
    fn test_parse_enable() {
        let got = enable(b"enable UTF8=ACCEPT\r\n").unwrap().1;
        assert_eq!(
            CommandBody::enable(vec![CapabilityEnable::Utf8(Utf8Kind::Accept)]).unwrap(),
            got
        );
    }

    #[test]
    fn test_kat_inverse_command_enable() {
        kat_inverse_command(&[
            #[cfg(feature = "ext_utf8")]
            (
                b"A ENABLE UTF8=ONLY\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::enable(vec![CapabilityEnable::Utf8(Utf8Kind::Only)]).unwrap(),
                )
                .unwrap(),
            ),
            #[cfg(feature = "ext_utf8")]
            (
                b"A ENABLE UTF8=ACCEPT\r\n?",
                b"?".as_ref(),
                Command::new(
                    "A",
                    CommandBody::enable(vec![CapabilityEnable::Utf8(Utf8Kind::Accept)]).unwrap(),
                )
                .unwrap(),
            ),
            (
                b"A ENABLE FOO\r\n??",
                b"??",
                Command::new(
                    "A",
                    CommandBody::enable(vec![CapabilityEnable::from(
                        Atom::try_from("FOO").unwrap(),
                    )])
                    .unwrap(),
                )
                .unwrap(),
            ),
        ]);
    }
}
