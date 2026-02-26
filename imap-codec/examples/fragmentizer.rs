use std::io::{Read, stdin};

use imap_codec::{fragmentizer::Fragmentizer, imap_types::utils::escape_byte_string};

fn main() {
    let mut fragmentizer = Fragmentizer::new(1024);

    loop {
        match fragmentizer.progress() {
            Some(fragment_info) => {
                println!(
                    "[!] Fragment: {fragment_info:#?} // b\"{}\"",
                    escape_byte_string(fragmentizer.fragment_bytes(fragment_info))
                );

                if fragmentizer.is_message_complete() {
                    println!(
                        "[!] Complete message: {}",
                        escape_byte_string(fragmentizer.message_bytes())
                    );
                }
            }
            None => {
                println!("[!] Reading stdin (ctrl+d to flush)...");
                let mut buffer = [0; 64];
                let count = stdin().read(&mut buffer).unwrap();
                if count == 0 {
                    println!("[!] Connection closed");
                    break;
                }
                let chunk = &buffer[..count];

                println!(
                    "[!] Enqueueing {} byte(s) (b\"{}\")",
                    count,
                    escape_byte_string(chunk)
                );
                fragmentizer.enqueue_bytes(chunk);
            }
        }
    }
}
