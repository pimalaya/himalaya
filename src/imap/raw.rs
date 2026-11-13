//! # IMAP raw
//!
//! The `imap raw` command, a byte-for-byte passthrough to the server.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::{error::PartialOutcome, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    imap::{client::ImapClient, outcome::ImapStatus},
    shared::raw::RawCommandArg,
};

/// Send raw IMAP commands and print the server's responses.
///
/// The input goes out byte for byte, no tag added and no CRLF trimmed, so
/// every command carries its own tag and a CRLF separates them. That is
/// what lets a whole batch be pipelined at once.
///
/// A literal `\r` or `\n` typed on the shell becomes a real CRLF, and a
/// trailing one is appended when missing. The response is read until
/// every tagged completion has arrived, possibly out of order.
///
/// The text form prints every response as received. Under `--json`,
/// untagged and continuation responses print as events, then the tagged
/// completions as the result, each split into its parts. A tagged NO or
/// BAD is part of the result and exits 3, the server having answered.
#[derive(Debug, Parser)]
pub struct ImapRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl ImapRawCommand {
    /// Sends the commands and prints the responses.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mut command = self.command.parse()?;

        // NOTE: io-imap rejects an unterminated command, so the CRLF the
        // caller may have left off the last one is appended.
        if !command.ends_with('\n') {
            command.push_str("\r\n");
        }

        let reply = client.raw(command.as_bytes())?;
        let (events, output) = split_reply(&reply);

        // NOTE: a text event reaches a terminal only, and the untagged
        // responses are what a raw command asks for, so the text form
        // prints them within the result instead.
        if printer.is_json() {
            for event in events {
                printer.event(event)?;
            }
        }

        let refused = output
            .completions
            .iter()
            .any(|completion| completion.status != ImapStatus::Ok);

        printer.out(output)?;

        if refused {
            return Err(PartialOutcome::default().into());
        }

        Ok(())
    }
}

/// The result of `imap raw`: the tagged completions, in the order the
/// server sent them.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapRawOutput {
    /// One per command sent.
    completions: Vec<ImapCompletion>,
    /// Every response as received, for the text form.
    #[serde(skip)]
    lines: Vec<String>,
}

impl fmt::Display for ImapRawOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }

        Ok(())
    }
}

/// Splits a raw reply into the events printed before the result and the
/// result.
fn split_reply(reply: &str) -> (Vec<ImapRawEvent>, ImapRawOutput) {
    let mut events = Vec::new();
    let mut completions = Vec::new();
    let mut lines = Vec::new();

    for response in split_responses(reply) {
        lines.push(response.to_owned());

        match parse_response(response) {
            ImapResponse::Completion(completion) => completions.push(completion),
            ImapResponse::Event(event) => events.push(event),
        }
    }

    (events, ImapRawOutput { completions, lines })
}

/// A tagged completion, split into its parts.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapCompletion {
    /// The tag of the command it completes.
    tag: String,
    /// `OK`, `NO` or `BAD`.
    status: ImapStatus,
    /// The response code between brackets, `null` when there is none.
    code: Option<String>,
    /// The human-readable text.
    text: String,
}

/// What `imap raw` prints before its result: each response the server
/// sent that is not a tagged completion.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub(crate) enum ImapRawEvent {
    /// An untagged response, `* ...`.
    Untagged {
        /// The response kind, as spelled on the wire: `EXISTS`, `FETCH`,
        /// `OK`, `CAPABILITY`...
        kind: String,
        /// The number before the kind, as in `* 3 EXISTS`, `null` when
        /// there is none.
        number: Option<u32>,
        /// What follows the kind.
        text: String,
        /// The line as received, for the text form.
        #[serde(skip)]
        line: String,
    },
    /// A continuation request, `+ ...`.
    Continuation {
        /// What follows the `+`.
        text: String,
        /// The line as received, for the text form.
        #[serde(skip)]
        line: String,
    },
    /// A response that could not be parsed.
    Unparsed {
        /// The response as received.
        text: String,
    },
}

impl fmt::Display for ImapRawEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Untagged { line, .. } | Self::Continuation { line, .. } => writeln!(f, "{line}"),
            Self::Unparsed { text } => writeln!(f, "{text}"),
        }
    }
}

/// A response of the server, either side of the result.
enum ImapResponse {
    Completion(ImapCompletion),
    Event(ImapRawEvent),
}

