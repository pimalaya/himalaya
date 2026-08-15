use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::{imap::client::ImapClient, shared::raw::RawCommandArg};

/// Send one or more raw IMAP commands and print the verbatim server
/// response.
///
/// The input is sent to the server byte-for-byte: no tag is added and no
/// CRLF is trimmed. Every command must therefore carry its own tag and be
/// separated by CRLF, which lets you pipeline a whole batch, e.g.
/// `a1 SELECT INBOX\r\na2 SEARCH ALL\r\n`. Literal `\r`/`\n` escapes typed
/// on the shell are turned into real CRLF, and a trailing CRLF is appended
/// when missing. The response is read until every command's tagged
/// completion has arrived (possibly out of order). Tagged NO/BAD replies
/// are returned as output, not errors.
#[derive(Debug, Parser)]
pub struct ImapRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl ImapRawCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mut command = self.command.parse()?;

        // NOTE: io-imap rejects an unterminated command, so append the
        // final newline the caller may have omitted on the last (or
        // only) command.
        if !command.ends_with('\n') {
            command.push('\n');
        }

        let response = client.raw(command.as_bytes())?;

        printer.out(Message::new(response))
    }
}
