//! The IMAP COMPRESS Extension

// Additional changes:
//
// command-auth   =/ compress
// capability     =/ "COMPRESS=" algorithm
// resp-text-code =/ "COMPRESSIONACTIVE"

use std::io::Write;

use imap_types::{command::CommandBody, extensions::compress::CompressionAlgorithm};
use nom::{
    bytes::streaming::tag_no_case,
    combinator::{map, value},
    sequence::preceded,
};

use crate::{
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext},
};

/// `algorithm = "DEFLATE"`
pub(crate) fn algorithm(input: &[u8]) -> IMAPResult<&[u8], CompressionAlgorithm> {
    value(CompressionAlgorithm::Deflate, tag_no_case("DEFLATE"))(input)
}

/// `compress = "COMPRESS" SP algorithm`
pub(crate) fn compress(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    map(preceded(tag_no_case("COMPRESS "), algorithm), |algorithm| {
        CommandBody::Compress { algorithm }
    })(input)
}

impl EncodeIntoContext for CompressionAlgorithm {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use imap_types::command::{Command, CommandBody};

    use super::*;
    use crate::testing::kat_inverse_command;

    #[test]
    fn test_parse_compress() {
        let tests = [
            (
                b"compress deflate ".as_ref(),
                Ok((
                    b" ".as_ref(),
                    CommandBody::compress(CompressionAlgorithm::Deflate),
                )),
            ),
            (b"compress deflat ".as_ref(), Err(())),
            (b"compres deflate ".as_ref(), Err(())),
            (b"compress  deflate ".as_ref(), Err(())),
        ];

        for (test, expected) in tests {
            match expected {
                Ok((expected_rem, expected_object)) => {
                    let (got_rem, got_object) = compress(test).unwrap();
                    assert_eq!(expected_object, got_object);
                    assert_eq!(expected_rem, got_rem);
                }
                Err(_) => {
                    assert!(compress(test).is_err())
                }
            }
        }
    }

    #[test]
    fn test_kat_inverse_body_compress() {
        kat_inverse_command(&[
            (
                b"A COMPRESS DEFLATE\r\n".as_ref(),
                b"".as_ref(),
                Command::new("A", CommandBody::compress(CompressionAlgorithm::Deflate)).unwrap(),
            ),
            (
                b"A COMPRESS DEFLATE\r\n?".as_ref(),
                b"?".as_ref(),
                Command::new("A", CommandBody::compress(CompressionAlgorithm::Deflate)).unwrap(),
            ),
        ]);
    }
}
