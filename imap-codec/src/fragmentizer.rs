//! Utilities to split IMAP bytes into line and literal fragments.
//!
//! These utilities can be used to fragment a stream of IMAP bytes into lines (with metadata) and
//! literals (before actually doing detailed IMAP parsing).
//!
//! This approach has multiple advantages: It separates literal handling from IMAP parsing and sets
//! clear message boundaries even in the presence of malformed messages. Consequently, malformed
//! messages can be reliably discarded. (A naive implementation of byte discardment may lead to
//! adventurous (security) issues, such as, literal data being interpreted as command or response.)
//! Further, this two-layered approach allows to more easily guard against excessive memory
//! allocation by malevolant actors.
//!
//! # Example
//!
//! ```rust,ignore
//! # use std::io::{stdin, Read};
//! #
//! # use imap_codec::{
//! #     fragmentizer::Fragmentizer,
//! #     imap_types::utils::escape_byte_string,
//! # };
//! #
//! # fn read_bytes() -> &'static [u8] { b"" }
//! #
//! # fn main() {
//! let mut fragmentizer = Fragmentizer::new(1024);
//!
//! loop {
//!     match fragmentizer.progress() {
//!         Some(fragment_info) => {
//!             let fragment_bytes = fragmentizer.fragment_bytes(fragment_info);
//!
//!             if fragmentizer.is_message_complete() {
//!                 let message_bytes = fragmentizer.message_bytes();
//!             }
//!         }
//!         None => {
//!             let received = read_bytes();
//!             fragmentizer.enqueue_bytes(received);
//!         }
//!     }
//! }
//! # }
//! ```
use std::{collections::VecDeque, ops::Range};

use imap_types::{
    core::{LiteralMode, Tag},
    secret::Secret,
};

use crate::decode::Decoder;

/// Splits IMAP bytes into line and literal fragments.
///
/// The `Fragmentizer` prevents excessive memory allocation through a configurable maximum message size.
/// Correct fragmentation is ensured even for messages exceeding the allowed message size.
///
/// If the message size is exceeded,
/// [`Fragmentizer::decode_message`] will fail and
/// [`Fragmentizer::message_bytes`] will emit truncated message bytes.
/// However, fragmentation will seamlessly continue with the following message.
#[derive(Clone, Debug)]
pub struct Fragmentizer {
    /// Enqueued bytes that are not parsed by [`Fragmentizer::progress`] yet.
    unparsed_buffer: VecDeque<u8>,
    /// Upper limit for the size of parsed messages.
    max_message_size: Option<u32>,
    /// Whether the size limit is exceeded for the current message.
    max_message_size_exceeded: bool,
    /// The current message was poisoned. The message will still be parsed, but the decoding
    /// will fail.
    message_poisoned: bool,
    /// Parsed bytes of the current messages. The length is limited by
    /// [`Fragmentizer::max_message_size`].
    message_buffer: Vec<u8>,
    /// Parser for the next fragment of the current message. Is `None` if no fragment is expected
    /// because the message is complete.
    parser: Option<Parser>,
}

impl Fragmentizer {
    /// Creates a `Fragmentizer` with a maximum message size.
    ///
    /// The maximum message size is bounded by `max_message_size` to prevent excessive memory allocation.
    pub fn new(max_message_size: u32) -> Self {
        Self {
            unparsed_buffer: VecDeque::new(),
            max_message_size: Some(max_message_size),
            max_message_size_exceeded: false,
            message_poisoned: false,
            message_buffer: Vec::new(),
            parser: Some(Parser::Line(LineParser::new(0))),
        }
    }

    /// Creates a `Fragmentizer` without a maximum message size.
    ///
    /// <div class="warning">
    /// This is dangerous because it allows an attacker to allocate an excessive amount of memory
    /// by sending a huge message.
    /// </div>
    pub fn without_max_message_size() -> Self {
        Self {
            unparsed_buffer: VecDeque::new(),
            max_message_size: None,
            max_message_size_exceeded: false,
            message_poisoned: false,
            message_buffer: Vec::new(),
            parser: Some(Parser::Line(LineParser::new(0))),
        }
    }

    /// Continue parsing the current message until the next fragment is detected.
    ///
    /// Returns `None` if more bytes need to be enqueued via [`Fragmentizer::enqueue_bytes`].
    /// If [`Fragmentizer::is_message_complete`] returns true after this function was called,
    /// then the message was fully parsed. The following call of this function will then start
    /// the next message.
    pub fn progress(&mut self) -> Option<FragmentInfo> {
        let parser = match &mut self.parser {
            Some(parser) => {
                // Continue current message
                parser
            }
            None => {
                // Start next message
                self.max_message_size_exceeded = false;
                self.message_poisoned = false;
                self.message_buffer.clear();
                self.parser.insert(Parser::Line(LineParser::new(0)))
            }
        };

        // Progress fragment
        let (parsed_byte_count, fragment) = match parser {
            Parser::Line(parser) => parser.parse(&self.unparsed_buffer),
            Parser::Literal(parser) => parser.parse(&self.unparsed_buffer),
        };
        self.dequeue_parsed_bytes(parsed_byte_count);

        if let Some(fragment) = fragment {
            self.parser = match fragment {
                // Finish current message
                FragmentInfo::Line {
                    announcement: None, ..
                } => None,
                // Next fragment will be a literal
                FragmentInfo::Line {
                    end,
                    announcement: Some(LiteralAnnouncement { length, .. }),
                    ..
                } => Some(Parser::Literal(LiteralParser::new(end, length))),
                // Next fragment will be a line
                FragmentInfo::Literal { end, .. } => Some(Parser::Line(LineParser::new(end))),
            }
        }

        fragment
    }

