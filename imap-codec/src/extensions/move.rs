//! IMAP - MOVE Extension

use abnf_core::streaming::sp;
use imap_types::command::CommandBody;
use nom::{bytes::streaming::tag_no_case, sequence::tuple};

use crate::{decode::IMAPResult, mailbox::mailbox, sequence::sequence_set};

/// ```abnf
/// move = "MOVE" SP sequence-set SP mailbox
/// ```
pub(crate) fn r#move(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((tag_no_case(b"MOVE"), sp, sequence_set, sp, mailbox));

    let (remaining, (_, _, sequence_set, _, mailbox)) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Move {
            sequence_set,
            mailbox,
            uid: false,
        },
    ))
}

#[cfg(test)]
mod tests {
    use imap_types::command::{Command, CommandBody};

    use crate::testing::kat_inverse_command;

    #[test]
    fn test_kat_inverse_command_move() {
        kat_inverse_command(&[
            (
                b"A MOVE 1 INBOX\r\n".as_ref(),
                b"".as_ref(),
                Command::new("A", CommandBody::r#move("1", "inBox", false).unwrap()).unwrap(),
            ),
            (
                b"A UID MOVE 1 INBOX\r\n?",
                b"?",
                Command::new("A", CommandBody::r#move("1", "inBox", true).unwrap()).unwrap(),
            ),
            (
                b"A MOVE 1:* test\r\n??",
                b"??",
                Command::new("A", CommandBody::r#move("1:*", "test", false).unwrap()).unwrap(),
            ),
        ]);
    }
}
