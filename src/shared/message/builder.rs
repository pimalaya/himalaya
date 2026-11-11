//! # Message builder
//!
//! The MIME assembly behind the `compose`, `reply` and `forward`
//! commands, which collapse into one set of fields once the source
//! message, if there is one, has been fetched.
//!
//! An RFC 5322 message comes out, headers derived from the source and the
//! whole assembled with mime-rs's strict builder, which writes the
//! `Date` and `Message-ID` given here and nothing it was not told.

use std::{
    io::{IsTerminal, Read as _, stdin},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use mime::{
    build::MimeBuildOptions,
    builder::{MimeAttachment, MimeBuilder},
    header::marker::{
        BCC, CC, DATE, FROM, IN_REPLY_TO, MESSAGE_ID, REFERENCES, REPLY_TO, SUBJECT, TO,
    },
    tree::{cst::MimeCst, entity::MimeEntity, field::address::parse_addresses},
    value::{
        address::{MimeAddress, MimeAddressList, MimeMailbox},
        content_type::MimeContentType,
        datetime::MimeDateTime,
        message_id::MimeMessageId,
    },
};
use pimalaya_cli::session::in_session;

use crate::{
    config::PostingStyle,
    shared::message::{header, part},
};

/// Everything the MIME assembler needs, which each command fills in from
/// its own clap struct.
pub struct BuilderArgs<'a> {
    /// Address the `From` header carries.
    ///
    /// A value spelling out a display name is split back apart rather
    /// than taken for one long address.
    pub from: Option<&'a str>,
    /// Name that address goes by, kept apart from it so the builder
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
    /// The attachments given inline, attached after the files.
    pub attach_data: Vec<InlineAttachment>,
    /// The signature, `None` when `signature_file` names the file holding
    /// it.
    pub signature: Option<&'a str>,
    /// The file the signature is read from instead.
    pub signature_file: Option<&'a Path>,
    /// Separator written before the signature, verbatim.
    pub signature_delim: &'a str,
}

/// An attachment given as its bytes rather than a path, which only the MCP
/// server hands a composer (`attach-data`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InlineAttachment {
    /// The file name the part carries.
    pub filename: String,
    /// The MIME type, guessed from the file name when `None`, as a path
    /// given to `--attach` is.
    pub mime: Option<String>,
    /// The decoded bytes.
    pub content: Vec<u8>,
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
    /// Whether the attachments of the source are left out of a
    /// forward, the quoting being all a reply ever took.
    pub no_attachments: bool,
    /// Whether a reply also goes to the other recipients of the source.
    pub reply_all: bool,
    /// The account's own address, left out of a reply to all.
    pub own_email: Option<&'a str>,
}