    /// Enqueues more byte that can be parsed by [`Fragmentizer::progress`].
    ///
    /// Note that the message size limit is not enforced on the enqueued bytes. You can control
    /// the size of the enqueued bytes by only calling this function if more bytes are necessary.
    /// More bytes are necessary if [`Fragmentizer::progress`] returns `None`.
    pub fn enqueue_bytes(&mut self, bytes: &[u8]) {
        self.unparsed_buffer.extend(bytes);
    }

    /// Returns the bytes for a fragment of the current message.
    pub fn fragment_bytes(&self, fragment_info: FragmentInfo) -> &[u8] {
        let (start, end) = match fragment_info {
            FragmentInfo::Line { start, end, .. } => (start, end),
            FragmentInfo::Literal { start, end } => (start, end),
        };
        let start = start.min(self.message_buffer.len());
        let end = end.min(self.message_buffer.len());
        &self.message_buffer[start..end]
    }

    /// Returns whether the current message was fully parsed.
    ///
    /// If it returns true then it makes sense to call [`Fragmentizer::decode_message`]
    /// to decode the message. Alternatively, you can access all bytes of the message via
    /// [`Fragmentizer::message_bytes`].
    pub fn is_message_complete(&self) -> bool {
        self.parser.is_none()
    }

    /// Returns the bytes of the current message.
    ///
    /// Note that the bytes might be incomplete:
    /// - The message might not be fully parsed yet and [`Fragmentizer::progress`] need to be
    ///   called. You can check whether the message is complete via
    ///   [`Fragmentizer::is_message_complete`].
    /// - The size limit might be exceeded and bytes might be dropped. You can check this
    ///   via [`Fragmentizer::is_max_message_size_exceeded`]
    pub fn message_bytes(&self) -> &[u8] {
        &self.message_buffer
    }

    /// Returns whether the size limit is exceeded for the current message.
    pub fn is_max_message_size_exceeded(&self) -> bool {
        self.max_message_size_exceeded
    }

    /// Returns whether the current message was explicitly poisoned to prevent decoding.
    pub fn is_message_poisoned(&self) -> bool {
        self.message_poisoned
    }

    /// Skips the current message and starts the next message immediately.
    ///
    /// Warning: Using this method might be dangerous. If client and server don't
    /// agree at which point a message is skipped, then the client or server might
    /// treat untrusted bytes (e.g. literal bytes) as IMAP messages. Currently the
    /// only valid use-case is a server that rejects synchronizing literals from the
    /// client. Otherwise consider using [`Fragmentizer::poison_message`].
    pub fn skip_message(&mut self) {
        self.max_message_size_exceeded = false;
        self.message_poisoned = false;
        self.message_buffer.clear();
        self.parser = Some(Parser::Line(LineParser::new(0)));
    }

    /// Poisons the current message to prevent its decoding.
    ///
    /// When this function is called the fragments of the current message are parsed normally, but
    /// [`Fragmentizer::decode_message`] is guaranteed to fail and return
    /// [`DecodeMessageError::MessagePoisoned`]. This allows to skip malformed messages (e.g.
    /// a message with an unexpected line ending) safely without the risk of treating untrusted
    /// bytes (e.g. literal bytes) as IMAP messages
    pub fn poison_message(&mut self) {
        self.message_poisoned = true;
    }

    /// Tries to decode the [`Tag`] for the current message.
    ///
    /// Note that decoding the [`Tag`] is on best effort basis. Some message types don't have
    /// a [`Tag`] and without context you can't know whether this function will succeed. However,
    /// this function is useful if the message is incomplete or malformed and you want to decode
    /// the [`Tag`] in order to send a response.
    pub fn decode_tag(&self) -> Option<Tag> {
        parse_tag(&self.message_buffer)
    }

