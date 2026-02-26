use imap_codec::{GreetingCodec, ResponseCodec, fragmentizer::Fragmentizer};

#[path = "common/common.rs"]
mod common;

use common::read_more;

use crate::common::Role;

enum State {
    Greeting,
    Response,
}

const WELCOME: &str = r#"# Parsing of IMAP greeting and responses

"S:" denotes the server.
".." denotes the continuation of an (incomplete) response, e.g., due to the use of an IMAP literal.

Note: "\n" will be automatically replaced by "\r\n".

--------------------------------------------------------------------------------------------------

Enter intial IMAP greeting followed by IMAP responses (or "exit").
"#;

fn main() {
    println!("{WELCOME}");

    let mut fragmentizer = Fragmentizer::new(10 * 1024);
    let mut state = State::Greeting;

    loop {
        // Progress next fragment.
        let Some(_fragment_info) = fragmentizer.progress() else {
            // Read more bytes ...
            let bytes = read_more(Role::Server, fragmentizer.message_bytes().is_empty());

            // ... and pass the bytes to the Fragmentizer ...
            fragmentizer.enqueue_bytes(&bytes);

            // ... and try again.
            continue;
        };

        // Check whether the Fragmentizer detected a complete message.
        if !fragmentizer.is_message_complete() {
            // Read next fragment.
            continue;
        }

        // The Fragmentizer detected a complete message.
        match state {
            State::Greeting => {
                match fragmentizer.decode_message(&GreetingCodec::default()) {
                    Ok(greeting) => {
                        // Do something with the greeting ...
                        println!("{greeting:#?}");

                        // ... and proceed with reponses.
                        state = State::Response;
                    }
                    Err(err) => {
                        println!("Error parsing greeting: {err:?}");
                    }
                };
            }
            State::Response => {
                match fragmentizer.decode_message(&ResponseCodec::default()) {
                    Ok(response) => {
                        // Do something with the response.
                        println!("{response:#?}");
                    }
                    Err(err) => {
                        println!("Error parsing response: {err:?}");
                    }
                };
            }
        };
    }
}