/// A mailbox: its display name, when it carries one, and its address.
type Mailbox = (Option<String>, String);

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
    let mut builder = MimeBuilder::default();
    let mut sender = None;

    if let Some(from) = args.from {
        let (parsed_name, address) = parse_mailboxes(from)?.remove(0);
        let name = args.from_name.map(str::to_owned).or(parsed_name);
        builder.header::<FROM>(address_list(&[(name, address.clone())]))?;
        sender = Some(address);
    }

    builder.header::<DATE>(now())?;
    builder.header::<MESSAGE_ID>(vec![new_message_id(sender.as_deref())])?;

    let parsed_source = source.as_ref().map(|source| MimeCst::parse(source.raw));

    let mut subject = args.subject.map(str::to_owned);
    let mut to = addresses(args.to)?;
    let mut cc = addresses(args.cc)?;
    let mut source_text = String::new();

    if let (Some(source), Some(parsed)) = (source.as_ref(), parsed_source.as_ref()) {
        let root = &parsed.root;
        let prefix = match source.mode {
            SourceMode::Reply => "Re: ",
            SourceMode::Forward => "Fwd: ",
        };
        let src_subject = header::unstructured(root, "Subject").unwrap_or_default();
        if subject.is_none() {
            subject = Some(if has_prefix(&src_subject, prefix) {
                src_subject
            } else {
                format!("{prefix}{src_subject}")
            });
        }

        if source.mode == SourceMode::Reply {
            let derived = match args.to.is_empty() {
                true => reply_recipients(root),
                false => Vec::new(),
            };
            if !derived.is_empty() {
                to = derived.clone();
            }

            if source.reply_all && args.cc.is_empty() {
                let mut to: Vec<String> = derived.into_iter().map(|(_, email)| email).collect();
                for value in args.to {
                    to.extend(parse_mailboxes(value)?.into_iter().map(|(_, email)| email));
                }

                cc = reply_all_cc(root, &to, source.own_email);
            }
        }

        if let Some(message_id) = root
            .header::<MESSAGE_ID>()
            .and_then(|ids| ids.into_iter().next())
        {
            if source.mode == SourceMode::Reply {
                builder.header::<IN_REPLY_TO>(vec![message_id.clone()])?;
            }
            builder.header::<REFERENCES>(compute_references(root, &message_id))?;
        }

        source_text = part::body_text(parsed);
    }

    if !to.is_empty() {
        builder.header::<TO>(address_list(&to))?;
    }
    if !cc.is_empty() {
        builder.header::<CC>(address_list(&cc))?;
    }
    if !args.bcc.is_empty() {
        builder.header::<BCC>(address_list(&addresses(args.bcc)?))?;
    }
    if let Some(subject) = subject {
        builder.header::<SUBJECT>(subject)?;
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
    builder.text(body);

    for path in args.attach {
        let bytes = std::fs::read(path)
            .map_err(|err| anyhow!("read attachment {}: {err}", path.display()))?;
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "attachment".to_string());
        builder.attachment(attachment(&mime_for(path), file_name, bytes))?;
    }

    for inline in args.attach_data {
        let mime = match inline.mime {
            Some(mime) => mime,
            None => mime_for(Path::new(&inline.filename)),
        };
        builder.attachment(attachment(&mime, inline.filename, inline.content))?;
    }

    if let (Some(source), Some(parsed)) = (source.as_ref(), parsed_source.as_ref())
        && source.mode == SourceMode::Forward
        && !source.no_attachments
    {
        carry_source_attachments(&mut builder, parsed)?;
    }

    builder
        .write(&MimeBuildOptions::default())
        .context("Cannot serialize the composed message")
}

/// An attachment of `mime` named `filename`, its bytes written as given.
fn attachment(mime: &str, filename: String, content: Vec<u8>) -> MimeAttachment {
    let (top, sub) = mime
        .split_once('/')
        .unwrap_or(("application", "octet-stream"));

    MimeAttachment {
        content_type: MimeContentType {
            top: top.to_owned(),
            sub: sub.to_owned(),
            params: Vec::new(),
        },
        filename: Some(filename),
        content,
        ..Default::default()
    }
}

/// Carries the attachments of the source into the forward, each under
/// its original or derived file name, leaving bodies and inline
/// resources to the quoting.
fn carry_source_attachments(builder: &mut MimeBuilder, parsed: &MimeCst) -> Result<()> {
    // NOTE: the leaves alone: a multipart container holds no bytes of its
    // own, and a `name` attribute on it would pass the attachment role
    // fallthrough.
    for leaf in part::leaves(parsed) {
        if part::role(leaf.part) != part::PartRole::Attachment {
            continue;
        }
        let mime = part::mime(leaf.part);
        let file_name =
            part::filename(leaf.part).unwrap_or_else(|| fallback_attachment_name(leaf.part, &mime));
        let bytes = part::bytes(leaf.part).into_owned();
        builder.attachment(attachment(&mime, file_name, bytes))?;
    }

    Ok(())
}

