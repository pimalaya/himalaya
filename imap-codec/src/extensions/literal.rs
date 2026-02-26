#[cfg(test)]
mod tests {
    use imap_types::{
        command::{Command, CommandBody},
        core::{Literal, Vec1},
        response::{Capability, Code, Greeting},
    };

    use crate::testing::{kat_inverse_command, kat_inverse_greeting};

    #[test]
    fn test_kat_inverse_command_login_literal_plus() {
        kat_inverse_command(&[
            (
                b"A LOGIN {0}\r\n {1}\r\nA\r\n".as_ref(),
                b"".as_ref(),
                Command::new(
                    "A",
                    CommandBody::login(
                        Literal::try_from("").unwrap(),
                        Literal::try_from("A").unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            ),
            (
                b"A LOGIN {1}\r\nA {2}\r\nAB\r\n?".as_ref(),
                b"?".as_ref(),
                Command::new(
                    "A",
                    CommandBody::login(
                        Literal::try_from("A").unwrap(),
                        Literal::try_from("AB").unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            ),
            (
                b"A LOGIN {0+}\r\n {1+}\r\nA\r\n??".as_ref(),
                b"??".as_ref(),
                Command::new(
                    "A",
                    CommandBody::login(
                        Literal::try_from("").unwrap().into_non_sync(),
                        Literal::try_from("A").unwrap().into_non_sync(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            ),
            (
                b"A LOGIN {1+}\r\nA {2+}\r\nAB\r\n???".as_ref(),
                b"???".as_ref(),
                Command::new(
                    "A",
                    CommandBody::login(
                        Literal::try_from("A").unwrap().into_non_sync(),
                        Literal::try_from("AB").unwrap().into_non_sync(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            ),
        ]);
    }

    #[test]
    fn test_kat_inverse_greeting_capability_literal_plus() {
        kat_inverse_greeting(&[
            (
                b"* OK [CAPABILITY LITERAL+] ...\r\n".as_ref(),
                b"".as_ref(),
                Greeting::ok(
                    Some(Code::Capability(Vec1::from(Capability::LiteralPlus))),
                    "...",
                )
                .unwrap(),
            ),
            (
                b"* OK [CAPABILITY LITERAL-] ...\r\n?",
                b"?",
                Greeting::ok(
                    Some(Code::Capability(Vec1::from(Capability::LiteralMinus))),
                    "...",
                )
                .unwrap(),
            ),
        ]);
    }
}
