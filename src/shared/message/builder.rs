//! # Message builder
//!
//! The MIME assembly behind the `compose`, `reply` and `forward`
//! commands, which collapse into one set of fields once the source
//! message, if there is one, has been fetched.
//!
//! An RFC 5322 message comes out, headers derived from the source with
//! mail-parser and the whole assembled with mail-builder.

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
use mail_parser::{HeaderValue, MessageParser, parsers::MessageStream};

/// Where a quoted source body sits relative to the written one.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum PostingStyle {
    /// The written body above the quoted source body.
    Top,
    /// The quoted source body above the written body.
    Bottom,
}

/// Everything the MIME assembler needs, which each command fills in from
/// its own clap struct.
pub struct BuilderArgs<'a> {
    /// Address the `From` header carries.
    ///
    /// A value spelling out a display name is split back apart rather
    /// than taken for one long address.
    pub from: Option<&'a str>,
    /// Name that address goes by, kept apart from it so mail-builder
    /// encodes it: a name holding a comma or a quote wants no rule here.
    pub from_name: Option<&'a str>,
    /// Addresses the `To` header carries.
    pub to: &'a [String],
    /// Addresses the `Cc` header carries.
    pub cc: &'a [String],
    /// Addresses the `Bcc` header carries.
    pub bcc: &'a [String],
    /// The `Subject` header.
    pub subject: Option<&'a str>,
    /// The text body, when it was given inline.
    pub body: Option<&'a str>,
    /// The file the text body is read from instead.
    pub body_file: Option<&'a Path>,
    /// The files to attach.
    pub attach: &'a [PathBuf],
    /// The signature, `None` when `signature_file` names the file holding
    /// it.
    pub signature: Option<&'a str>,
    /// The file the signature is read from instead.
    pub signature_file: Option<&'a Path>,
    /// Separator written before the signature, verbatim.
    pub signature_delim: &'a str,
}

/// The source message a reply or a forward is derived from.
pub struct SourceArgs<'a> {
    /// The raw RFC 5322 bytes of the source.
    pub raw: &'a [u8],
    /// Whether it is being replied to or forwarded.
    pub mode: SourceMode,
    /// Where its quoted body sits relative to the written one.
    pub posting_style: PostingStyle,
    /// The headline placed before its quoted body.
    pub quote_headline: &'a str,
}

/// Whether the source message is being replied to or forwarded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMode {
    /// The reply derives `In-Reply-To`, `References` and a `Re:` subject.
    Reply,
    /// The forward derives `References` and a `Fwd:` subject.
    Forward,
}

