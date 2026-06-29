//! Shared MIME-building helpers for the built-in `compose`, `reply`
//! and `forward` subcommands.
//!
//! Each subcommand has its own clap struct (different positional /
//! optional args), but they all collapse into the same set of fields
//! once the source message — if any — is fetched. The helpers here
//! accept those fields and assemble an RFC 5322 message with
//! `mail_builder` (plus reply/forward header derivation via
//! `mail_parser`).
//!
//! The `-with` subcommands delegate composition entirely to an
//! external command and never go through this module.

use std::{
    io::{IsTerminal, Read as _, stdin},
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use clap::ValueEnum;
use mail_builder::{
    MessageBuilder,
    headers::{address::Address, raw::Raw},
};
use mail_parser::{HeaderValue, MessageParser};

/// How a quoted source body is laid out relative to the user's body
/// when replying or forwarding.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum PostingStyle {
    /// User body above the quoted source body.
    Top,
    /// Quoted source body above the user body.
    Bottom,
}

/// All the fields the built-in MIME assembler needs. Each subcommand
/// populates these from its own clap struct.
pub struct BuilderArgs<'a> {
    pub from: Option<&'a str>,
    pub to: &'a [String],
    pub cc: &'a [String],
    pub bcc: &'a [String],
    pub subject: Option<&'a str>,
    pub body: Option<&'a str>,
    pub body_file: Option<&'a Path>,
    pub attach: &'a [PathBuf],
    pub signature: Option<&'a str>,
    pub signature_file: Option<&'a Path>,
}

/// Source-message metadata, populated for reply/forward subcommands.
pub struct SourceArgs<'a> {
    pub raw: &'a [u8],
    pub mode: SourceMode,
    pub posting_style: PostingStyle,
    pub quote_headline: &'a str,
}

/// Whether the source message is being replied to or forwarded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMode {
    Reply,
    Forward,
}

/// Assembles a MIME message from `args` and an optional reply/forward
/// `source`. Returns the raw RFC 5322 bytes.
pub fn build(args: BuilderArgs<'_>, source: Option<SourceArgs<'_>>) -> Result<Vec<u8>> {
    let mut builder = MessageBuilder::new();

    if let Some(from) = args.from {
        builder = builder.from(from);
    }
    if !args.to.is_empty() {
        builder = builder.to(addresses(args.to));
    }
    if !args.cc.is_empty() {
        builder = builder.cc(addresses(args.cc));
    }
    if !args.bcc.is_empty() {
        builder = builder.bcc(addresses(args.bcc));
    }

    let parsed_source = source
        .as_ref()
        .and_then(|s| MessageParser::new().parse(s.raw));

    let mut subject = args.subject.map(str::to_owned);
    let mut source_text = String::new();

    if let (Some(source), Some(parsed)) = (source.as_ref(), parsed_source.as_ref()) {
        let prefix = match source.mode {
            SourceMode::Reply => "Re: ",
            SourceMode::Forward => "Fwd: ",
        };
        let src_subject = parsed.subject().unwrap_or("");
        if subject.is_none() {
            subject = Some(if has_prefix(src_subject, prefix) {
                src_subject.to_string()
            } else {
                format!("{prefix}{src_subject}")
            });
        }

        if source.mode == SourceMode::Reply && args.to.is_empty() {
            if let Some(addrs) = reply_recipients(parsed) {
                builder = builder.to(addrs);
            }
        }

        if let Some(message_id) = parsed.message_id() {
            if source.mode == SourceMode::Reply {
                builder = builder.in_reply_to(vec![message_id.to_string()]);
            }
            let refs = compute_references(parsed, message_id);
            if !refs.is_empty() {
                builder = builder.header("References", Raw::new(refs));
            }
        }

        source_text = parsed
            .body_text(0)
            .map(|c| c.into_owned())
            .unwrap_or_default();
    }

    if let Some(s) = subject {
        builder = builder.subject(s);
    }

    let user_body = read_body(args.body, args.body_file)?;
    let signature = read_signature(args.signature, args.signature_file)?;
    let (style, headline) = match source.as_ref() {
        Some(s) => (s.posting_style, s.quote_headline),
        None => (PostingStyle::Top, ""),
    };
    let body = compose_body(
        &user_body,
        &source_text,
        headline,
        signature.as_deref().unwrap_or(""),
        style,
    );
    builder = builder.text_body(body);

    for path in args.attach {
        let bytes = std::fs::read(path)
            .map_err(|err| anyhow!("read attachment {}: {err}", path.display()))?;
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "attachment".to_string());
        let mime = mime_for(path);
        builder = builder.attachment(mime, file_name, bytes);
    }

    builder
        .write_to_vec()
        .map_err(|err| anyhow!("serialize composed message: {err}"))
}

fn addresses(values: &[String]) -> Address<'static> {
    Address::new_list(
        values
            .iter()
            .map(|s| Address::new_address(None::<&str>, s.clone()))
            .collect(),
    )
}

