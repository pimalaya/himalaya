//! # JMAP email read
//!
//! The `jmap email read` command, an `Email/get` asking for the body
//! values too.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::email::{JmapEmail, JmapEmailAddress, get::JmapEmailGetOptions};
use log::warn;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{jmap::client::JmapClient, shared::message::read::sanitize_body};

/// Read the content of a JMAP email (Email/get with body).
///
/// Shows headers and plain text body by default.
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
    /// Fetches the emails with their bodies and prints them, failing
    /// when none of the ids names one.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let opts = JmapEmailGetOptions {
            properties: None,
            fetch_text_body_values: !self.html,
            fetch_html_body_values: self.html,
            max_body_value_bytes: 0,
        };
        let output = client.email_get(self.ids.clone(), opts)?;

        if output.emails.is_empty() {
            bail!("No JMAP email found: {}", output.not_found.join(", "));
        }

        for id in output.not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let texts = output
            .emails
            .iter()
            .map(|email| render(email, self.html))
            .collect();

        printer.out(JmapEmailReadOutput {
            emails: output.emails,
            texts,
        })
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
    /// The emails found, with the body values asked for.
    emails: Vec<JmapEmail>,
    /// The text form of each email.
    #[serde(skip)]
    texts: Vec<String>,
}

impl fmt::Display for JmapEmailReadOutput {
    /// Separates the emails with a blank line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, text) in self.texts.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{text}")?;
        }
        Ok(())
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

    use crate::jmap::email::read::{JmapEmailReadOutput, render};

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

    #[test]
    fn several_emails_are_set_apart() {
        let output = JmapEmailReadOutput {
            emails: Vec::new(),
            texts: vec![
                render(&email("a", "one"), true),
                render(&email("b", "two"), true),
            ],
        };

        assert_eq!(output.to_string(), "one\n\ntwo\n");
    }
}