/// Assembles the raw RFC 5322 bytes of a message, deriving the reply or
/// forward headers when a source is given.
pub fn build(args: BuilderArgs<'_>, source: Option<SourceArgs<'_>>) -> Result<Vec<u8>> {
    let mut builder = MessageBuilder::new();

    if let Some(from) = args.from {
        let (parsed_name, address) = parse_mailbox(from)?;
        let name = args.from_name.map(str::to_owned).or(parsed_name);
        builder = builder.from(Address::new_address(name, address));
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

/// Splits a mailbox into its display name, when it carries one, and its
/// address.
///
/// Handing mail-builder the whole `Alice <alice@example.org>` as an
/// address would emit a `From: <Alice <alice@example.org>>` no SMTP
/// server accepts.
fn parse_mailbox(value: &str) -> Result<(Option<String>, String)> {
    use mail_parser::Address as ParserAddress;

    // NOTE: the header parser flushes its last token on the
    // terminating newline, which a bare address, unlike one closed by
    // a `>`, does not otherwise carry.
    let header = format!("{value}\n");
    let parsed = MessageStream::new(header.as_bytes()).parse_address();

    let mailbox = match &parsed {
        HeaderValue::Address(ParserAddress::List(list)) => list.first(),
        HeaderValue::Address(ParserAddress::Group(groups)) => {
            groups.first().and_then(|group| group.addresses.first())
        }
        _ => None,
    }
    .ok_or_else(|| anyhow!("Could not parse address `{value}`"))?;

    let address = mailbox
        .address
        .as_ref()
        .ok_or_else(|| anyhow!("Address `{value}` has no email"))?
        .to_string();
    let name = mailbox.name.as_ref().map(|name| name.to_string());

    Ok((name, address))
}

/// Builds an address list out of bare addresses.
fn addresses(values: &[String]) -> Address<'static> {
    Address::new_list(
        values
            .iter()
            .map(|s| Address::new_address(None::<&str>, s.clone()))
            .collect(),
    )
}

/// Reads the text body from the flag, the file it names, or piped
/// standard input.
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

/// Reads the signature from the flag or the file it names.
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

/// Lays out the final text body: the written text, the quoted source
/// under its headline, and the signature after its separator.
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

/// Whether a subject already carries a `Re:` or `Fwd:` prefix.
fn has_prefix(subject: &str, prefix: &str) -> bool {
    let s = subject.trim_start();
    // NOTE: the colon is part of the comparison, letters alone reading
    // "Ready to ship" as already `Re:`-prefixed.
    let p = prefix.trim();
    s.len() >= p.len() && s.get(..p.len()).map(|h| h.eq_ignore_ascii_case(p)) == Some(true)
}

/// Derives the recipients of a reply from the source's `Reply-To`, or
/// from its `From` when it names none.
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

/// Builds the `References` header of a reply: the source's own chain,
/// or its `In-Reply-To`, with the source id appended.
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

/// Appends one message id to a `References` chain, wrapped in angle
/// brackets.
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

/// Guesses the MIME type of an attachment from its path.
fn mime_for(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A raw source message for the reply and forward tests, carrying a
    /// `References` header so threading can be asserted.
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
        // NOTE: a comma is what an unquoted display name breaks on, the
        // header then reading as two addresses.
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

        assert_eq!(msg.subject(), Some("Re: Project update"));
        assert!(text.contains("alice@example.com"));
        assert!(text.contains("In-Reply-To:"));
        assert!(text.contains("orig-2@example.com"));
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
        let msg = parse(SOURCE);
        assert_eq!(
            compute_references(&msg, "orig-2@example.com"),
            "<orig-0@example.com> <orig-1@example.com> <orig-2@example.com>",
        );

        let raw = b"In-Reply-To: <a@e>\r\nMessage-ID: <b@e>\r\n\r\nx";
        let msg = parse(raw);
        assert_eq!(compute_references(&msg, "b@e"), "<a@e> <b@e>");

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
    fn parse_mailbox_splits_name_and_address() {
        let (name, address) = parse_mailbox("Alice <alice@example.org>").unwrap();
        assert_eq!(name.as_deref(), Some("Alice"));
        assert_eq!(address, "alice@example.org");

        let (name, address) = parse_mailbox("alice@example.org").unwrap();
        assert_eq!(name, None);
        assert_eq!(address, "alice@example.org");

        let (name, address) = parse_mailbox("\"Doe, Alice\" <alice@example.org>").unwrap();
        assert_eq!(name.as_deref(), Some("Doe, Alice"));
        assert_eq!(address, "alice@example.org");

        let (name, _) = parse_mailbox("=?utf-8?B?QWxpY2U=?= <alice@example.org>").unwrap();
        assert_eq!(name.as_deref(), Some("Alice"));

        assert!(parse_mailbox("not-an-address").is_err());
        assert!(parse_mailbox("").is_err());
    }

    #[test]
    fn compose_from_keeps_a_spelled_out_display_name_apart() {
        let empty: Vec<String> = Vec::new();
        let raw = build(args("Alice <alice@example.org>", &empty, None, "hi"), None).unwrap();
        let text = String::from_utf8(raw.clone()).unwrap();

        assert!(!text.contains("<Alice <alice@example.org>>"));

        let msg = parse(&raw);
        let from = msg.from().unwrap().first().unwrap();
        assert_eq!(from.name(), Some("Alice"));
        assert_eq!(from.address(), Some("alice@example.org"));
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
        // NOTE: a delimiter meant to stand on its own line carries its own
        // newline; one that does not, does not.
        let custom = compose_body("mine", "", "", "Alice", "~~~\n", PostingStyle::Top);
        assert_eq!(custom, "mine\n\n~~~\nAlice");

        let inline = compose_body("mine", "", "", "Alice", "", PostingStyle::Top);
        assert_eq!(inline, "mine\n\nAlice");
    }
}