    /// Tries to decode the current message with the given decoder.
    ///
    /// You usually want to call this method once [`Fragmentizer::is_message_complete`] returns
    /// true. Which decoder should be used depends on the state of the IMAP conversation. The
    /// caller is responsible for tracking this state and choosing the decoder.
    pub fn decode_message<'a, C: Decoder>(
        &'a self,
        codec: &C,
    ) -> Result<C::Message<'a>, DecodeMessageError<'a, C>> {
        if self.max_message_size_exceeded {
            return Err(DecodeMessageError::MessageTooLong {
                initial: Secret::new(&self.message_buffer),
            });
        }

        if self.message_poisoned {
            return Err(DecodeMessageError::MessagePoisoned {
                discarded: Secret::new(&self.message_buffer),
            });
        }

        let (remainder, message) = match codec.decode(&self.message_buffer) {
            Ok(res) => res,
            Err(err) => return Err(DecodeMessageError::DecodingFailure(err)),
        };

        if !remainder.is_empty() {
            return Err(DecodeMessageError::DecodingRemainder {
                message,
                remainder: Secret::new(remainder),
            });
        }

        Ok(message)
    }

    fn dequeue_parsed_bytes(&mut self, parsed_byte_count: usize) {
        // This will remove the parsed bytes even if we don't add them to the message buffer
        let parsed_bytes = self.unparsed_buffer.drain(..parsed_byte_count);
        // How many bytes can we add to the message buffer?
        let remaining_size = self
            .max_message_size
            .map(|size| size as usize - self.message_buffer.len());

        // Add bytes to the message buffer
        match remaining_size {
            Some(remaining_size) if remaining_size < parsed_byte_count => {
                let remaining_bytes = parsed_bytes.take(remaining_size);
                self.message_buffer.extend(remaining_bytes);
                self.max_message_size_exceeded = true;
            }
            _ => {
                self.message_buffer.extend(parsed_bytes);
            }
        }
    }
}

/// Stateful parser for the next fragment.
#[derive(Clone, Debug)]
enum Parser {
    Line(LineParser),
    Literal(LiteralParser),
}

/// Stateful parser for the next line fragment.
#[derive(Clone, Debug)]
struct LineParser {
    /// Where we started parsing the line.
    start: usize,
    /// Until where we parsed the line.
    end: usize,
    /// Accumulated state based on the parsed bytes.
    latest_byte: LatestByte,
}

impl LineParser {
    fn new(start: usize) -> Self {
        Self {
            start,
            end: start,
            latest_byte: LatestByte::Other,
        }
    }

    fn parse(&mut self, unprocessed_bytes: &VecDeque<u8>) -> (usize, Option<FragmentInfo>) {
        let mut parsed_byte_count = 0;
        let mut parsed_line = None;

        // Parse next byte
        for &next_byte in unprocessed_bytes {
            parsed_byte_count += 1;
            self.end += 1;

            self.latest_byte = match self.latest_byte {
                LatestByte::Other => match next_byte {
                    b'\r' => LatestByte::Cr { announcement: None },
                    b'\n' => {
                        parsed_line = Some(FragmentInfo::Line {
                            start: self.start,
                            end: self.end,
                            announcement: None,
                            ending: LineEnding::Lf,
                        });
                        LatestByte::Other
                    }
                    b'{' => LatestByte::OpeningBracket,
                    _ => LatestByte::Other,
                },
                LatestByte::OpeningBracket => match next_byte {
                    b'\r' => LatestByte::Cr { announcement: None },
                    b'\n' => {
                        parsed_line = Some(FragmentInfo::Line {
                            start: self.start,
                            end: self.end,
                            announcement: None,
                            ending: LineEnding::Lf,
                        });
                        LatestByte::Other
                    }
                    b'{' => LatestByte::OpeningBracket,
                    b'0'..=b'9' => {
                        let digit = (next_byte - b'0') as u32;
                        LatestByte::Digit { length: digit }
                    }
                    _ => LatestByte::Other,
                },
                LatestByte::Plus { length } => match next_byte {
                    b'\r' => LatestByte::Cr { announcement: None },
                    b'\n' => {
                        parsed_line = Some(FragmentInfo::Line {
                            start: self.start,
                            end: self.end,
                            announcement: None,
                            ending: LineEnding::Lf,
                        });
                        LatestByte::Other
                    }
                    b'{' => LatestByte::OpeningBracket,
                    b'}' => LatestByte::ClosingBracket {
                        announcement: LiteralAnnouncement {
                            mode: LiteralMode::NonSync,
                            length,
                        },
                    },
                    _ => LatestByte::Other,
                },
                LatestByte::Digit { length } => match next_byte {
                    b'\r' => LatestByte::Cr { announcement: None },
                    b'\n' => {
                        parsed_line = Some(FragmentInfo::Line {
                            start: self.start,
                            end: self.end,
                            announcement: None,
                            ending: LineEnding::Lf,
                        });
                        LatestByte::Other
                    }
                    b'{' => LatestByte::OpeningBracket,
                    b'0'..=b'9' => {
                        let digit = (next_byte - b'0') as u32;
                        let new_length = length.checked_mul(10).and_then(|x| x.checked_add(digit));
                        match new_length {
                            None => LatestByte::Other,
                            Some(length) => LatestByte::Digit { length },
                        }
                    }
                    b'+' => LatestByte::Plus { length },
                    b'}' => LatestByte::ClosingBracket {
                        announcement: LiteralAnnouncement {
                            mode: LiteralMode::Sync,
                            length,
                        },
                    },
                    _ => LatestByte::Other,
                },
                LatestByte::ClosingBracket { announcement } => match next_byte {
                    b'\r' => LatestByte::Cr {
                        announcement: Some(announcement),
                    },
                    b'\n' => {
                        parsed_line = Some(FragmentInfo::Line {
                            start: self.start,
                            end: self.end,
                            announcement: Some(announcement),
                            ending: LineEnding::Lf,
                        });
                        LatestByte::Other
                    }
                    b'{' => LatestByte::OpeningBracket,
                    _ => LatestByte::Other,
                },
                LatestByte::Cr { announcement } => match next_byte {
                    b'\r' => LatestByte::Cr { announcement: None },
                    b'\n' => {
                        parsed_line = Some(FragmentInfo::Line {
                            start: self.start,
                            end: self.end,
                            announcement,
                            ending: LineEnding::CrLf,
                        });
                        LatestByte::Other
                    }
                    b'{' => LatestByte::OpeningBracket,
                    _ => LatestByte::Other,
                },
            };

            if parsed_line.is_some() {
                // We parsed a complete line
                break;
            }
        }

        (parsed_byte_count, parsed_line)
    }
}