fn read_body(body: Option<&str>, body_file: Option<&Path>) -> Result<String> {
    if let Some(body) = body {
        return Ok(body.to_owned());
    }

    if let Some(path) = body_file {
        return std::fs::read_to_string(path)
            .map_err(|err| anyhow!("read body file {}: {err}", path.display()));
    }

    if !stdin().is_terminal() {
        let mut buf = String::new();
        stdin().read_to_string(&mut buf)?;
        return Ok(buf);
    }

    Ok(String::new())
}

fn read_signature(
    signature: Option<&str>,
    signature_file: Option<&Path>,
) -> Result<Option<String>> {
    if let Some(sig) = signature {
        return Ok(Some(sig.to_owned()));
    }

    if let Some(path) = signature_file {
        let s = std::fs::read_to_string(path)
            .map_err(|err| anyhow!("read signature file {}: {err}", path.display()))?;
        return Ok(Some(s));
    }

    Ok(None)
}

/// Builds the final text body from user input, optional quoted
/// source text, an optional headline, an optional signature, and the
/// requested posting style.
fn compose_body(
    user_body: &str,
    source_text: &str,
    headline: &str,
    signature: &str,
    style: PostingStyle,
) -> String {
    let user_body = user_body.trim_end_matches('\n');
    let source_text = source_text.trim();

    let quote = if source_text.is_empty() {
        String::new()
    } else {
        let mut buf = String::new();
        if !headline.is_empty() {
            buf.push_str(headline.trim_end_matches('\n'));
            buf.push('\n');
        }
        for line in source_text.lines() {
            buf.push('>');
            if !line.starts_with('>') {
                buf.push(' ');
            }
            buf.push_str(line);
            buf.push('\n');
        }
        buf.pop();
        buf
    };

    let mut body = match (style, quote.is_empty()) {
        (_, true) => user_body.to_string(),
        (PostingStyle::Top, false) => {
            if user_body.is_empty() {
                quote
            } else {
                format!("{user_body}\n\n{quote}")
            }
        }
        (PostingStyle::Bottom, false) => {
            if user_body.is_empty() {
                quote
            } else {
                format!("{quote}\n\n{user_body}")
            }
        }
    };

    if !signature.trim().is_empty() {
        let sig = signature.trim_end_matches('\n');
        body.push_str("\n\n-- \n");
        body.push_str(sig);
    }

    body
}

fn has_prefix(subject: &str, prefix: &str) -> bool {
    let s = subject.trim_start();
    let p = prefix.trim_end_matches(' ').trim_end_matches(':');
    s.len() >= p.len() && s.get(..p.len()).map(|h| h.eq_ignore_ascii_case(p)) == Some(true)
}

fn reply_recipients(msg: &mail_parser::Message<'_>) -> Option<Address<'static>> {
    use mail_parser::Address as ParserAddress;

    let header = msg
        .header("Reply-To")
        .or_else(|| msg.header("From"))
        .cloned();

    let HeaderValue::Address(addr) = header? else {
        return None;
    };

    let collected: Vec<Address<'static>> = match addr {
        ParserAddress::List(list) => list
            .into_iter()
            .filter_map(|a| {
                let email = a.address?.into_owned();
                let name = a.name.map(|s| s.into_owned());
                Some(Address::new_address(name, email))
            })
            .collect(),
        ParserAddress::Group(groups) => groups
            .into_iter()
            .flat_map(|g| g.addresses.into_iter())
            .filter_map(|a| {
                let email = a.address?.into_owned();
                let name = a.name.map(|s| s.into_owned());
                Some(Address::new_address(name, email))
            })
            .collect(),
    };

    if collected.is_empty() {
        None
    } else {
        Some(Address::new_list(collected))
    }
}

fn compute_references(msg: &mail_parser::Message<'_>, source_message_id: &str) -> String {
    let mut out = String::new();

    if let Some(header) = msg.header("References") {
        if let HeaderValue::TextList(items) = header {
            for r in items {
                push_msg_id(&mut out, r);
            }
        } else if let HeaderValue::Text(s) = header {
            for r in s.split_whitespace() {
                push_msg_id(&mut out, r);
            }
        }
    } else if let Some(header) = msg.header("In-Reply-To") {
        if let HeaderValue::TextList(items) = header {
            for r in items {
                push_msg_id(&mut out, r);
            }
        } else if let HeaderValue::Text(s) = header {
            for r in s.split_whitespace() {
                push_msg_id(&mut out, r);
            }
        }
    }

    push_msg_id(&mut out, source_message_id);
    out
}

fn push_msg_id(out: &mut String, id: &str) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    if !out.is_empty() {
        out.push(' ');
    }
    if id.starts_with('<') {
        out.push_str(id);
    } else {
        out.push('<');
        out.push_str(id);
        out.push('>');
    }
}

fn mime_for(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_owned()
}
