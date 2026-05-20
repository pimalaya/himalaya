// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use anyhow::{Result, anyhow, bail};
use clap::Parser;
use percent_encoding::percent_decode_str;
use pimalaya_cli::printer::Printer;
use url::Url;

use crate::shared::{
    client::EmailClient,
    messages::{
        builder::{self, BuilderArgs},
        output, runner,
    },
};

/// Compose a new message from a `mailto:` URI, opening a user-defined
/// composer with the URI's recipient and headers prefilled.
///
/// The URI is parsed per RFC 6068: the path is the comma-separated
/// `To:` list; supported query params are `to`, `cc`, `bcc`, `subject`
/// and `body` (any other param is ignored). The parsed fields are
/// folded into a draft RFC 5322 skeleton via the built-in MIME
/// assembler, then piped on stdin to the composer (same contract as
/// `messages compose-with` / `reply-with` / `forward-with`). The
/// composer's stdout is routed through `--save` / `--send`, or to
/// stdout if neither is set.
#[derive(Debug, Parser)]
pub struct MessageMailtoCommand {
    /// `mailto:` URI as defined by RFC 6068.
    #[arg(value_name = "URI")]
    pub uri: String,

    /// Name of an entry in `[message.composer.*]`. Optional: when
    /// omitted, the composer flagged `default = true` is used.
    #[arg(value_name = "NAME", conflicts_with = "command")]
    pub name: Option<String>,

    /// Ad-hoc shell command, mutually exclusive with `<name>`.
    #[arg(long, value_name = "SHELL")]
    pub command: Option<String>,

    /// Save the produced message to the given mailbox.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    /// Submit the produced message through the account's send backend.
    #[arg(long)]
    pub send: bool,
}

impl MessageMailtoCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let fields = parse_mailto_uri(&self.uri)?;

        let to: Vec<String> = fields.to;
        let cc: Vec<String> = fields.cc;
        let bcc: Vec<String> = fields.bcc;

        let draft = builder::build(
            BuilderArgs {
                from: None,
                to: &to,
                cc: &cc,
                bcc: &bcc,
                subject: fields.subject.as_deref(),
                body: fields.body.as_deref(),
                body_file: None,
                attach: &[],
                signature: None,
                signature_file: None,
            },
            None,
        )?;

        let command = match self.command.as_deref() {
            Some(cmd) => cmd.to_owned(),
            None => {
                runner::resolve_composer(&client.account.composer, self.name.as_deref())?.to_owned()
            }
        };

        let raw = runner::run(&command, &draft)?;
        if raw.is_empty() {
            bail!("composer `{command}` produced no output");
        }

        output::route(printer, &mut client, raw, self.save.as_deref(), self.send)
    }
}

/// Fields extracted from a `mailto:` URI. Unrecognised query params
/// are silently ignored.
struct MailtoFields {
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: Option<String>,
    body: Option<String>,
}

/// Parses a `mailto:` URI per RFC 6068.
///
/// The path carries one or more comma-separated recipient addresses
/// (percent-decoded). The query string carries the headers `to`, `cc`,
/// `bcc`, `subject`, and `body`; addresses in `to` / `cc` / `bcc` may
/// themselves be comma-separated. Any other parameter is dropped.
fn parse_mailto_uri(uri: &str) -> Result<MailtoFields> {
    let url = Url::parse(uri).map_err(|err| anyhow!("invalid mailto URI `{uri}`: {err}"))?;
    if url.scheme() != "mailto" {
        bail!("expected `mailto:` URI, got scheme `{}`", url.scheme());
    }

    let mut to = split_addresses(url.path());
    let mut cc = Vec::new();
    let mut bcc = Vec::new();
    let mut subject = None;
    let mut body = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref().to_ascii_lowercase().as_str() {
            "to" => to.extend(split_addresses(value.as_ref())),
            "cc" => cc.extend(split_addresses(value.as_ref())),
            "bcc" => bcc.extend(split_addresses(value.as_ref())),
            "subject" => subject = Some(value.into_owned()),
            "body" => body = Some(value.into_owned()),
            _ => {}
        }
    }

    Ok(MailtoFields {
        to,
        cc,
        bcc,
        subject,
        body,
    })
}

/// Splits a comma-separated address list, percent-decodes each entry,
/// and drops the empties. Used both for the URI path and for the `to`
/// / `cc` / `bcc` query params (query values already come decoded
/// from `query_pairs`, but the comma split applies to both shapes).
fn split_addresses(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|part| {
            let decoded = percent_decode_str(part).decode_utf8_lossy().into_owned();
            let trimmed = decoded.trim().to_owned();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_recipient() {
        let f = parse_mailto_uri("mailto:bob@example.org").unwrap();
        assert_eq!(f.to, vec!["bob@example.org"]);
        assert!(f.cc.is_empty());
        assert!(f.bcc.is_empty());
        assert!(f.subject.is_none());
        assert!(f.body.is_none());
    }

    #[test]
    fn comma_separated_path() {
        let f = parse_mailto_uri("mailto:a@x.org,b@x.org").unwrap();
        assert_eq!(f.to, vec!["a@x.org", "b@x.org"]);
    }

    #[test]
    fn percent_decoded_path() {
        let f = parse_mailto_uri("mailto:bob%40example.org").unwrap();
        assert_eq!(f.to, vec!["bob@example.org"]);
    }

    #[test]
    fn subject_and_body_via_query() {
        let f = parse_mailto_uri(
            "mailto:bob@example.org?subject=Hello%20World&body=Hi%20Bob%2C%0AHow%20are%20you%3F",
        )
        .unwrap();
        assert_eq!(f.subject.as_deref(), Some("Hello World"));
        assert_eq!(f.body.as_deref(), Some("Hi Bob,\nHow are you?"));
    }

    #[test]
    fn cc_and_bcc_lists() {
        let f = parse_mailto_uri(
            "mailto:bob@example.org?cc=carol@example.org,dave@example.org&bcc=eve@example.org",
        )
        .unwrap();
        assert_eq!(f.cc, vec!["carol@example.org", "dave@example.org"]);
        assert_eq!(f.bcc, vec!["eve@example.org"]);
    }

    #[test]
    fn empty_path_with_to_param() {
        let f = parse_mailto_uri("mailto:?to=bob@example.org&subject=Hi").unwrap();
        assert_eq!(f.to, vec!["bob@example.org"]);
        assert_eq!(f.subject.as_deref(), Some("Hi"));
    }

    #[test]
    fn case_insensitive_query_keys() {
        let f = parse_mailto_uri("mailto:bob@example.org?Subject=Hi&BODY=Yo").unwrap();
        assert_eq!(f.subject.as_deref(), Some("Hi"));
        assert_eq!(f.body.as_deref(), Some("Yo"));
    }

    #[test]
    fn unknown_params_are_ignored() {
        let f = parse_mailto_uri("mailto:bob@example.org?foo=bar&subject=Hi").unwrap();
        assert_eq!(f.subject.as_deref(), Some("Hi"));
        assert!(f.body.is_none());
    }

    #[test]
    fn non_mailto_scheme_is_rejected() {
        assert!(parse_mailto_uri("https://example.org").is_err());
    }
}
