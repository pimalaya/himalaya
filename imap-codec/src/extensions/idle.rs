//! IMAP4 IDLE command
//!
//! This extension enables the [`CommandBody::Idle`](imap_types::command::CommandBody#variant.Idle) variant.
//! No additional types are used.

// Additional changes:
//
// command_auth =/ idle

use std::io::Write;

#[cfg(not(feature = "quirk_crlf_relaxed"))]
use abnf_core::streaming::crlf;
#[cfg(feature = "quirk_crlf_relaxed")]
use abnf_core::streaming::crlf_relaxed as crlf;
use imap_types::{command::CommandBody, extensions::idle::IdleDone};
use nom::{bytes::streaming::tag_no_case, combinator::value, sequence::tuple};

use crate::{
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext},
};

/// `idle = "IDLE" CRLF "DONE"` (edited)
///
/// ```text
/// idle = "IDLE" CRLF "DONE"
///        ^^^^^^^^^^^
///        |
///        This is parsed here.
///        CRLF is consumed in upper command parser.
/// ```
///
/// Valid only in Authenticated or Selected state
pub(crate) fn idle(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    value(CommandBody::Idle, tag_no_case("IDLE"))(input)
}

/// `idle = "IDLE" CRLF "DONE"` (edited)
///
/// ```text
/// idle = "IDLE" CRLF "DONE" CRLF
///                    ^^^^^^^^^^^
///                    |
///                    This is parsed here.
///                    CRLF is additionally consumed in this parser.
/// ```
///
/// Valid only in Authenticated or Selected state
///
/// Note: This parser must be executed *instead* of the command parser
/// when the server is in the IDLE state.
pub(crate) fn idle_done(input: &[u8]) -> IMAPResult<&[u8], IdleDone> {
    value(IdleDone, tuple((tag_no_case("DONE"), crlf)))(input)
}

impl EncodeIntoContext for IdleDone {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(b"DONE\r\n")
    }
}

#[cfg(test)]
mod tests {
    use imap_types::command::{Command, CommandBody};

    use super::*;
    use crate::{
        IdleDoneCodec,
        decode::{Decoder, IdleDoneDecodeError},
        testing::kat_inverse_command,
    };

    #[test]
    fn test_kat_inverse_command_idle() {
        kat_inverse_command(&[
            (
                b"A IDLE\r\n".as_ref(),
                b"".as_ref(),
                Command::new("A", CommandBody::Idle).unwrap(),
            ),
            (
                b"A IDLE\r\n?",
                b"?",
                Command::new("A", CommandBody::Idle).unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_decode_idle_done() {
        let tests = [
            // Ok
            (b"done\r\n".as_ref(), Ok((b"".as_ref(), IdleDone))),
            (b"done\r\n?".as_ref(), Ok((b"?".as_ref(), IdleDone))),
            // Incomplete
            (b"d".as_ref(), Err(IdleDoneDecodeError::Incomplete)),
            (b"do".as_ref(), Err(IdleDoneDecodeError::Incomplete)),
            (b"don".as_ref(), Err(IdleDoneDecodeError::Incomplete)),
            (b"done".as_ref(), Err(IdleDoneDecodeError::Incomplete)),
            (b"done\r".as_ref(), Err(IdleDoneDecodeError::Incomplete)),
            // Failed
            (b"donee\r\n".as_ref(), Err(IdleDoneDecodeError::Failed)),
            (b" done\r\n".as_ref(), Err(IdleDoneDecodeError::Failed)),
            (b"done \r\n".as_ref(), Err(IdleDoneDecodeError::Failed)),
            (b" done \r\n".as_ref(), Err(IdleDoneDecodeError::Failed)),
        ];

        for (test, expected) in tests {
            let got = IdleDoneCodec::default().decode(test);

            dbg!((std::str::from_utf8(test).unwrap(), &expected, &got));

            assert_eq!(expected, got);
        }
    }
}
