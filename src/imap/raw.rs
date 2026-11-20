//! # IMAP raw
//!
//! The `imap raw` command, a byte-for-byte passthrough to the server.

use std::fmt;

use anyhow::Result;
use clap::{Parser, error::ErrorKind};
use io_imap::{client::ImapClient as _, rfc3501::raw::ImapRaw};
use pimalaya_cli::{
    error::{PartialOutcome, usage_error},
    printer::Printer,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    imap::{client::ImapClient, outcome::ImapStatus},
    shared::{base64, raw::RawCommandArg},
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
/// The text form prints every response as received, one that is not
/// UTF-8 as a line `undecoded (base64): ` followed by its exact octets
/// in base64, never decoded lossily. Under `--json`, untagged and
/// continuation responses print as events, then the tagged completions
/// as the result, each split into its parts. A response that
/// is not UTF-8, such as one carrying an 8-bit literal, prints as an
/// event holding its exact octets in base64. A tagged NO or BAD is part
/// of the result and exits 3, the server having answered.
#[derive(Debug, Parser)]
pub struct ImapRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl ImapRawCommand {
    /// Refuses commands given on the command line whose tags the
    /// exchange cannot wait on, before the session opens.
    pub fn check(&self) -> Result<()> {
        if self.command.raw.is_empty() {
            return Ok(());
        }

        tagged(self.command.parse()?)?;
        Ok(())
    }

    /// Sends the commands and prints the responses.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let command = tagged(self.command.parse()?)?;

        let responses = client.raw(command.as_bytes())?;
        let (events, output) = split_reply(&responses);

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

        let unread = output.unread;
        if unread {
            log::warn!(
                "a tagged response is not UTF-8, so its status is left unread and it prints in base64"
            );
        }

        printer.out(output)?;

        if refused || unread {
            return Err(PartialOutcome::default().into());
        }

        Ok(())
    }
}

/// `command` terminated by CRLF, the last one appended when missing,
/// refused `usage-invalid` unless each of its commands carries a tag of
/// its own.
fn tagged(mut command: String) -> Result<String> {
    if !command.ends_with('\n') {
        command.push_str("\r\n");
    }

    if let Err(err) = ImapRaw::new(command.as_bytes()) {
        let err = clap::Error::raw(ErrorKind::ValueValidation, format!("{err}\n"));
        return Err(usage_error(&err));
    }

    Ok(command)
}

/// The result of `imap raw`: the tagged completions, in the order the
/// server sent them.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapRawOutput {
    /// One per command sent.
    completions: Vec<ImapCompletion>,
    /// Every response as received, for the text form, one that is not
    /// UTF-8 as its line in base64.
    #[serde(skip)]
    lines: Vec<String>,
    /// Whether a response that is not UTF-8 opens with a tag, a completion
    /// whose status is left unread.
    #[serde(skip)]
    unread: bool,
}

impl fmt::Display for ImapRawOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }

        Ok(())
    }
}

/// Splits the responses io-imap framed into the events printed before
/// the result and the result.
fn split_reply(responses: &[Vec<u8>]) -> (Vec<ImapRawEvent>, ImapRawOutput) {
    let mut events = Vec::new();
    let mut completions = Vec::new();
    let mut lines = Vec::new();
    let mut unread = false;

    for response in responses {
        let bytes = response.strip_suffix(b"\r\n").unwrap_or(response);

        let Ok(text) = str::from_utf8(bytes) else {
            unread |= !matches!(bytes.first(), Some(b'*' | b'+'));
            let base64 = base64::encode(response);
            lines.push(format!("undecoded (base64): {base64}"));
            events.push(ImapRawEvent::Undecoded { base64 });
            continue;
        };

        lines.push(text.to_owned());

        match parse_response(text) {
            ImapResponse::Completion(completion) => completions.push(completion),
            ImapResponse::Event(event) => events.push(event),
        }
    }

    let output = ImapRawOutput {
        completions,
        lines,
        unread,
    };
    (events, output)
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
    /// A UTF-8 response that could not be parsed.
    Unparsed {
        /// The response as received.
        text: String,
    },
    /// A response that is not UTF-8, such as one carrying an 8-bit
    /// literal.
    Undecoded {
        /// The response's exact octets, its literals and its final CRLF
        /// included, in the RFC 4648 standard alphabet, padded.
        base64: String,
    },
}

