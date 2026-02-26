use std::fmt::Debug;

use imap_types::{
    auth::AuthenticateData,
    command::Command,
    extensions::idle::IdleDone,
    response::{Greeting, Response},
    utils::escape_byte_string,
};

use crate::{
    AuthenticateDataCodec, CommandCodec, GreetingCodec, IdleDoneCodec, ResponseCodec,
    decode::{Decoder, IMAPResult},
    encode::{EncodeContext, EncodeIntoContext},
};

pub(crate) fn known_answer_test_encode(
    (test_object, expected_bytes): (impl EncodeIntoContext, impl AsRef<[u8]>),
) {
    let expected_bytes = expected_bytes.as_ref();
    let mut ctx = EncodeContext::new();
    test_object.encode_ctx(&mut ctx).unwrap();

    let got_bytes = ctx.dump();
    let got_bytes = got_bytes.as_slice();

    if expected_bytes != got_bytes {
        println!("# Debug (`escape_byte_string`, encapsulated by `<<<` and `>>>`)");
        println!(
            "Left:  <<<{}>>>\nRight: <<<{}>>>",
            escape_byte_string(expected_bytes),
            escape_byte_string(got_bytes),
        );
        println!("# Debug");
        panic!("Left:  {expected_bytes:02x?}\nRight: {got_bytes:02x?}");
    }
}

pub(crate) fn known_answer_test_parse<'a, O, P>(
    (test, expected_remainder, expected_object): (&'a [u8], &[u8], O),
    parser: P,
) where
    O: Debug + Eq + 'a,
    P: Fn(&'a [u8]) -> IMAPResult<'a, &'a [u8], O>,
{
    let (got_remainder, got_object) = parser(test).unwrap();
    assert_eq!(expected_remainder, got_remainder);
    assert_eq!(expected_object, got_object);
}

// Note: Maybe there is a cleaner way to write this using generic bounds. However,
// we tried it and failed to provide a cleaner solution. Thus, it's a macro for now.
macro_rules! impl_kat_inverse {
    ($fn_name:ident, $decoder:ident, $item:ty) => {
        pub(crate) fn $fn_name(tests: &[(&[u8], &[u8], $item)]) {
            for (no, (test_input, expected_remainder, expected_object)) in tests.iter().enumerate()
            {
                println!("# {no}");

                let (got_remainder, got_object) = $decoder::default()
                    .decode(test_input)
                    .expect("first parsing failed");
                assert_eq!(*expected_object, got_object);
                assert_eq!(*expected_remainder, got_remainder);

                let mut ctx = EncodeContext::new();
                got_object.encode_ctx(&mut ctx).unwrap();

                let got_output = ctx.dump();

                // This second `decode` makes using generic bounds more complicated due to the
                // different lifetime.
                let (got_remainder, got_object_again) = $decoder::default()
                    .decode(&got_output)
                    .expect("second parsing failed");
                assert_eq!(got_object, got_object_again);
                assert!(got_remainder.is_empty());
            }
        }
    };
}

impl_kat_inverse! {kat_inverse_greeting, GreetingCodec, Greeting}
impl_kat_inverse! {kat_inverse_command, CommandCodec, Command}
impl_kat_inverse! {kat_inverse_response, ResponseCodec, Response}
//impl_kat_inverse! {kat_inverse_continue, ContinueCodec, Continue}
impl_kat_inverse! {kat_inverse_authenticate_data, AuthenticateDataCodec, AuthenticateData}
impl_kat_inverse! {kat_inverse_done, IdleDoneCodec, IdleDone}

#[cfg(test)]
mod tests {
    use imap_types::command::{Command, CommandBody};

    use super::*;

    #[test]
    #[should_panic]
    fn test_known_answer_test_encode() {
        known_answer_test_encode((Command::new("A", CommandBody::Noop).unwrap(), b""));
    }
}