/// Splits a raw reply into its responses: lines, a literal's announced
/// octets kept with the line announcing them.
fn split_responses(reply: &str) -> Vec<&str> {
    let bytes = reply.as_bytes();
    let mut responses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;

    while let Some(offset) = reply[cursor..].find("\r\n") {
        let end = cursor + offset;

        // NOTE: a line ending with `{n}` or `{n+}` announces n octets of
        // literal, which belong to the same response.
        if let Some(size) = literal_size(&reply[cursor..end]) {
            cursor = (end + 2 + size).min(bytes.len());
            continue;
        }

        responses.push(&reply[start..end]);
        start = end + 2;
        cursor = start;
    }

    if start < reply.len() && !reply[start..].trim().is_empty() {
        responses.push(&reply[start..]);
    }

    responses
}

/// The octet count a line announces as literal, if it ends with one.
fn literal_size(line: &str) -> Option<usize> {
    let inner = line.strip_suffix('}')?;
    let (_, size) = inner.rsplit_once('{')?;
    size.trim_end_matches('+').parse().ok()
}

/// Parses one response.
fn parse_response(response: &str) -> ImapResponse {
    let unparsed = || {
        ImapResponse::Event(ImapRawEvent::Unparsed {
            text: response.to_owned(),
        })
    };

    if let Some(rest) = response.strip_prefix("* ") {
        let (number, rest) = match rest.split_once(' ') {
            Some((number, rest)) if number.bytes().all(|b| b.is_ascii_digit()) => {
                (number.parse().ok(), rest)
            }
            _ => (None, rest),
        };

        let (kind, text) = rest.split_once(' ').unwrap_or((rest, ""));

        if kind.is_empty() {
            return unparsed();
        }

        return ImapResponse::Event(ImapRawEvent::Untagged {
            kind: kind.to_owned(),
            number,
            text: text.to_owned(),
            line: response.to_owned(),
        });
    }

    if let Some(text) = response.strip_prefix('+') {
        return ImapResponse::Event(ImapRawEvent::Continuation {
            text: text.trim_start().to_owned(),
            line: response.to_owned(),
        });
    }

    let Some((tag, rest)) = response.split_once(' ') else {
        return unparsed();
    };

    let (status, rest) = rest.split_once(' ').unwrap_or((rest, ""));

    let status = match status.to_ascii_uppercase().as_str() {
        "OK" => ImapStatus::Ok,
        "NO" => ImapStatus::No,
        "BAD" => ImapStatus::Bad,
        _ => return unparsed(),
    };

    let (code, text) = match rest.strip_prefix('[').and_then(|rest| rest.split_once(']')) {
        Some((code, text)) => (Some(code.to_owned()), text.trim_start()),
        None => (None, rest),
    };

    ImapResponse::Completion(ImapCompletion {
        tag: tag.to_owned(),
        status,
        code,
        text: text.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::imap::raw::{ImapResponse, parse_response, split_reply, split_responses};

    #[test]
    fn the_text_form_prints_the_untagged_responses() {
        let reply = "* CAPABILITY IMAP4rev1 SORT\r\n+ go on\r\nA1 OK done\r\n";
        let (events, output) = split_reply(reply);

        assert_eq!(events.len(), 2);
        assert_eq!(
            output.to_string(),
            "* CAPABILITY IMAP4rev1 SORT\n+ go on\nA1 OK done\n"
        );
        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            json!({ "completions": [{ "tag": "A1", "status": "OK", "code": null, "text": "done" }] })
        );
    }

    #[test]
    fn a_literal_stays_in_its_response() {
        let reply = "* 1 FETCH (BODY[] {5}\r\nab\r\nc)\r\n* 2 EXISTS\r\nA1 OK done\r\n";

        assert_eq!(
            split_responses(reply),
            [
                "* 1 FETCH (BODY[] {5}\r\nab\r\nc)",
                "* 2 EXISTS",
                "A1 OK done"
            ]
        );
    }

    #[test]
    fn responses_split_into_events_and_completions() {
        let json = |response| match parse_response(response) {
            ImapResponse::Event(event) => serde_json::to_value(event).unwrap(),
            ImapResponse::Completion(completion) => serde_json::to_value(completion).unwrap(),
        };

        assert_eq!(
            json("* 172 EXISTS"),
            json!({ "event": "untagged", "kind": "EXISTS", "number": 172, "text": "" })
        );
        assert_eq!(
            json("* OK [UIDVALIDITY 3857529045] UIDs valid"),
            json!({
                "event": "untagged",
                "kind": "OK",
                "number": null,
                "text": "[UIDVALIDITY 3857529045] UIDs valid",
            })
        );
        assert_eq!(
            json("A1 OK [READ-WRITE] SELECT completed"),
            json!({ "tag": "A1", "status": "OK", "code": "READ-WRITE", "text": "SELECT completed" })
        );
        assert_eq!(
            json("A2 NO no such mailbox"),
            json!({ "tag": "A2", "status": "NO", "code": null, "text": "no such mailbox" })
        );
        assert_eq!(
            json("garbage"),
            json!({ "event": "unparsed", "text": "garbage" })
        );
    }
}