impl fmt::Display for ImapRawEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Untagged { line, .. } | Self::Continuation { line, .. } => writeln!(f, "{line}"),
            Self::Unparsed { text } => writeln!(f, "{text}"),
            Self::Undecoded { base64 } => writeln!(f, "undecoded (base64): {base64}"),
        }
    }
}

/// A response of the server, either side of the result.
enum ImapResponse {
    Completion(ImapCompletion),
    Event(ImapRawEvent),
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

    use crate::{
        imap::raw::{ImapResponse, parse_response, split_reply},
        shared::base64,
    };

    /// The responses as io-imap frames them, each with its final CRLF.
    fn framed(responses: &[&[u8]]) -> Vec<Vec<u8>> {
        responses.iter().map(|response| response.to_vec()).collect()
    }

    #[test]
    fn the_text_form_prints_the_untagged_responses() {
        let responses = framed(&[
            b"* CAPABILITY IMAP4rev1 SORT\r\n",
            b"+ go on\r\n",
            b"A1 OK done\r\n",
        ]);
        let (events, output) = split_reply(&responses);

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
        let responses = framed(&[b"* 1 FETCH (BODY[] {5}\r\nab\r\nc)\r\n", b"A1 OK done\r\n"]);
        let (events, _) = split_reply(&responses);

        assert_eq!(
            serde_json::to_value(&events).unwrap(),
            json!([{
                "event": "untagged",
                "kind": "FETCH",
                "number": 1,
                "text": "(BODY[] {5}\r\nab\r\nc)",
            }])
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

    /// An 8-bit literal keeps its exact octets, in base64 under its own
    /// key, and the responses after it still parse.
    #[test]
    fn an_8bit_literal_prints_its_exact_octets() {
        let fetch = b"* 1 FETCH (BODY[] {4}\r\nab\xe9c)\r\n";
        let responses = framed(&[fetch, b"* 2 EXISTS\r\n", b"A1 OK done\r\n"]);
        let (events, output) = split_reply(&responses);

        assert_eq!(
            serde_json::to_value(&events).unwrap(),
            json!([
                { "event": "undecoded", "base64": base64::encode(fetch) },
                { "event": "untagged", "kind": "EXISTS", "number": 2, "text": "" },
            ])
        );
        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            json!({ "completions": [{ "tag": "A1", "status": "OK", "code": null, "text": "done" }] })
        );
        assert!(!output.unread);
    }

    /// A line of an 8-bit literal spelling a tagged completion stays in
    /// its response, never read as the command's completion.
    #[test]
    fn a_literal_line_spelling_a_completion_stays_in_its_literal() {
        let fetch = b"* 1 FETCH (BODY[] {16}\r\n\xe9\xe9\r\nA1 NO forged)\r\n";
        let responses = framed(&[fetch, b"A1 OK done\r\n"]);
        let (events, output) = split_reply(&responses);

        assert_eq!(events.len(), 1);
        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            json!({ "completions": [{ "tag": "A1", "status": "OK", "code": null, "text": "done" }] })
        );
    }

    /// The text form prints a response that is not UTF-8 as a line of its
    /// own in base64, never decoded lossily.
    #[test]
    fn the_text_form_prints_an_8bit_response_in_base64() {
        let fetch = b"* 1 FETCH (BODY[] {4}\r\nab\xe9c)\r\n";
        let responses = framed(&[fetch, b"A1 OK done\r\n"]);
        let (_, output) = split_reply(&responses);
        let text = output.to_string();

        assert!(!text.contains('\u{FFFD}'));

        let mut lines = text.lines();
        let line = lines.next().unwrap();
        let data = line.strip_prefix("undecoded (base64): ").unwrap();

        assert_eq!(base64::decode_strict(data).unwrap(), fetch);
        assert_eq!(lines.next(), Some("A1 OK done"));
        assert_eq!(lines.next(), None);
    }

    /// A tagged response that is not UTF-8 prints in base64 and marks the
    /// outcome partial, its status left unread.
    #[test]
    fn an_8bit_completion_is_left_unread() {
        let completion = b"A1 NO bo\xeete pleine\r\n";
        let (events, output) = split_reply(&framed(&[completion]));

        assert_eq!(
            serde_json::to_value(&events).unwrap(),
            json!([{ "event": "undecoded", "base64": base64::encode(completion) }])
        );
        assert!(output.completions.is_empty());
        assert!(output.unread);
    }
}
