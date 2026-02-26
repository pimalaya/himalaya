use std::{io::Write, str::from_utf8};

use abnf_core::streaming::dquote;
use imap_types::{
    extensions::utf8::QuotedUtf8,
    utils::{escape_quoted, indicators::is_quoted_specials, unescape_quoted},
};
use nom::{
    bytes::streaming::{escaped, take_while1},
    character::streaming::one_of,
    combinator::map_res,
    sequence::tuple,
};

use crate::{
    decode::IMAPResult,
    encode::{EncodeContext, EncodeIntoContext},
};

impl EncodeIntoContext for QuotedUtf8<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "\"{}\"", escape_quoted(self.0.as_ref()))
    }
}

/// ```abnf
/// ; QUOTED-CHAR is not modified, as it will affect other RFC 3501 ABNF non-terminals.
/// quoted =/ DQUOTE *uQUOTED-CHAR DQUOTE
///
/// uQUOTED-CHAR  = QUOTED-CHAR / UTF8-2 / UTF8-3 / UTF8-4
/// UTF8-2        =   <Defined in Section 4 of RFC 3629>
/// UTF8-3        =   <Defined in Section 4 of RFC 3629>
/// UTF8-4        =   <Defined in Section 4 of RFC 3629>
/// ```
///
/// This function only allocates a new String, when needed, i.e. when
/// quoted chars need to be replaced.
pub(crate) fn quoted_utf8(input: &[u8]) -> IMAPResult<&[u8], QuotedUtf8> {
    let mut parser = tuple((
        dquote,
        map_res(
            escaped(
                take_while1(|c| !is_quoted_specials(c)),
                '\\',
                one_of("\\\""),
            ),
            from_utf8,
        ),
        dquote,
    ));

    let (remaining, (_, quoted_utf8, _)) = parser(input)?;

    Ok((remaining, QuotedUtf8(unescape_quoted(quoted_utf8))))
}

#[cfg(test)]
mod test {
    use imap_types::extensions::utf8::QuotedUtf8;

    use super::quoted_utf8;

    #[test]
    fn test_quoted_utf8() {
        assert_eq!(
            (b"".as_ref(), QuotedUtf8("äö¹".into())),
            quoted_utf8("\"äö¹\"".as_bytes()).unwrap()
        );
    }
}