/// The latest byte seen by the [`LineParser`] with additional accumulated state.
#[derive(Clone, Debug)]
enum LatestByte {
    Other,
    OpeningBracket,
    Digit {
        length: u32,
    },
    Plus {
        length: u32,
    },
    ClosingBracket {
        announcement: LiteralAnnouncement,
    },
    Cr {
        announcement: Option<LiteralAnnouncement>,
    },
}

/// Stateful parser for the next literal fragment.
#[derive(Clone, Debug)]
struct LiteralParser {
    /// Where we started parsing the literal.
    start: usize,
    /// Until where we parsed the literal.
    end: usize,
    /// Remaining bytes we need to parse.
    remaining: u32,
}

impl LiteralParser {
    fn new(start: usize, length: u32) -> Self {
        Self {
            start,
            end: start,
            remaining: length,
        }
    }

    fn parse(&mut self, unprocessed_bytes: &VecDeque<u8>) -> (usize, Option<FragmentInfo>) {
        if unprocessed_bytes.len() < self.remaining as usize {
            // Not enough bytes yet
            let parsed_byte_count = unprocessed_bytes.len();
            self.end += parsed_byte_count;
            self.remaining -= parsed_byte_count as u32;
            (parsed_byte_count, None)
        } else {
            // There are enough bytes
            let parsed_byte_count = self.remaining as usize;
            self.end += parsed_byte_count;
            self.remaining = 0;
            let parsed_literal = FragmentInfo::Literal {
                start: self.start,
                end: self.end,
            };
            (parsed_byte_count, Some(parsed_literal))
        }
    }
}

/// Describes a fragment of the current message found by [`Fragmentizer::progress`].
///
/// The corresponding bytes can be retrieved via [`Fragmentizer::fragment_bytes`]
/// until [`Fragmentizer::is_message_complete`] returns true. After that the
/// next call of [`Fragmentizer::progress`] will start the next message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FragmentInfo {
    /// The fragment is a line.
    Line {
        /// Inclusive start index relative to the current message.
        start: usize,
        /// Exclusive end index relative to the current message.
        end: usize,
        /// Whether the next fragment will be a literal.
        announcement: Option<LiteralAnnouncement>,
        /// The detected ending sequence for this line.
        ending: LineEnding,
    },
    /// The fragment is a literal.
    Literal {
        /// Inclusive start index relative to the current message.
        start: usize,
        /// Exclusive end index relative to the current message.
        end: usize,
    },
}

impl FragmentInfo {
    /// The index range relative to the current message.
    pub fn range(self) -> Range<usize> {
        match self {
            FragmentInfo::Line { start, end, .. } => start..end,
            FragmentInfo::Literal { start, end } => start..end,
        }
    }
}

/// Used by a line to announce a literal following the line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiteralAnnouncement {
    /// The mode of the announced literal.
    pub mode: LiteralMode,
    /// The length of the announced literal in bytes.
    pub length: u32,
}

/// The character sequence used for ending a line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineEnding {
    /// The line ends with the character `\n`.
    Lf,
    /// The line ends with the character sequence `\r\n`.
    CrLf,
}

