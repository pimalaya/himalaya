use imap_codec::{
    AuthenticateDataCodec, CommandCodec, IdleDoneCodec,
    fragmentizer::{FragmentInfo, Fragmentizer, LiteralAnnouncement},
};

#[path = "common/common.rs"]
mod common;

use common::{COLOR_SERVER, RESET, read_more};
use imap_types::{
    IntoStatic,
    command::{Command, CommandBody},
    core::{LiteralMode, Tag},
};

use crate::common::Role;

enum State {
    Command,
    Authenticate(Tag<'static>),
    Idle,
}

const WELCOME: &str = r#"# Parsing of IMAP commands

"C:" denotes the client,
"S:" denotes the server, and
".." denotes the continuation of an (incomplete) command, e.g., due to the use of an IMAP literal.

Note: "\n" will be automatically replaced by "\r\n".

--------------------------------------------------------------------------------------------------

Enter IMAP commands (or "exit").
"#;

fn main() {
    println!("{WELCOME}");

    let mut fragmentizer = Fragmentizer::new(10 * 1024);
    let mut state = State::Command;

    // Send a greeting.
    println!("S: {COLOR_SERVER}* OK ...{RESET}");

    loop {
        // Progress next fragment.
        let Some(fragment_info) = fragmentizer.progress() else {
            // Read more bytes ...
            let bytes = read_more(Role::Client, fragmentizer.message_bytes().is_empty());

            // ... and pass the bytes to the Fragmentizer ...
            fragmentizer.enqueue_bytes(&bytes);

            // ... and try again.
            continue;
        };

        // The Fragmentizer detected a line that announces a sync literal.
        if let FragmentInfo::Line {
            announcement:
                Some(LiteralAnnouncement {
                    mode: LiteralMode::Sync,
                    length,
                }),
            ..
        } = fragment_info
        {
            // Check the length of the literal.
            if length <= 1024 {
                // Accept the literal ...
                println!("S: {COLOR_SERVER}+ {RESET}");

                // ... and continue with the remaining message.
                continue;
            } else if let Some(tag) = fragmentizer.decode_tag() {
                // Reject the literal ...
                println!("S: {COLOR_SERVER}{} BAD ...{RESET}", tag.as_ref());

                // ... and skip the current message ...
                fragmentizer.skip_message();

                // ... and continue with the next message.
                continue;
            } else {
                // The partially received message is malformed. It's unclear what will follow.
                // To be on the safe side, prevent the message from being decoded ...
                fragmentizer.poison_message();

                // ... but continue parsing the message.
                continue;
            }
        }

        // Check whether the Fragmentizer detected a complete message.
        if !fragmentizer.is_message_complete() {
            // Read next fragment.
            continue;
        }

        // The Fragmentizer detected a complete message.
        match state {
            State::Command => {
                match fragmentizer.decode_message(&CommandCodec::default()) {
                    Ok(Command {
                        tag,
                        body: CommandBody::Authenticate { .. },
                    }) => {
                        // Request another SASL round ...
                        println!("S: {COLOR_SERVER}+ {RESET}");

                        // ... and proceed with authenticate data.
                        state = State::Authenticate(tag.into_static());
                    }
                    Ok(Command {
                        body: CommandBody::Idle,
                        ..
                    }) => {
                        // Accept the idle ...
                        println!("S: {COLOR_SERVER}+ ...{RESET}");

                        // ... and proceed with idle done.
                        state = State::Idle;
                    }
                    Ok(command) => {
                        // Do something with the command.
                        println!("{command:#?}");
                    }
                    Err(err) => {
                        println!("Error parsing command: {err:?}");
                    }
                };
            }
            State::Authenticate(ref tag) => {
                match fragmentizer.decode_message(&AuthenticateDataCodec::default()) {
                    Ok(_authenticate_data) => {
                        // Accept the authentication after one SASL round.
                        println!("S: {COLOR_SERVER}{} OK ...{RESET}", tag.as_ref());

                        // ... and proceed with commands.
                        state = State::Command;
                    }
                    Err(err) => {
                        println!("Error parsing authenticate data: {err:?}");
                    }
                };
            }
            State::Idle => {
                match fragmentizer.decode_message(&IdleDoneCodec::default()) {
                    Ok(_idle_done) => {
                        // End idle and proceed with commands.
                        state = State::Command;
                    }
                    Err(err) => {
                        println!("Error parsing idle done: {err:?}");
                    }
                };
            }
        }
    }
}