/// Names an attached email from its subject, or a file from its MIME type.
fn fallback_attachment_name(part: &MimeEntity, mime: &str) -> String {
    if mime == "message/rfc822" {
        let nested = part.nested();
        let inner = match (&part.body, &nested) {
            (mime::tree::body::MimeBodyNode::Message(inner), _) => Some(inner.as_ref()),
            (_, Some(nested)) => Some(&nested.root),
            _ => None,
        };
        let subject = inner
            .and_then(|inner| header::unstructured(inner, "Subject"))
            .unwrap_or_default();
        let subject: String = subject
            .chars()
            .map(|c| {
                if c.is_control()
                    || matches!(c, '/' | '\\' | ':' | '"' | '<' | '>' | '|' | '?' | '*')
                {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let subject = subject.trim_matches(|c: char| c == '.' || c.is_whitespace());
        let stem = if subject.is_empty() {
            "attachment"
        } else {
            subject
        };
        return format!("{stem}.eml");
    }

    let extension = if mime == "application/octet-stream" {
        Some("bin")
    } else {
        mime_guess::get_mime_extensions_str(mime).and_then(|exts| exts.first().copied())
    };
    match extension {
        Some(ext) => format!("attachment.{ext}"),
        None => "attachment".to_string(),
    }
}

/// The current time, at the local offset.
fn now() -> MimeDateTime {
    let now = chrono::Local::now();
    let offset = now.offset().local_minus_utc() / 60;
    MimeDateTime::from_unix_seconds(now.timestamp(), offset as i16)
}

/// The `Message-ID` of a new message: on the domain of the sender's
/// address, else on the machine's hostname, else on `localhost`.
fn new_message_id(sender: Option<&str>) -> String {
    sender
        .and_then(message_id)
        .or_else(|| {
            let host = gethostname::gethostname();
            MimeMessageId::new(&unique_id(), &host.to_string_lossy())
                .ok()
                .map(String::from)
        })
        .unwrap_or_else(|| format!("{}@localhost", unique_id()))
}

/// Generates a `Message-ID` on the domain of the sender's address.
///
/// The machine's hostname on the right of the `@` would tell every
/// recipient the name of the machine and is rarely a domain at all.
/// `None` when the address carries no plain domain, the hostname then
/// being the fallback.
fn message_id(address: &str) -> Option<String> {
    let (_, domain) = address.rsplit_once('@')?;

    // NOTE: RFC 5322 section 3.6.4 wants a dot-atom on the right, so a
    // domain literal or an internationalized domain is left out.
    let is_atext = |c: char| c.is_ascii_alphanumeric() || c == '-';
    if domain.is_empty()
        || !domain
            .split('.')
            .all(|l| !l.is_empty() && l.chars().all(is_atext))
    {
        return None;
    }

    MimeMessageId::new(&unique_id(), domain)
        .ok()
        .map(String::from)
}

/// The left part of a new `Message-ID`, unique on this machine: the time
/// in nanoseconds, the process id and a per-process counter.
fn unique_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{nanos:x}.{:x}.{count}", std::process::id())
}

/// Splits an address list into its mailboxes, each with its display
/// name, when it carries one, and its address.
///
/// Handing the builder the whole `Alice <alice@example.org>` as an
/// address would emit a `To: <Alice <alice@example.org>>` no SMTP
/// server accepts. A comma inside a quoted display name does not split
/// the list. The returned list is never empty.
fn parse_mailboxes(value: &str) -> Result<Vec<Mailbox>> {
    let mailboxes = header_mailboxes(&parse_addresses(value.as_bytes()));

    // NOTE: a bare word reads as an address with no domain, which no
    // transport delivers.
    if mailboxes.is_empty() || mailboxes.iter().any(|(_, email)| !email.contains('@')) {
        bail!("Could not parse address `{value}`");
    }

    Ok(mailboxes)
}

/// Every mailbox the values carry, in order.
fn addresses(values: &[String]) -> Result<Vec<Mailbox>> {
    let mut list = Vec::new();

    for value in values {
        list.extend(parse_mailboxes(value)?);
    }

    Ok(list)
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

    // NOTE: inside a session stdin carries the commands, so a body given
    // nowhere is empty, as it is at a terminal.
    if !stdin().is_terminal() && !in_session() {
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
        (PostingStyle::None, _) | (_, true) => user_body.to_string(),
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
fn reply_recipients(source: &MimeEntity) -> Vec<Mailbox> {
    source
        .header::<REPLY_TO>()
        .filter(|list| !list.is_empty())
        .or_else(|| source.header::<FROM>())
        .map(|list| header_mailboxes(&list))
        .unwrap_or_default()
}

/// Derives the carbon-copy recipients of a reply to all: the `To` and
/// `Cc` of the source, without the account's own address nor anyone
/// already in `to`, and without duplicates, compared ignoring case.
fn reply_all_cc(source: &MimeEntity, to: &[String], own: Option<&str>) -> Vec<Mailbox> {
    let mut seen: Vec<String> = to
        .iter()
        .map(String::as_str)
        .chain(own)
        .map(|email| email.trim().to_ascii_lowercase())
        .collect();
    let mut cc = Vec::new();

    for list in [source.header::<TO>(), source.header::<CC>()] {
        for (display, email) in list.map(|list| header_mailboxes(&list)).unwrap_or_default() {
            let key = email.trim().to_ascii_lowercase();
            if !seen.contains(&key) {
                seen.push(key);
                cc.push((display, email));
            }
        }
    }

    cc
}

/// Collects the mailboxes of an address list, groups flattened and
/// entries without an address dropped.
fn header_mailboxes(list: &MimeAddressList) -> Vec<Mailbox> {
    list.mailboxes()
        .filter_map(|mailbox| {
            let email = mailbox.address.clone().filter(|email| !email.is_empty())?;
            Some((mailbox.name.clone(), email))
        })
        .collect()
}

/// Turns mailboxes into the address list a header carries.
fn address_list(mailboxes: &[Mailbox]) -> MimeAddressList {
    MimeAddressList {
        items: mailboxes
            .iter()
            .map(|(name, email)| {
                MimeAddress::Mailbox(match name {
                    Some(name) => MimeMailbox::named(name.clone(), email.clone()),
                    None => MimeMailbox::address(email.clone()),
                })
            })
            .collect(),
    }
}

/// The `References` of a reply or forward: the source's own chain, or
/// its `In-Reply-To`, then the source id, each without angle brackets.
fn compute_references(source: &MimeEntity, source_message_id: &str) -> Vec<String> {
    let mut references = source
        .header::<REFERENCES>()
        .filter(|ids| !ids.is_empty())
        .or_else(|| source.header::<IN_REPLY_TO>())
        .unwrap_or_default();

    references.push(source_message_id.to_owned());
    references.retain(|id| !id.trim().is_empty());
    references
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

    /// A message read back, through the readers the tests assert on, its
    /// root header block reachable as an entity.
    struct Built(MimeCst<'static>);

    impl std::ops::Deref for Built {
        type Target = MimeEntity<'static>;

        fn deref(&self) -> &Self::Target {
            &self.0.root
        }
    }

    impl Built {
        fn subject(&self) -> Option<String> {
            header::unstructured(&self.0.root, "Subject")
        }

        fn body_text(&self) -> String {
            part::body_text(&self.0)
        }

        fn message_id(&self) -> Option<String> {
            self.0.root.header::<MESSAGE_ID>()?.into_iter().next()
        }

        fn mailboxes(&self, name: &str) -> Vec<Mailbox> {
            self.0
                .root
                .header
                .get(name)
                .map(|field| header_mailboxes(&parse_addresses(field.value())))
                .unwrap_or_default()
        }
    }

    fn parse(raw: &[u8]) -> Built {
        Built(MimeCst::parse(raw).into_owned())
    }

    fn source(mode: SourceMode) -> SourceArgs<'static> {
        SourceArgs {
            raw: SOURCE,
            mode,
            posting_style: PostingStyle::Top,
            quote_headline: "On a day, Alice wrote:",
            no_attachments: true,
            reply_all: false,
            own_email: None,
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
            attach_data: Vec::new(),
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

        assert_eq!(msg.subject().as_deref(), Some("Hello"));
        assert!(msg.body_text().contains("Hi Bob"));
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
    fn message_id_sits_on_the_sender_domain() {
        let to = vec!["bob@example.com".to_string()];
        let a = args("Alice <alice@example.com>", &to, Some("Hello"), "Hi Bob");

        let raw = build(a, None).unwrap();
        let msg = parse(&raw);
        let id = msg.message_id().expect("a message id");

        assert!(id.ends_with("@example.com"), "{id}");
        assert!(id.len() > "@example.com".len());
        assert_eq!(
            String::from_utf8(raw)
                .unwrap()
                .matches("Message-ID:")
                .count(),
            1
        );
    }

    #[test]
    fn message_id_needs_a_plain_domain() {
        assert!(message_id("alice@example.com").is_some());
        assert_ne!(
            message_id("alice@example.com"),
            message_id("alice@example.com")
        );
        assert_eq!(message_id("alice"), None);
        assert_eq!(message_id("alice@"), None);
        assert_eq!(message_id("alice@[192.0.2.1]"), None);
        assert_eq!(message_id("alice@exa mple.com"), None);
        assert_eq!(message_id("alice@b\u{fc}cher.example"), None);
    }

    #[test]
    fn reply_sets_subject_recipients_and_threading() {
        let empty: Vec<String> = Vec::new();
        let a = args("bob@example.com", &empty, None, "My reply");

        let raw = build(a, Some(source(SourceMode::Reply))).unwrap();
        let msg = parse(&raw);
        let text = String::from_utf8(raw.clone()).unwrap();

        assert_eq!(msg.subject().as_deref(), Some("Re: Project update"));
        assert!(text.contains("alice@example.com"));
        assert!(text.contains("In-Reply-To:"));
        assert!(text.contains("orig-2@example.com"));
        let body = msg.body_text();
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
            no_attachments: true,
            reply_all: false,
            own_email: None,
        };
        let empty: Vec<String> = Vec::new();
        let built = build(args("b@e", &empty, None, "r"), Some(src)).unwrap();
        assert_eq!(
            parse(&built).subject().as_deref(),
            Some("Re: Already replied")
        );
    }

    #[test]
    fn reply_prefixes_subject_that_merely_starts_with_re_letters() {
        let raw = b"Subject: Ready to ship\r\nMessage-ID: <x@e>\r\n\r\nbody";
        let src = SourceArgs {
            raw,
            mode: SourceMode::Reply,
            posting_style: PostingStyle::Top,
            quote_headline: "",
            no_attachments: true,
            reply_all: false,
            own_email: None,
        };
        let empty: Vec<String> = Vec::new();
        let built = build(args("b@e", &empty, None, "r"), Some(src)).unwrap();
        assert_eq!(
            parse(&built).subject().as_deref(),
            Some("Re: Ready to ship")
        );
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

    /// A source with several `To` and `Cc` recipients, the account's own
    /// address and a repeat among them, and a `Reply-To` of its own.
    const GROUP_SOURCE: &[u8] = b"From: Alice <alice@example.com>\r\n\
Reply-To: Lists <lists@example.com>\r\n\
To: Bob <bob@example.com>, \"Carol, C\" <CAROL@example.com>, me@example.com\r\n\
Cc: bob@example.com, Dave <dave@example.com>, Me <ME@Example.com>\r\n\
Subject: Plan\r\n\
Message-ID: <plan@example.com>\r\n\
\r\n\
Body.\r\n";

    /// Builds a reply to [`GROUP_SOURCE`] and returns its `To` and `Cc`
    /// addresses.
    fn reply_lists(reply_all: bool, to: &[String], cc: &[String]) -> (Vec<String>, Vec<String>) {
        let mut a = args("me@example.com", to, None, "r");
        a.cc = cc;
        let source = SourceArgs {
            raw: GROUP_SOURCE,
            mode: SourceMode::Reply,
            posting_style: PostingStyle::Top,
            quote_headline: "",
            no_attachments: true,
            reply_all,
            own_email: Some("me@example.com"),
        };

        let raw = build(a, Some(source)).unwrap();
        let msg = parse(&raw);
        let emails = |name: &str| {
            msg.mailboxes(name)
                .into_iter()
                .map(|(_, email)| email)
                .collect()
        };

        (emails("To"), emails("Cc"))
    }

    #[test]
    fn reply_without_all_leaves_cc_empty() {
        let (to, cc) = reply_lists(false, &[], &[]);
        assert_eq!(to, ["lists@example.com"]);
        assert!(cc.is_empty());
    }

    #[test]
    fn reply_all_copies_the_other_recipients_once() {
        let (to, cc) = reply_lists(true, &[], &[]);
        assert_eq!(to, ["lists@example.com"]);
        assert_eq!(
            cc,
            ["bob@example.com", "CAROL@example.com", "dave@example.com"]
        );
    }

    #[test]
    fn reply_all_keeps_the_display_names() {
        let raw = build(
            args("me@example.com", &[], None, "r"),
            Some(SourceArgs {
                raw: GROUP_SOURCE,
                mode: SourceMode::Reply,
                posting_style: PostingStyle::Top,
                quote_headline: "",
                no_attachments: true,
                reply_all: true,
                own_email: Some("me@example.com"),
            }),
        )
        .unwrap();
        let text = String::from_utf8(raw).unwrap();

        assert!(text.contains("\"Carol, C\" <CAROL@example.com>"), "{text}");
    }

    #[test]
    fn reply_all_leaves_out_who_is_already_in_to() {
        let msg = parse(GROUP_SOURCE);
        let to = vec!["bob@example.com".to_string()];

        let cc: Vec<_> = reply_all_cc(&msg, &to, Some("me@example.com"))
            .into_iter()
            .map(|(_, email)| email)
            .collect();

        assert_eq!(cc, ["CAROL@example.com", "dave@example.com"]);
    }

    #[test]
    fn reply_all_with_explicit_to_dedupes_against_it() {
        let to = vec!["Dave <dave@example.com>".to_string()];
        let (to, cc) = reply_lists(true, &to, &[]);
        assert_eq!(to, ["dave@example.com"]);
        assert_eq!(cc, ["bob@example.com", "CAROL@example.com"]);
    }

    #[test]
    fn reply_all_explicit_cc_wins() {
        let cc = vec!["erin@example.com".to_string()];
        let (to, cc) = reply_lists(true, &[], &cc);
        assert_eq!(to, ["lists@example.com"]);
        assert_eq!(cc, ["erin@example.com"]);
    }

    #[test]
    fn reply_all_without_own_email_keeps_every_recipient() {
        let msg = parse(GROUP_SOURCE);
        let cc = reply_all_cc(&msg, &[], None);
        assert_eq!(cc.len(), 4);
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

        assert_eq!(msg.subject().as_deref(), Some("Fwd: Project update"));
        assert!(!text.contains("In-Reply-To"));
        assert!(msg.body_text().contains("> Original body line."));
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
            [
                "orig-0@example.com",
                "orig-1@example.com",
                "orig-2@example.com"
            ],
        );

        let raw = b"In-Reply-To: <a@e>\r\nMessage-ID: <b@e>\r\n\r\nx";
        let msg = parse(raw);
        assert_eq!(compute_references(&msg, "b@e"), ["a@e", "b@e"]);

        let raw = b"Message-ID: <b@e>\r\n\r\nx";
        let msg = parse(raw);
        assert_eq!(compute_references(&msg, "b@e"), ["b@e"]);
    }

    #[test]
    fn parse_mailboxes_splits_name_and_address() {
        let alice =
            |name: Option<&str>| vec![(name.map(str::to_owned), "alice@example.org".into())];

        let mailboxes = parse_mailboxes("Alice <alice@example.org>").unwrap();
        assert_eq!(mailboxes, alice(Some("Alice")));

        let mailboxes = parse_mailboxes("alice@example.org").unwrap();
        assert_eq!(mailboxes, alice(None));

        let mailboxes = parse_mailboxes("\"Doe, Alice\" <alice@example.org>").unwrap();
        assert_eq!(mailboxes, alice(Some("Doe, Alice")));

        let mailboxes = parse_mailboxes("=?utf-8?B?QWxpY2U=?= <alice@example.org>").unwrap();
        assert_eq!(mailboxes, alice(Some("Alice")));

        let mailboxes = parse_mailboxes("alice@example.org, Bob <bob@example.org>").unwrap();
        assert_eq!(mailboxes[0], (None, "alice@example.org".into()));
        assert_eq!(mailboxes[1], (Some("Bob".into()), "bob@example.org".into()));

        assert!(parse_mailboxes("not-an-address").is_err());
        assert!(parse_mailboxes("").is_err());
    }

    #[test]
    fn compose_from_keeps_a_spelled_out_display_name_apart() {
        let empty: Vec<String> = Vec::new();
        let raw = build(args("Alice <alice@example.org>", &empty, None, "hi"), None).unwrap();
        let text = String::from_utf8(raw.clone()).unwrap();

        assert!(!text.contains("<Alice <alice@example.org>>"));

        let msg = parse(&raw);
        let from = msg.mailboxes("From");
        assert_eq!(from, [(Some("Alice".into()), "alice@example.org".into())]);
    }

    #[test]
    fn compose_to_keeps_a_spelled_out_display_name_apart() {
        let to = vec!["Alice <alice@example.org>".to_string()];
        let raw = build(args("sender@example.org", &to, None, "hi"), None).unwrap();
        let text = String::from_utf8(raw.clone()).unwrap();

        // regression: the whole value used to land inside the brackets
        assert!(!text.contains("<Alice <alice@example.org>>"));

        let msg = parse(&raw);
        let to = msg.mailboxes("To");
        assert_eq!(to, [(Some("Alice".into()), "alice@example.org".into())]);
    }

    #[test]
    fn compose_to_keeps_every_mailbox_of_every_value() {
        let to = vec![
            "\"Doe, Alice\" <alice@example.org>, bob@example.org".to_string(),
            "carol@example.org".to_string(),
        ];
        let raw = build(args("sender@example.org", &to, None, "hi"), None).unwrap();

        let msg = parse(&raw);
        let to = msg.mailboxes("To");
        assert_eq!(
            to,
            [
                (Some("Doe, Alice".into()), "alice@example.org".into()),
                (None, "bob@example.org".into()),
                (None, "carol@example.org".into()),
            ],
        );
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

        let unquoted = compose_body("mine", "theirs", "wrote:", "", "-- \n", PostingStyle::None);
        assert_eq!(unquoted, "mine");

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
