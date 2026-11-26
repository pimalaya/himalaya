//! # JMAP email read
//!
//! The `jmap email read` command, an `Email/get` asking for the body
//! values too.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::email::{JmapEmail, JmapEmailAddress};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{
    error::{ErrorCode, ItemError, partial_when, write_item_errors},
    jmap::{
        backend::not_found_errors,
        client::JmapClient,
        email::query::email_of,
        raw::{self, JmapRawList},
    },
    unified::message::read::sanitize_body,
};

/// Read the content of a JMAP email (Email/get with body).
///
/// Shows headers and plain text body by default. Under `--json` the
/// emails print as the server sent them. An id the server does not find
/// is an entry of `errors`, coded `message-not-found`, and exits 3.
#[derive(Debug, Parser)]
pub struct JmapEmailReadCommand {
    /// The email ID(s) to read.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
    /// Show HTML body instead of plain text.
    #[arg(long)]
    pub html: bool,
}

impl JmapEmailReadCommand {
    /// Fetches the emails with their bodies and prints them, then the
    /// error of each id not found.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let mut args = Map::new();
        let fetch = match self.html {
            true => "fetchHTMLBodyValues",
            false => "fetchTextBodyValues",
        };
        args.insert(String::from(fetch), json!(true));

        let list = raw::get_with(client, "Email", Some(&self.ids), args)?;
        let output = JmapEmailReadOutput::new(list, self.html);

        printer.out(&output)?;
        partial_when(&output.errors)
    }
}

/// Lays one email out as text: its headers then its plain bodies, or its
/// HTML bodies alone, every value from the server sanitized.
fn render(email: &JmapEmail, html: bool) -> String {
    let mut text = String::new();

    if !html {
        let mut header = |name: &str, value: &str| {
            text.push_str(&format!("{name}: {}\n", sanitize::text(value)));
        };

        if let Some(addrs) = &email.from {
            header("From", &format_addresses(addrs));
        }
        if let Some(addrs) = &email.to {
            header("To", &format_addresses(addrs));
        }
        if let Some(addrs) = &email.cc {
            header("Cc", &format_addresses(addrs));
        }
        if let Some(subject) = &email.subject {
            header("Subject", subject);
        }
        if let Some(date) = &email.sent_at {
            header("Date", date);
        }
    }

    let parts = if html {
        &email.html_body
    } else {
        &email.text_body
    };

    if let (Some(values), Some(parts)) = (&email.body_values, parts) {
        for part in parts {
            if let Some(value) = part.part_id.as_ref().and_then(|id| values.get(id)) {
                if !html {
                    text.push('\n');
                }
                text.push_str(&sanitize_body(&value.value));
                if !text.ends_with('\n') {
                    text.push('\n');
                }
            }
        }
    }

    text
}

/// The `jmap email read` output: the emails as the server sent them,
/// their headers and bodies laid out as text.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JmapEmailReadOutput {
    /// The emails found, each as the server sent it, with the body values
    /// asked for.
    #[schemars(with = "Vec<Map<String, Value>>")]
    emails: Vec<Value>,
    /// The text form of each email.
    #[serde(skip)]
    texts: Vec<String>,
    /// The error of each email the server did not find (`notFound`).
    errors: Vec<ItemError>,
}

impl JmapEmailReadOutput {
    /// The emails of a get as sent, each laid out as text, and the error
    /// of each id not found.
    fn new(list: JmapRawList, html: bool) -> Self {
        let texts = list
            .list
            .iter()
            .map(|email| render(&email_of(email), html))
            .collect();

        Self {
            emails: list.list,
            texts,
            errors: not_found_errors(list.not_found, ErrorCode::MessageNotFound),
        }
    }
}

impl fmt::Display for JmapEmailReadOutput {
    /// Separates the emails with a blank line, then lists the errors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, text) in self.texts.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{text}")?;
        }
        write_item_errors(f, &self.errors)
    }
}

fn format_addresses(addrs: &[JmapEmailAddress]) -> String {
    addrs
        .iter()
        .map(|a| match &a.name {
            Some(name) if !name.is_empty() => format!("{name} <{}>", a.email),
            _ => a.email.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use io_jmap::rfc8621::email::JmapEmail;
    use serde_json::json;

    use crate::jmap::{
        email::read::{JmapEmailReadOutput, render},
        raw::JmapRawList,
    };

    fn email(subject: &str, body: &str) -> JmapEmail {
        serde_json::from_value(serde_json::json!({
            "id": "E1",
            "from": [{"name": "Mallory\u{1b}]0;pwn\u{7}", "email": "m@x.org"}],
            "subject": subject,
            "textBody": [{"partId": "1"}],
            "htmlBody": [{"partId": "1"}],
            "bodyValues": {"1": {
                "value": body,
                "isEncodingProblem": false,
                "isTruncated": false,
            }},
        }))
        .unwrap()
    }

    #[test]
    fn controls_from_the_message_never_reach_the_terminal() {
        let text = render(
            &email("\u{1b}[31mred\u{202e}", "\u{1b}[32mgreen\tok\r\n"),
            false,
        );

        assert_eq!(
            text,
            "From: Mallory\u{fffd}]0;pwn\u{fffd} <m@x.org>\n\
             Subject: \u{fffd}[31mred\u{fffd}\n\
             \n\
             \u{fffd}[32mgreen\tok\n"
        );
    }

    /// The emails print as the server sent them, a property io-jmap does
    /// not model kept and none added, and an id not found is an item error.
    #[test]
    fn an_email_prints_as_sent_and_a_missing_id_is_an_item_error() {
        let sent = json!({"id": "E1", "subject": "hi", "x-vendor": 1});
        let list = JmapRawList {
            list: vec![sent.clone()],
            not_found: vec![String::from("E2")],
            rest: Default::default(),
        };

        let output = JmapEmailReadOutput::new(list, false);
        let json = serde_json::to_value(&output).unwrap();

        assert_eq!(json["emails"], json!([sent]));
        assert_eq!(json["errors"][0]["id"], "E2");
        assert_eq!(json["errors"][0]["code"], "message-not-found");
        assert!(output.to_string().contains("E2"), "{output}");
    }

    #[test]
    fn several_emails_are_set_apart() {
        let output = JmapEmailReadOutput {
            emails: Vec::new(),
            errors: Vec::new(),
            texts: vec![
                render(&email("a", "one"), true),
                render(&email("b", "two"), true),
            ],
        };

        assert_eq!(output.to_string(), "one\n\ntwo\n");
    }
}
