//! The IMAP NAMESPACE Extension

use std::io::Write;

use abnf_core::streaming::{dquote, sp};
use imap_types::{
    command::CommandBody,
    core::Vec1,
    extensions::namespace::{Namespace, NamespaceResponseExtension, Namespaces},
    response::Data,
};
use nom::{
    branch::alt,
    bytes::streaming::tag_no_case,
    character::streaming::char,
    combinator::{map, value},
    multi::{many0, many1, separated_list1},
    sequence::{delimited, preceded, tuple},
};

use crate::{
    core::{nil, quoted_char, string},
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext},
};

/// ```abnf
/// namespace = "NAMESPACE"
/// ```
pub(crate) fn namespace_command(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    value(CommandBody::Namespace, tag_no_case(b"NAMESPACE"))(input)
}

/// ```abnf
/// ;; The first Namespace is the Personal Namespace(s)
/// ;; The second Namespace is the Other Users' Namespace(s)
/// ;; The third Namespace is the Shared Namespace(s)
/// Namespace-Response = "NAMESPACE" SP Namespace SP Namespace SP Namespace
/// ```
pub(crate) fn namespace_response(input: &[u8]) -> IMAPResult<&[u8], Data> {
    let mut parser = tuple((
        tag_no_case("NAMESPACE "),
        namespace,
        preceded(sp, namespace),
        preceded(sp, namespace),
    ));

    let (remaining, (_, personal, other, shared)) = parser(input)?;

    Ok((
        remaining,
        Data::Namespace {
            personal,
            other,
            shared,
        },
    ))
}

/// ```abnf
/// Namespace = nil / "(" 1*Namespace-Descr ")"
/// ```
fn namespace(input: &[u8]) -> IMAPResult<&[u8], Namespaces> {
    alt((
        map(nil, |_| Vec::new()),
        delimited(char('('), many1(namespace_descr), char(')')),
    ))(input)
}

/// ```abnf
/// Namespace-Descr = "("
///                     string
///                     SP
///                     (DQUOTE QUOTED-CHAR DQUOTE / nil)
///                     *(Namespace-Response-Extension)
///                   ")"
/// ```
fn namespace_descr(input: &[u8]) -> IMAPResult<&[u8], Namespace> {
    map(
        delimited(
            char('('),
            tuple((
                string,
                sp,
                alt((
                    map(delimited(dquote, quoted_char, dquote), Some),
                    value(None, nil),
                )),
                many0(namespace_response_extension),
            )),
            char(')'),
        ),
        |(prefix, _, delimiter, extensions)| Namespace {
            prefix,
            delimiter,
            extensions,
        },
    )(input)
}

/// ```abnf
/// Namespace-Response-Extension = SP string SP "(" string *(SP string) ")"
/// ```
fn namespace_response_extension(input: &[u8]) -> IMAPResult<&[u8], NamespaceResponseExtension> {
    map(
        tuple((
            preceded(sp, string),
            preceded(
                sp,
                delimited(char('('), separated_list1(sp, string), char(')')),
            ),
        )),
        |(key, values)| NamespaceResponseExtension {
            key,
            // We can use `unvalidated` because we know the vector has at least one element due to the `separated_list1` call above.
            values: Vec1::unvalidated(values),
        },
    )(input)
}

impl EncodeIntoContext for Namespace<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "(")?;
        self.prefix.encode_ctx(ctx)?;
        write!(ctx, " ")?;

        match &self.delimiter {
            Some(delimiter_char) => {
                write!(ctx, "\"")?;
                delimiter_char.encode_ctx(ctx)?;
                write!(ctx, "\"")?;
            }
            None => {
                ctx.write_all(b"NIL")?;
            }
        }

        for ext in &self.extensions {
            ext.encode_ctx(ctx)?;
        }

        write!(ctx, ")")
    }
}

impl EncodeIntoContext for NamespaceResponseExtension<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, " ")?;
        self.key.encode_ctx(ctx)?;

        write!(ctx, " (")?;
        if let Some((last, head)) = self.values.as_ref().split_last() {
            for value in head {
                value.encode_ctx(ctx)?;
                write!(ctx, " ")?;
            }
            last.encode_ctx(ctx)?;
        }
        write!(ctx, ")")
    }
}

pub fn encode_namespaces(ctx: &mut EncodeContext, list: &Namespaces<'_>) -> std::io::Result<()> {
    if list.is_empty() {
        ctx.write_all(b"NIL")
    } else {
        ctx.write_all(b"(")?;
        for desc in list {
            desc.encode_ctx(ctx)?;
        }
        ctx.write_all(b")")
    }
}

#[cfg(test)]
mod tests {
    use super::namespace_response;

    #[test]
    fn parse_namespace_response() {
        let tests = [
            b"NAMESPACE ((\"0\" \"\\\"\")) NIL NIL\r\n".as_ref(),
            #[cfg(feature = "ext_utf8")]
            b"NAMESPACE ((\"^^\x00\" \"\x07\")) NIL NIL\r\n",
        ];

        for test in tests.into_iter() {
            namespace_response(test).unwrap();
        }
    }
}
