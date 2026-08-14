use std::io::{Read, stdin};

use anyhow::Result;

/// Reads a raw RFC 5322 message either from the given argument or, when
/// absent, from standard input.
pub fn read_message(arg: Option<String>) -> Result<Vec<u8>> {
    match arg {
        Some(message) => Ok(message.into_bytes()),
        None => {
            let mut raw = String::new();
            stdin().read_to_string(&mut raw)?;
            Ok(raw.into_bytes())
        }
    }
}
