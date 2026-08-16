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
    /// Address the `From` header carries, from `--from` or from the
    /// account's `email`.
    pub from: Option<&'a str>,
    /// Name that address goes by, from the account's `display-name`.
    /// Kept apart from the address so `mail_builder` encodes it, a
    /// name holding a comma or a quote needing no rule of ours.
    pub from_name: Option<&'a str>,
    pub to: &'a [String],
    pub cc: &'a [String],
    pub bcc: &'a [String],
    pub subject: Option<&'a str>,
    pub body: Option<&'a str>,
    pub body_file: Option<&'a Path>,
    pub attach: &'a [PathBuf],
    /// Signature text, from `--signature` or from the account's
    /// `signature`. Left `None` when `signature_file` names the file
    /// holding it.
    pub signature: Option<&'a str>,
    pub signature_file: Option<&'a Path>,
    /// Separator written before the signature, from the account's
    /// `signature-delim`. Written verbatim.
    pub signature_delim: &'a str,
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
        builder = builder.from(Address::new_address(args.from_name, from));
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

        if source.mode == SourceMode::Reply
            && args.to.is_empty()
            && let Some(addrs) = reply_recipients(parsed)
        {
            builder = builder.to(addrs);
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
        args.signature_delim,
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
/// source text, an optional headline, an optional signature and the
/// separator introducing it, and the requested posting style.
fn compose_body(
    user_body: &str,
    source_text: &str,
    headline: &str,
    signature: &str,
    signature_delim: &str,
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

    // NOTE: the separator is written verbatim, its own trailing
    // newline included, so a delimiter meant to stand on its own line
    // says so rather than relying on a rule here.
    if !signature.trim().is_empty() {
        body.push_str("\n\n");
        body.push_str(signature_delim);
        body.push_str(signature.trim_end_matches('\n'));
    }

    body
}

fn has_prefix(subject: &str, prefix: &str) -> bool {
    let s = subject.trim_start();
    // Keep the colon: comparing only the letters would treat a subject
    // like "Ready to ship" as already carrying a "Re:" prefix and drop
    // the real one.
    let p = prefix.trim();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A raw source message used by the reply/forward tests. It already
    /// carries a `References` header so threading can be asserted.
    const SOURCE: &[u8] = b"From: Alice <alice@example.com>\r\n\
To: Bob <bob@example.com>\r\n\
Subject: Project update\r\n\
Message-ID: <orig-2@example.com>\r\n\
References: <orig-0@example.com> <orig-1@example.com>\r\n\
\r\n\
Original body line.\r\n";

    fn parse(raw: &[u8]) -> mail_parser::Message<'_> {
        MessageParser::new()
            .parse(raw)
            .expect("parse built message")
    }

    fn source(mode: SourceMode) -> SourceArgs<'static> {
        SourceArgs {
            raw: SOURCE,
            mode,
            posting_style: PostingStyle::Top,
            quote_headline: "On a day, Alice wrote:",
        }
    }

    fn args<'a>(
        from: &'a str,
        to: &'a [String],
        subject: Option<&'a str>,
        body: &'a str,
    ) -> BuilderArgs<'a> {
        BuilderArgs {
            from: Some(from),
            from_name: None,
            to,
            cc: &[],
            bcc: &[],
            subject,
            body: Some(body),
            body_file: None,
            attach: &[],
            signature: None,
            signature_file: None,
            signature_delim: "-- \n",
        }
    }

    #[test]
    fn compose_populates_headers_and_body() {
        let to = vec!["bob@example.com".to_string()];
        let cc = vec!["carol@example.com".to_string()];
        let mut a = args("alice@example.com", &to, Some("Hello"), "Hi Bob");
        a.cc = &cc;

        let raw = build(a, None).unwrap();
        let msg = parse(&raw);
        let text = String::from_utf8(raw.clone()).unwrap();

        assert_eq!(msg.subject(), Some("Hello"));
        assert!(msg.body_text(0).unwrap().contains("Hi Bob"));
        assert!(text.contains("alice@example.com"));
        assert!(text.contains("bob@example.com"));
        assert!(text.contains("carol@example.com"));
        assert!(!text.contains("In-Reply-To"));
    }

    #[test]
    fn compose_names_the_from_address() {
        let to = vec!["bob@example.com".to_string()];
        let mut a = args("alice@example.com", &to, Some("Hello"), "Hi Bob");
        // A comma is what an unquoted display name would break on, the
        // header reading as two addresses.
        a.from_name = Some("Doe, Alice");

        let raw = build(a, None).unwrap();
        let text = String::from_utf8(raw).unwrap();

        assert!(text.contains("From: \"Doe, Alice\" <alice@example.com>"));
    }

    #[test]
    fn reply_sets_subject_recipients_and_threading() {
        let empty: Vec<String> = Vec::new();
        let a = args("bob@example.com", &empty, None, "My reply");

        let raw = build(a, Some(source(SourceMode::Reply))).unwrap();
        let msg = parse(&raw);
        let text = String::from_utf8(raw.clone()).unwrap();

        // subject gains a single "Re:" prefix
        assert_eq!(msg.subject(), Some("Re: Project update"));
        // with no explicit --to, the reply goes to the source's From
        assert!(text.contains("alice@example.com"));
        // threading: In-Reply-To is the source id, References appends it
        assert!(text.contains("In-Reply-To:"));
        assert!(text.contains("orig-2@example.com"));
        // body: user text above the quoted source, with a headline
        let body = msg.body_text(0).unwrap();
        assert!(body.contains("My reply"));
        assert!(body.contains("On a day, Alice wrote:"));
        assert!(body.contains("> Original body line."));
    }

    #[test]
    fn reply_keeps_existing_re_prefix() {
        let raw = b"Subject: Re: Already replied\r\nMessage-ID: <x@e>\r\n\r\nbody";
        let src = SourceArgs {
            raw,
            mode: SourceMode::Reply,
            posting_style: PostingStyle::Top,
            quote_headline: "",
        };
        let empty: Vec<String> = Vec::new();
        let built = build(args("b@e", &empty, None, "r"), Some(src)).unwrap();
        assert_eq!(parse(&built).subject(), Some("Re: Already replied"));
    }

    #[test]
    fn reply_prefixes_subject_that_merely_starts_with_re_letters() {
        // regression: "Ready…" starts with "Re" but is not "Re:"-prefixed
        let raw = b"Subject: Ready to ship\r\nMessage-ID: <x@e>\r\n\r\nbody";
        let src = SourceArgs {
            raw,
            mode: SourceMode::Reply,
            posting_style: PostingStyle::Top,
            quote_headline: "",
        };
        let empty: Vec<String> = Vec::new();
        let built = build(args("b@e", &empty, None, "r"), Some(src)).unwrap();
        assert_eq!(parse(&built).subject(), Some("Re: Ready to ship"));
    }

    #[test]
    fn reply_explicit_to_overrides_source_from() {
        let to = vec!["dave@example.com".to_string()];
        let raw = build(
            args("bob@example.com", &to, None, "r"),
            Some(source(SourceMode::Reply)),
        )
        .unwrap();
        let text = String::from_utf8(raw).unwrap();
        assert!(text.contains("dave@example.com"));
        assert!(!text.contains("alice@example.com"));
    }

    #[test]
    fn forward_prefixes_subject_and_omits_in_reply_to() {
        let to = vec!["dave@example.com".to_string()];
        let raw = build(
            args("bob@example.com", &to, None, "FYI"),
            Some(source(SourceMode::Forward)),
        )
        .unwrap();
        let msg = parse(&raw);
        let text = String::from_utf8(raw.clone()).unwrap();

        assert_eq!(msg.subject(), Some("Fwd: Project update"));
        assert!(!text.contains("In-Reply-To"));
        assert!(msg.body_text(0).unwrap().contains("> Original body line."));
    }

    #[test]
    fn has_prefix_requires_the_colon_case_insensitively() {
        assert!(has_prefix("Re: x", "Re: "));
        assert!(has_prefix("re:x", "Re: "));
        assert!(has_prefix("RE: x", "Re: "));
        assert!(has_prefix("Fwd: x", "Fwd: "));
        assert!(!has_prefix("Ready to ship", "Re: "));
        assert!(!has_prefix("Review", "Re: "));
        assert!(!has_prefix("Forwarding note", "Fwd: "));
    }

    #[test]
    fn compute_references_appends_source_id() {
        // existing References win and the source id is appended
        let msg = parse(SOURCE);
        assert_eq!(
            compute_references(&msg, "orig-2@example.com"),
            "<orig-0@example.com> <orig-1@example.com> <orig-2@example.com>",
        );

        // falls back to In-Reply-To when there is no References header
        let raw = b"In-Reply-To: <a@e>\r\nMessage-ID: <b@e>\r\n\r\nx";
        let msg = parse(raw);
        assert_eq!(compute_references(&msg, "b@e"), "<a@e> <b@e>");

        // neither header: just the source id, wrapped
        let raw = b"Message-ID: <b@e>\r\n\r\nx";
        let msg = parse(raw);
        assert_eq!(compute_references(&msg, "b@e"), "<b@e>");
    }

    #[test]
    fn push_msg_id_wraps_and_separates() {
        let mut out = String::new();
        push_msg_id(&mut out, "a@e");
        push_msg_id(&mut out, "<b@e>");
        push_msg_id(&mut out, "  ");
        push_msg_id(&mut out, "c@e");
        assert_eq!(out, "<a@e> <b@e> <c@e>");
    }

    #[test]
    fn compose_body_honours_posting_style_and_signature() {
        let top = compose_body("mine", "theirs", "wrote:", "", "-- \n", PostingStyle::Top);
        assert_eq!(top, "mine\n\nwrote:\n> theirs");

        let bottom = compose_body(
            "mine",
            "theirs",
            "wrote:",
            "",
            "-- \n",
            PostingStyle::Bottom,
        );
        assert_eq!(bottom, "wrote:\n> theirs\n\nmine");

        let signed = compose_body("mine", "", "", "Alice", "-- \n", PostingStyle::Top);
        assert_eq!(signed, "mine\n\n-- \nAlice");
    }

    #[test]
    fn compose_body_writes_the_signature_delimiter_verbatim() {
        // A delimiter meant to stand on its own line carries its own
        // newline; one that does not, does not.
        let custom = compose_body("mine", "", "", "Alice", "~~~\n", PostingStyle::Top);
        assert_eq!(custom, "mine\n\n~~~\nAlice");

        let inline = compose_body("mine", "", "", "Alice", "", PostingStyle::Top);
        assert_eq!(inline, "mine\n\nAlice");
    }
}
