//! # IMAP raw
//!
//! The `imap raw` command, a byte-for-byte passthrough to the server.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::{imap::client::ImapClient, shared::raw::RawCommandArg};

/// Send raw IMAP commands and print the verbatim server response.
///
/// The input goes out byte for byte, no tag added and no CRLF trimmed, so
/// every command carries its own tag and a CRLF separates them. That is
/// what lets a whole batch be pipelined at once.
///
/// A literal `\r` or `\n` typed on the shell becomes a real CRLF, and a
/// trailing one is appended when missing. The response is read until
/// every tagged completion has arrived, possibly out of order, and a
/// tagged NO or BAD comes back as output rather than as an error.
#[derive(Debug, Parser)]
pub struct ImapRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl ImapRawCommand {
    /// Sends the commands and prints the raw response.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let command = terminate(self.command.parse()?);
        let response = client.raw(command.as_bytes())?;

        printer.out(Message::new(response))
    }
}

/// Appends the CRLF the caller may have left off the last command.
///
/// io-imap rejects an unterminated command, and it has to be a full CRLF:
/// a bare LF passes io-imap but servers such as Gmail never answer it, so
/// the exchange would stall until the stream times out.
fn terminate(mut command: String) -> String {
    if !command.ends_with("\r\n") {
        command.push_str("\r\n");
    }

    command
}

#[cfg(test)]
mod tests {
    use super::terminate;

    #[test]
    fn unterminated_command_gains_crlf() {
        assert_eq!(terminate("a1 NOOP".into()), "a1 NOOP\r\n");
    }

    #[test]
    fn terminated_command_is_untouched() {
        assert_eq!(terminate("a1 NOOP\r\n".into()), "a1 NOOP\r\n");
    }

    #[test]
    fn last_command_of_a_batch_gains_crlf() {
        assert_eq!(
            terminate("a1 SELECT INBOX\r\na2 NOOP".into()),
            "a1 SELECT INBOX\r\na2 NOOP\r\n",
        );
    }
}
