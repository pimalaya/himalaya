use std::{borrow::Cow, io::Write, num::NonZeroU32};

#[cfg(not(feature = "quirk_crlf_relaxed"))]
use abnf_core::streaming::crlf;
#[cfg(feature = "quirk_crlf_relaxed")]
use abnf_core::streaming::crlf_relaxed as crlf;
use imap_types::{
    core::LiteralMode,
    extensions::binary::{Literal8, LiteralOrLiteral8},
};
use nom::{
    bytes::streaming::{tag, take},
    character::streaming::char,
    combinator::{map, opt},
    sequence::{delimited, separated_pair, terminated, tuple},
};

use crate::{
    core::{number, nz_number},
    decode::{IMAPErrorKind, IMAPParseError, IMAPResult},
    encode::{EncodeContext, EncodeIntoContext},
    fetch::section_part,
};

/// See <https://datatracker.ietf.org/doc/html/rfc3516> and <https://datatracker.ietf.org/doc/html/rfc4466>
///
/// ```abnf
/// literal8 = "~{" number ["+"] "}" CRLF *OCTET
/// ;; <number> represents the number of OCTETs in the response string.
/// ;; The "+" is only allowed when both LITERAL+ and BINARY extensions are supported by the server.
/// ```
pub(crate) fn literal8(input: &[u8]) -> IMAPResult<&[u8], Literal8> {
    let (remaining, (length, mode)) = terminated(
        delimited(
            tag(b"~{"),
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

    Ok((
        remaining,
        Literal8 {
            data: Cow::Borrowed(data),
            mode,
        },
    ))
}

impl EncodeIntoContext for LiteralOrLiteral8<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            LiteralOrLiteral8::Literal(lit) => lit.encode_ctx(ctx),
            LiteralOrLiteral8::Literal8(lit8) => lit8.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for Literal8<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self.mode {
            LiteralMode::Sync => write!(ctx, "~{{{}}}\r\n", self.data.len())?,
            LiteralMode::NonSync => write!(ctx, "~{{{}+}}\r\n", self.data.len())?,
        }

        ctx.push_line();
        ctx.write_all(&self.data)?;
        ctx.push_literal(self.mode);

        Ok(())
    }
}

/// ```abnf
/// section-binary = "[" [section-part] "]"
/// ```
pub(crate) fn section_binary(input: &[u8]) -> IMAPResult<&[u8], Vec<NonZeroU32>> {
    delimited(
        tag("["),
        // We use `Vec<T>` instead of `Option<Vec1<T>>`.
        map(opt(section_part), |section_part| {
            section_part.map(|i| i.into_inner()).unwrap_or_default()
        }),
        tag("]"),
    )(input)
}

/// ```abnf
/// partial = "<" number "." nz-number ">"
/// ```
pub(crate) fn partial(input: &[u8]) -> IMAPResult<&[u8], (u32, NonZeroU32)> {
    delimited(
        tag(b"<"),
        separated_pair(number, tag(b"."), nz_number),
        tag(b">"),
    )(input)
}
