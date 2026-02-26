pub mod decode;
pub mod encode;

/// Codec for greetings.
#[derive(Clone, Debug, Default, PartialEq)]
// We use `#[non_exhaustive]` to prevent users from using struct literal syntax.
//
// This allows to add configuration options later. For example, the
// codec could transparently replace all literals with non-sync literals.
#[non_exhaustive]
pub struct GreetingCodec;

/// Codec for commands.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CommandCodec;

/// Codec for authenticate data lines.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AuthenticateDataCodec;

/// Codec for responses.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ResponseCodec;

/// Codec for idle dones.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct IdleDoneCodec;

macro_rules! impl_codec_new {
    ($codec:ty) => {
        impl $codec {
            /// Create codec with default configuration.
            pub fn new() -> Self {
                Self::default()
            }
        }
    };
}

impl_codec_new!(GreetingCodec);
impl_codec_new!(CommandCodec);
impl_codec_new!(AuthenticateDataCodec);
impl_codec_new!(ResponseCodec);
impl_codec_new!(IdleDoneCodec);

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use imap_types::{
        auth::AuthenticateData,
        command::{Command, CommandBody},
        core::{IString, Literal, LiteralMode, NString, Tag, Vec1},
        extensions::idle::IdleDone,
        fetch::MessageDataItem,
        mailbox::Mailbox,
        response::{Data, Greeting, GreetingKind, Response},
    };

    use super::*;
    use crate::{
        decode::{CommandDecodeError, Decoder, GreetingDecodeError, ResponseDecodeError},
        testing::{
            kat_inverse_authenticate_data, kat_inverse_command, kat_inverse_done,
            kat_inverse_greeting, kat_inverse_response,
        },
    };

    #[test]
    fn test_kat_inverse_greeting() {
        kat_inverse_greeting(&[
            (
                b"* OK ...\r\n".as_ref(),
                b"".as_ref(),
                Greeting::new(GreetingKind::Ok, None, "...").unwrap(),
            ),
            (
                b"* ByE .\r\n???",
                b"???",
                Greeting::new(GreetingKind::Bye, None, ".").unwrap(),
            ),
            (
                b"* preaUth x\r\n?",
                b"?",
                Greeting::new(GreetingKind::PreAuth, None, "x").unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_command() {
        kat_inverse_command(&[
            (
                b"a nOOP\r\n".as_ref(),
                b"".as_ref(),
                Command::new("a", CommandBody::Noop).unwrap(),
            ),
            (
                b"a NooP\r\n???",
                b"???",
                Command::new("a", CommandBody::Noop).unwrap(),
            ),
            (
                b"a SeLECT {5}\r\ninbox\r\n",
                b"",
                Command::new(
                    "a",
                    CommandBody::Select {
                        mailbox: Mailbox::Inbox,
                        #[cfg(feature = "ext_condstore_qresync")]
                        parameters: Vec::default(),
                    },
                )
                .unwrap(),
            ),
            (
                b"a SElECT {5}\r\ninbox\r\nxxx",
                b"xxx",
                Command::new(
                    "a",
                    CommandBody::Select {
                        mailbox: Mailbox::Inbox,
                        #[cfg(feature = "ext_condstore_qresync")]
                        parameters: Vec::default(),
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
                b"* SEARCH 1\r\n".as_ref(),
                b"".as_ref(),
                Response::Data(Data::Search(
                    vec![NonZeroU32::new(1).unwrap()],
                    #[cfg(feature = "ext_condstore_qresync")]
                    None,
                )),
            ),
            (
                b"* SEARCH 1\r\n???",
                b"???",
                Response::Data(Data::Search(
                    vec![NonZeroU32::new(1).unwrap()],
                    #[cfg(feature = "ext_condstore_qresync")]
                    None,
                )),
            ),
            (
                b"* 1 FETCH (RFC822 {5}\r\nhello)\r\n",
                b"",
                Response::Data(Data::Fetch {
                    seq: NonZeroU32::new(1).unwrap(),
                    items: Vec1::from(MessageDataItem::Rfc822(NString(Some(IString::Literal(
                        Literal::try_from(b"hello".as_ref()).unwrap(),
                    ))))),
                }),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_authenticate_data() {
        kat_inverse_authenticate_data(&[
            (
                b"VGVzdA==\r\n".as_ref(),
                b"".as_ref(),
                AuthenticateData::r#continue(b"Test".to_vec()),
            ),
            (
                b"AA==\r\n".as_ref(),
                b"".as_ref(),
                AuthenticateData::r#continue(b"\x00".to_vec()),
            ),
            (
                b"aQ==\r\n".as_ref(),
                b"".as_ref(),
                AuthenticateData::r#continue(b"\x69".to_vec()),
            ),
            (b"*\r\n".as_ref(), b"".as_ref(), AuthenticateData::Cancel),
        ]);
    }

    #[test]
    fn test_kat_inverse_done() {
        kat_inverse_done(&[
            (b"done\r\n".as_ref(), b"".as_ref(), IdleDone),
            (b"DONE\r\n".as_ref(), b"".as_ref(), IdleDone),
        ]);
    }

    #[test]
    fn test_greeting_incomplete_failed() {
        let tests = [
            // Incomplete
            (b"*".as_ref(), Err(GreetingDecodeError::Incomplete)),
            (b"* ".as_ref(), Err(GreetingDecodeError::Incomplete)),
            (b"* O".as_ref(), Err(GreetingDecodeError::Incomplete)),
            (b"* OK".as_ref(), Err(GreetingDecodeError::Incomplete)),
            (b"* OK ".as_ref(), Err(GreetingDecodeError::Incomplete)),
            (b"* OK .".as_ref(), Err(GreetingDecodeError::Incomplete)),
            (b"* OK .\r".as_ref(), Err(GreetingDecodeError::Incomplete)),
            // Failed
            (b"**".as_ref(), Err(GreetingDecodeError::Failed)),
            (b"* NO x\r\n".as_ref(), Err(GreetingDecodeError::Failed)),
        ];

        for (test, expected) in tests {
            let got = GreetingCodec::default().decode(test);
            dbg!((std::str::from_utf8(test).unwrap(), &expected, &got));
            assert_eq!(expected, got);

            {
                let got = GreetingCodec::default().decode_static(test);
                assert_eq!(expected, got);
            }
        }
    }

    #[test]
    fn test_command_incomplete_failed() {
        let tests = [
            // Incomplete
            (b"a".as_ref(), Err(CommandDecodeError::Incomplete)),
            (b"a ".as_ref(), Err(CommandDecodeError::Incomplete)),
            (b"a n".as_ref(), Err(CommandDecodeError::Incomplete)),
            (b"a no".as_ref(), Err(CommandDecodeError::Incomplete)),
            (b"a noo".as_ref(), Err(CommandDecodeError::Incomplete)),
            (b"a noop".as_ref(), Err(CommandDecodeError::Incomplete)),
            (b"a noop\r".as_ref(), Err(CommandDecodeError::Incomplete)),
            // LiteralAckRequired
            (
                b"a select {5}\r\n".as_ref(),
                Err(CommandDecodeError::LiteralFound {
                    tag: Tag::try_from("a").unwrap(),
                    length: 5,
                    mode: LiteralMode::Sync,
                }),
            ),
            (
                b"a select {5+}\r\n".as_ref(),
                Err(CommandDecodeError::LiteralFound {
                    tag: Tag::try_from("a").unwrap(),
                    length: 5,
                    mode: LiteralMode::NonSync,
                }),
            ),
            // Incomplete (after literal)
            (
                b"a select {5}\r\nxxx".as_ref(),
                Err(CommandDecodeError::Incomplete),
            ),
            // Failed
            (b"* noop\r\n".as_ref(), Err(CommandDecodeError::Failed)),
            (b"A  noop\r\n".as_ref(), Err(CommandDecodeError::Failed)),
        ];

        for (test, expected) in tests {
            let got = CommandCodec::default().decode(test);
            dbg!((std::str::from_utf8(test).unwrap(), &expected, &got));
            assert_eq!(expected, got);

            {
                let got = CommandCodec::default().decode_static(test);
                assert_eq!(expected, got);
            }
        }
    }

    #[test]
    fn test_response_incomplete_failed() {
        let tests = [
            // Incomplete
            (b"".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"*".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* ".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* S".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* SE".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* SEA".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* SEAR".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* SEARC".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* SEARCH".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* SEARCH ".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (b"* SEARCH 1".as_ref(), Err(ResponseDecodeError::Incomplete)),
            (
                b"* SEARCH 1\r".as_ref(),
                Err(ResponseDecodeError::Incomplete),
            ),
            // LiteralAck treated as Incomplete
            (
                b"* 1 FETCH (RFC822 {5}\r\n".as_ref(),
                Err(ResponseDecodeError::LiteralFound { length: 5 }),
            ),
            // Failed
            (
                b"*  search 1 2 3\r\n".as_ref(),
                Err(ResponseDecodeError::Failed),
            ),
            (b"A search\r\n".as_ref(), Err(ResponseDecodeError::Failed)),
        ];

        for (test, expected) in tests {
            let got = ResponseCodec::default().decode(test);
            dbg!((std::str::from_utf8(test).unwrap(), &expected, &got));
            assert_eq!(expected, got);

            {
                let got = ResponseCodec::default().decode_static(test);
                assert_eq!(expected, got);
            }
        }
    }
}