/// An error returned by [`Fragmentizer::decode_message`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeMessageError<'a, C: Decoder> {
    /// The decoder failed decoding the message.
    DecodingFailure(C::Error<'a>),
    /// Not all bytes of the message were used when decoding the message.
    DecodingRemainder {
        /// The decoded message.
        message: C::Message<'a>,
        /// The unused bytes.
        remainder: Secret<&'a [u8]>,
    },
    /// Max message size was exceeded and bytes were dropped.
    MessageTooLong { initial: Secret<&'a [u8]> },
    /// The message was explicitly poisoned to prevent decoding.
    MessagePoisoned { discarded: Secret<&'a [u8]> },
}

fn parse_tag(message_bytes: &[u8]) -> Option<Tag> {
    let mut bytes = message_bytes.iter().enumerate();
    let sp = loop {
        let (i, byte) = bytes.next()?;
        match byte {
            // A tag is always delimited by SP
            b' ' => break i,
            // End of line reached
            b'\n' => return None,
            // Parse more bytes
            _ => continue,
        }
    };

    Tag::try_from(&message_bytes[..sp]).ok()
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::collections::VecDeque;

    use imap_types::{
        command::{Command, CommandBody},
        core::{LiteralMode, Tag},
        secret::Secret,
    };

    use super::{
        FragmentInfo, Fragmentizer, LineEnding, LineParser, LiteralAnnouncement, parse_tag,
    };
    use crate::{
        CommandCodec, ResponseCodec, decode::ResponseDecodeError, fragmentizer::DecodeMessageError,
    };

    #[test]
    fn fragmentizer_progress_nothing() {
        let mut fragmentizer = Fragmentizer::without_max_message_size();

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());

        fragmentizer.enqueue_bytes(&[]);
        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
    }

    #[test]
    fn fragmentizer_progress_single_message() {
        let mut fragmentizer = Fragmentizer::without_max_message_size();
        fragmentizer.enqueue_bytes(b"* OK ...\r\n");

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 10,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"* OK ...\r\n");
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
    }

    #[test]
    fn fragmentizer_progress_multiple_messages() {
        let mut fragmentizer = Fragmentizer::without_max_message_size();
        fragmentizer.enqueue_bytes(b"A1 OK ...\r\n");
        fragmentizer.enqueue_bytes(b"A2 BAD ...\r\n");

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 11,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"A1 OK ...\r\n");
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 12,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A2 BAD ...\r\n"
        );
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
    }

    #[test]
    fn fragmentizer_progress_multiple_messages_with_lf() {
        let mut fragmentizer = Fragmentizer::without_max_message_size();
        fragmentizer.enqueue_bytes(b"A1 NOOP\n");
        fragmentizer.enqueue_bytes(b"A2 LOGIN {5}\n");
        fragmentizer.enqueue_bytes(b"ABCDE");
        fragmentizer.enqueue_bytes(b" EFGIJ\n");

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 8,
                announcement: None,
                ending: LineEnding::Lf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"A1 NOOP\n");
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 13,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5
                }),
                ending: LineEnding::Lf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A2 LOGIN {5}\n"
        );
        assert!(!fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(fragment_info, FragmentInfo::Literal { start: 13, end: 18 });
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"ABCDE");
        assert!(!fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 18,
                end: 25,
                announcement: None,
                ending: LineEnding::Lf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b" EFGIJ\n");
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
    }

    #[test]
    fn fragmentizer_progress_message_with_multiple_literals() {
        let mut fragmentizer = Fragmentizer::without_max_message_size();
        fragmentizer.enqueue_bytes(b"A1 LOGIN {5}\r\n");
        fragmentizer.enqueue_bytes(b"ABCDE");
        fragmentizer.enqueue_bytes(b" {5}\r\n");
        fragmentizer.enqueue_bytes(b"FGHIJ");
        fragmentizer.enqueue_bytes(b"\r\n");

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 14,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5,
                }),
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A1 LOGIN {5}\r\n"
        );
        assert!(!fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(fragment_info, FragmentInfo::Literal { start: 14, end: 19 });
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"ABCDE");
        assert!(!fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 19,
                end: 25,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5,
                }),
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b" {5}\r\n");
        assert!(!fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(fragment_info, FragmentInfo::Literal { start: 25, end: 30 });
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"FGHIJ");
        assert!(!fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 30,
                end: 32,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"\r\n");
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
    }

    #[test]
    fn fragmentizer_progress_message_and_skip_after_literal_announcement() {
        let mut fragmentizer = Fragmentizer::without_max_message_size();
        fragmentizer.enqueue_bytes(b"A1 LOGIN {5}\r\n");
        fragmentizer.enqueue_bytes(b"A2 NOOP\r\n");

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 14,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5,
                }),
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A1 LOGIN {5}\r\n"
        );
        assert!(!fragmentizer.is_message_complete());

        fragmentizer.skip_message();

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 9,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"A2 NOOP\r\n");
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
    }

    #[test]
    fn fragmentizer_progress_message_byte_by_byte() {
        let mut fragmentizer = Fragmentizer::without_max_message_size();
        let mut bytes = VecDeque::new();
        bytes.extend(b"A1 LOGIN {5}\r\n");
        bytes.extend(b"ABCDE");
        bytes.extend(b" FGHIJ\r\n");

        for _ in 0..14 {
            let fragment_info = fragmentizer.progress();

            assert_eq!(fragment_info, None);
            assert!(!fragmentizer.is_message_complete());

            fragmentizer.enqueue_bytes(&[bytes.pop_front().unwrap()]);
        }

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 14,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5,
                }),
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A1 LOGIN {5}\r\n"
        );
        assert!(!fragmentizer.is_message_complete());

        for _ in 0..5 {
            let fragment_info = fragmentizer.progress();

            assert_eq!(fragment_info, None);
            assert!(!fragmentizer.is_message_complete());

            fragmentizer.enqueue_bytes(&[bytes.pop_front().unwrap()]);
        }

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(fragment_info, FragmentInfo::Literal { start: 14, end: 19 });
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"ABCDE");
        assert!(!fragmentizer.is_message_complete());

        for _ in 0..8 {
            let fragment_info = fragmentizer.progress();

            assert_eq!(fragment_info, None);
            assert!(!fragmentizer.is_message_complete());

            fragmentizer.enqueue_bytes(&[bytes.pop_front().unwrap()]);
        }

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 19,
                end: 27,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b" FGHIJ\r\n");
        assert!(fragmentizer.is_message_complete());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
    }

    #[track_caller]
    fn assert_is_line(
        unprocessed_bytes: &[u8],
        line_byte_count: usize,
        expected_announcement: Option<LiteralAnnouncement>,
        expected_ending: LineEnding,
    ) {
        let mut line_parser = LineParser::new(0);
        let unprocessed_bytes = unprocessed_bytes.iter().copied().collect();

        let (parsed_byte_count, fragment_info) = line_parser.parse(&unprocessed_bytes);

        assert_eq!(parsed_byte_count, line_byte_count);

        let Some(FragmentInfo::Line {
            start,
            end,
            announcement,
            ending,
        }) = fragment_info
        else {
            panic!("Unexpected fragment: {fragment_info:?}");
        };

        assert_eq!(start, 0);
        assert_eq!(end, line_byte_count);
        assert_eq!(announcement, expected_announcement);
        assert_eq!(ending, expected_ending);
    }

    #[test]
    fn fragmentizer_progress_multiple_messages_longer_than_max_size() {
        let mut fragmentizer = Fragmentizer::new(17);
        fragmentizer.enqueue_bytes(b"A1 NOOP\r\n");
        fragmentizer.enqueue_bytes(b"A2 LOGIN ABCDE EFGIJ\r\n");
        fragmentizer.enqueue_bytes(b"A3 LOGIN {5}\r\n");
        fragmentizer.enqueue_bytes(b"ABCDE");
        fragmentizer.enqueue_bytes(b" EFGIJ\r\n");
        fragmentizer.enqueue_bytes(b"A4 LOGIN A B\r\n");

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 9,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"A1 NOOP\r\n");
        assert_eq!(fragmentizer.message_bytes(), b"A1 NOOP\r\n");
        assert!(fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 22,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A2 LOGIN ABCDE EF"
        );
        assert_eq!(fragmentizer.message_bytes(), b"A2 LOGIN ABCDE EF");
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 14,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5
                }),
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A3 LOGIN {5}\r\n"
        );
        assert_eq!(fragmentizer.message_bytes(), b"A3 LOGIN {5}\r\n");
        assert!(!fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(fragment_info, FragmentInfo::Literal { start: 14, end: 19 });
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"ABC");
        assert_eq!(fragmentizer.message_bytes(), b"A3 LOGIN {5}\r\nABC");
        assert!(!fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 19,
                end: 27,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"");
        assert_eq!(fragmentizer.message_bytes(), b"A3 LOGIN {5}\r\nABC");
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 14,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A4 LOGIN A B\r\n"
        );
        assert_eq!(fragmentizer.message_bytes(), b"A4 LOGIN A B\r\n");
        assert!(fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_max_message_size_exceeded());
    }

    #[test]
    fn fragmentizer_progress_messages_with_zero_max_size() {
        let mut fragmentizer = Fragmentizer::new(0);
        fragmentizer.enqueue_bytes(b"A1 NOOP\r\n");
        fragmentizer.enqueue_bytes(b"A2 LOGIN ABCDE EFGIJ\r\n");
        fragmentizer.enqueue_bytes(b"A3 LOGIN {5}\r\n");
        fragmentizer.enqueue_bytes(b"ABCDE");
        fragmentizer.enqueue_bytes(b" EFGIJ\r\n");

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 9,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 22,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 14,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5
                }),
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(fragment_info, FragmentInfo::Literal { start: 14, end: 19 });
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 19,
                end: 27,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_max_message_size_exceeded());
    }

    #[test]
    fn fragmentizer_decode_message() {
        let command_codec = CommandCodec::new();
        let response_codec = ResponseCodec::new();

        let mut fragmentizer = Fragmentizer::new(10);
        fragmentizer.enqueue_bytes(b"A1 NOOP\r\n");
        fragmentizer.enqueue_bytes(b"A2 LOGIN ABCDE EFGIJ\r\n");

        fragmentizer.progress();
        assert_eq!(
            fragmentizer.decode_message(&command_codec),
            Ok(Command::new("A1", CommandBody::Noop).unwrap()),
        );
        assert_eq!(
            fragmentizer.decode_message(&response_codec),
            Err(DecodeMessageError::DecodingFailure(
                ResponseDecodeError::Failed
            )),
        );

        fragmentizer.progress();
        assert_eq!(
            fragmentizer.decode_message(&response_codec),
            Err(DecodeMessageError::MessageTooLong {
                initial: Secret::new(b"A2 LOGIN A"),
            }),
        );
    }

    #[test]
    fn fragmentizer_poison_message() {
        let command_codec = CommandCodec::new();

        let mut fragmentizer = Fragmentizer::without_max_message_size();
        fragmentizer.enqueue_bytes(b"A1 NOOP\r\n");
        fragmentizer.enqueue_bytes(b"A2 LOGIN {5}\r\n");
        fragmentizer.enqueue_bytes(b"ABCDE");
        fragmentizer.enqueue_bytes(b" EFGIJ\r\n");

        assert!(!fragmentizer.is_message_poisoned());

        fragmentizer.poison_message();

        assert!(fragmentizer.is_message_poisoned());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 9,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"A1 NOOP\r\n");
        assert_eq!(fragmentizer.message_bytes(), b"A1 NOOP\r\n");
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_message_poisoned());

        let decode_err = fragmentizer.decode_message(&command_codec).unwrap_err();

        assert_eq!(
            decode_err,
            DecodeMessageError::MessagePoisoned {
                discarded: Secret::new(fragmentizer.message_bytes())
            }
        );

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 14,
                announcement: Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length: 5
                }),
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(
            fragmentizer.fragment_bytes(fragment_info),
            b"A2 LOGIN {5}\r\n"
        );
        assert_eq!(fragmentizer.message_bytes(), b"A2 LOGIN {5}\r\n");
        assert!(!fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_message_poisoned());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(fragment_info, FragmentInfo::Literal { start: 14, end: 19 });
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"ABCDE");
        assert_eq!(fragmentizer.message_bytes(), b"A2 LOGIN {5}\r\nABCDE");
        assert!(!fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_message_poisoned());

        fragmentizer.poison_message();
        assert!(fragmentizer.is_message_poisoned());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 19,
                end: 27,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b" EFGIJ\r\n");
        assert_eq!(
            fragmentizer.message_bytes(),
            b"A2 LOGIN {5}\r\nABCDE EFGIJ\r\n"
        );
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_message_poisoned());

        let decode_err = fragmentizer.decode_message(&command_codec).unwrap_err();

        assert_eq!(
            decode_err,
            DecodeMessageError::MessagePoisoned {
                discarded: Secret::new(fragmentizer.message_bytes())
            }
        );

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_message_poisoned());
    }

    #[test]
    fn fragmentizer_poison_too_long_message() {
        let command_codec = CommandCodec::new();

        let mut fragmentizer = Fragmentizer::new(5);
        fragmentizer.enqueue_bytes(b"A1 NOOP\r\n");

        assert!(!fragmentizer.is_message_poisoned());

        fragmentizer.poison_message();

        assert!(fragmentizer.is_message_poisoned());

        let fragment_info = fragmentizer.progress().unwrap();

        assert_eq!(
            fragment_info,
            FragmentInfo::Line {
                start: 0,
                end: 9,
                announcement: None,
                ending: LineEnding::CrLf,
            }
        );
        assert_eq!(fragmentizer.fragment_bytes(fragment_info), b"A1 NO");
        assert_eq!(fragmentizer.message_bytes(), b"A1 NO");
        assert!(fragmentizer.is_message_complete());
        assert!(fragmentizer.is_max_message_size_exceeded());
        assert!(fragmentizer.is_message_poisoned());

        let decode_err = fragmentizer.decode_message(&command_codec).unwrap_err();

        assert_eq!(
            decode_err,
            DecodeMessageError::MessageTooLong {
                initial: Secret::new(b"A1 NO")
            }
        );

        let fragment_info = fragmentizer.progress();

        assert_eq!(fragment_info, None);
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert_eq!(fragmentizer.message_bytes(), b"");
        assert!(!fragmentizer.is_message_complete());
        assert!(!fragmentizer.is_max_message_size_exceeded());
        assert!(!fragmentizer.is_message_poisoned());
    }

    #[track_caller]
    fn assert_not_line(not_a_line_bytes: &[u8]) {
        let mut line_parser = LineParser::new(0);
        let not_a_line_bytes = not_a_line_bytes.iter().copied().collect();

        let (parsed_byte_count, fragment_info) = line_parser.parse(&not_a_line_bytes);

        assert_eq!(parsed_byte_count, not_a_line_bytes.len());
        assert_eq!(fragment_info, None);
    }

    #[test]
    fn parse_line_examples() {
        assert_not_line(b"");
        assert_not_line(b"foo");

        assert_is_line(b"\n", 1, None, LineEnding::Lf);
        assert_is_line(b"\r\n", 2, None, LineEnding::CrLf);
        assert_is_line(b"\n\r", 1, None, LineEnding::Lf);
        assert_is_line(b"foo\n", 4, None, LineEnding::Lf);
        assert_is_line(b"foo\r\n", 5, None, LineEnding::CrLf);
        assert_is_line(b"foo\n\r", 4, None, LineEnding::Lf);
        assert_is_line(b"foo\nbar\n", 4, None, LineEnding::Lf);
        assert_is_line(b"foo\r\nbar\r\n", 5, None, LineEnding::CrLf);
        assert_is_line(b"\r\nfoo\r\n", 2, None, LineEnding::CrLf);
        assert_is_line(
            b"{1}\r\n",
            5,
            Some(LiteralAnnouncement {
                length: 1,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(
            b"{1}\n",
            4,
            Some(LiteralAnnouncement {
                length: 1,
                mode: LiteralMode::Sync,
            }),
            LineEnding::Lf,
        );
        assert_is_line(
            b"foo {1}\r\n",
            9,
            Some(LiteralAnnouncement {
                length: 1,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(
            b"foo {2} {1}\r\n",
            13,
            Some(LiteralAnnouncement {
                length: 1,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(b"foo {1} \r\n", 10, None, LineEnding::CrLf);
        assert_is_line(b"foo \n {1}\r\n", 5, None, LineEnding::Lf);
        assert_is_line(b"foo {1} foo\r\n", 13, None, LineEnding::CrLf);
        assert_is_line(b"foo {1\r\n", 8, None, LineEnding::CrLf);
        assert_is_line(b"foo 1}\r\n", 8, None, LineEnding::CrLf);
        assert_is_line(b"foo { 1}\r\n", 10, None, LineEnding::CrLf);
        assert_is_line(
            b"foo {{1}\r\n",
            10,
            Some(LiteralAnnouncement {
                length: 1,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(
            b"foo {42}\r\n",
            10,
            Some(LiteralAnnouncement {
                length: 42,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(
            b"foo {42+}\r\n",
            11,
            Some(LiteralAnnouncement {
                length: 42,
                mode: LiteralMode::NonSync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(
            b"foo +{42}\r\n",
            11,
            Some(LiteralAnnouncement {
                length: 42,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(b"foo {+}\r\n", 9, None, LineEnding::CrLf);
        assert_is_line(b"foo {42++}\r\n", 12, None, LineEnding::CrLf);
        assert_is_line(b"foo {+42+}\r\n", 12, None, LineEnding::CrLf);
        assert_is_line(b"foo {+42}\r\n", 11, None, LineEnding::CrLf);
        assert_is_line(b"foo {42}+\r\n", 11, None, LineEnding::CrLf);
        assert_is_line(b"foo {-42}\r\n", 11, None, LineEnding::CrLf);
        assert_is_line(b"foo {42-}\r\n", 11, None, LineEnding::CrLf);
        assert_is_line(
            b"foo {4294967295}\r\n",
            18,
            Some(LiteralAnnouncement {
                length: 4294967295,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
        assert_is_line(b"foo {4294967296}\r\n", 18, None, LineEnding::CrLf);
    }

    #[test]
    fn parse_line_corner_case() {
        // According to the IMAP RFC, this line does not announce a literal.
        // We thought intensively about this corner case and asked different people.
        // Our conclusion: This corner case is an oversight of the RFC authors and
        // doesn't appear in the wild. We ignore it for now. If this becomes an issue
        // in practice then we should implement a detection for "* OK", "* NO" and
        // "* BAD".
        // See https://github.com/duesee/imap-codec/issues/432#issuecomment-1962427538
        assert_is_line(
            b"* OK {1}\r\n",
            10,
            Some(LiteralAnnouncement {
                length: 1,
                mode: LiteralMode::Sync,
            }),
            LineEnding::CrLf,
        );
    }

    #[test]
    fn parse_tag_examples() {
        assert_eq!(parse_tag(b"1 NOOP\r\n"), Tag::try_from("1").ok());
        assert_eq!(parse_tag(b"12 NOOP\r\n"), Tag::try_from("12").ok());
        assert_eq!(parse_tag(b"123 NOOP\r\n"), Tag::try_from("123").ok());
        assert_eq!(parse_tag(b"1234 NOOP\r\n"), Tag::try_from("1234").ok());
        assert_eq!(parse_tag(b"12345 NOOP\r\n"), Tag::try_from("12345").ok());

        assert_eq!(parse_tag(b"A1 NOOP\r\n"), Tag::try_from("A1").ok());
        assert_eq!(parse_tag(b"A1 NOOP"), Tag::try_from("A1").ok());
        assert_eq!(parse_tag(b"A1 "), Tag::try_from("A1").ok());
        assert_eq!(parse_tag(b"A1  "), Tag::try_from("A1").ok());
        assert_eq!(parse_tag(b"A1 \r\n"), Tag::try_from("A1").ok());
        assert_eq!(parse_tag(b"A1 \n"), Tag::try_from("A1").ok());
        assert_eq!(parse_tag(b"A1"), None);
        assert_eq!(parse_tag(b"A1\r\n"), None);
        assert_eq!(parse_tag(b"A1\n"), None);
        assert_eq!(parse_tag(b" \r\n"), None);
        assert_eq!(parse_tag(b"\r\n"), None);
        assert_eq!(parse_tag(b""), None);
        assert_eq!(parse_tag(b" A1 NOOP\r\n"), None);
    }
}
