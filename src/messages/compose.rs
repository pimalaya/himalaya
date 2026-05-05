use std::{
    io::{stdin, stdout, IsTerminal, Read as _, Write as _},
    path::PathBuf,
};

use anyhow::{anyhow, bail, Result};
use clap::{Parser, ValueEnum};
#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
use mail_builder::headers::raw::Raw;
use mail_builder::{headers::address::Address, MessageBuilder};
#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
use mail_parser::{HeaderValue, MessageParser};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
};

/// Compose a message from CLI arguments.
///
/// By default the assembled RFC 5322 bytes are written to stdout.
/// Pass `--send` to route the message through the active account's
/// send path (SMTP or JMAP). `--reply <id>` and `--forward <id>` are
/// mutually exclusive: each fetches the referenced source message,
/// pre-fills the relevant headers (Subject prefix, In-Reply-To,
/// References, and the recipient when replying), and includes the
/// quoted source body according to `--posting-style`. For richer
/// composition (multipart MIME, MML directives, etc.) build the
/// message externally and pipe it through `messages send`.
#[derive(Debug, Parser)]
pub struct MessagesComposeCommand {
    /// Sender address (`From` header). Plain `local@host` form.
    #[arg(long, value_name = "ADDR")]
    pub from: Option<String>,

    /// Recipient address(es) (`To` header). Repeat the flag or use a
    /// comma-separated list.
    #[arg(long, short = 't', value_name = "ADDR", value_delimiter = ',')]
    pub to: Vec<String>,

    /// Carbon-copy recipient(s) (`Cc` header).
    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub cc: Vec<String>,

    /// Blind carbon-copy recipient(s) (`Bcc` header).
    #[arg(long, value_name = "ADDR", value_delimiter = ',')]
    pub bcc: Vec<String>,

    /// Subject line.
    #[arg(long, short = 's', value_name = "TEXT")]
    pub subject: Option<String>,

    /// Inline body. Mutually exclusive with `--body-file` and stdin.
    #[arg(long, value_name = "TEXT", conflicts_with = "body_file")]
    pub body: Option<String>,

    /// Read the body from a file. Mutually exclusive with `--body`
    /// and stdin.
    #[arg(long = "body-file", value_name = "PATH")]
    pub body_file: Option<PathBuf>,

    /// Attachment file(s).
    #[arg(long = "attach", value_name = "PATH")]
    pub attach: Vec<PathBuf>,

    /// Reply to the message with this id. Pre-fills `To`, `Subject`,
    /// `In-Reply-To` and `References` from the source. Mutually
    /// exclusive with `--forward`.
    #[arg(long, value_name = "ID", conflicts_with = "forward")]
    pub reply: Option<String>,

    /// Forward the message with this id. Pre-fills the `Subject`
    /// prefix and `References` header from the source. Mutually
    /// exclusive with `--reply`.
    #[arg(long, value_name = "ID")]
    pub forward: Option<String>,

    /// Mailbox the source message lives in (only relevant for
    /// `--reply`/`--forward`). Ignored for JMAP, which addresses
    /// messages by id directly.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    /// How to lay out the quoted source body relative to the user's
    /// body. Only meaningful with `--reply` / `--forward`. Interleaved
    /// posting is left to the user — write your reply directly inside
    /// the quoted block.
    #[arg(
        long = "posting-style",
        short = 'P',
        value_name = "STYLE",
        default_value = "top"
    )]
    pub posting_style: PostingStyle,

    /// Plain-text headline placed before the quoted source body
    /// (e.g. `"On {date}, {from} wrote:"`). Empty by default — no
    /// substitution is performed; pass the literal string you want.
    #[arg(long = "quote-headline", short = 'Q', value_name = "TEXT")]
    pub quote_headline: Option<String>,

    /// Signature appended after the body, separated by the standard
    /// `-- ` delimiter (RFC 3676 §4.3).
    #[arg(long, value_name = "TEXT")]
    pub signature: Option<String>,

    /// Read the signature from a file. Mutually exclusive with
    /// `--signature`.
    #[arg(
        long = "signature-file",
        value_name = "PATH",
        conflicts_with = "signature"
    )]
    pub signature_file: Option<PathBuf>,

    /// Send the assembled message instead of writing it to stdout.
    /// Routes through the same SMTP/JMAP path as `messages send`.
    #[arg(long)]
    pub send: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum PostingStyle {
    /// User body above the quoted source body.
    Top,
    /// Quoted source body above the user body.
    Bottom,
}

impl MessagesComposeCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        #[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
        let source_raw = match (&self.reply, &self.forward) {
            (Some(id), None) | (None, Some(id)) => Some(crate::messages::fetch::fetch_raw(
                &config,
                &account_config,
                backend,
                &self.mailbox,
                id,
            )?),
            _ => None,
        };

        #[cfg(not(any(feature = "imap", feature = "jmap", feature = "maildir")))]
        let source_raw: Option<Vec<u8>> = if self.reply.is_some() || self.forward.is_some() {
            bail!(
                "`--reply` / `--forward` need IMAP, JMAP or Maildir support, \
                 but this build has none"
            );
        } else {
            None
        };

        let raw = build_message(&self, source_raw.as_deref())?;

        if self.send {
            send_raw(config, account_config, backend, raw)?;
            return printer.out(Message::new("Message successfully composed and sent"));
        }

        let mut out = stdout().lock();
        out.write_all(&raw)?;
        Ok(())
    }
}

fn build_message(
    cmd: &MessagesComposeCommand,
    #[cfg_attr(
        not(any(feature = "imap", feature = "jmap", feature = "maildir")),
        allow(unused_variables)
    )]
    source: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut builder = MessageBuilder::new();

    if let Some(from) = &cmd.from {
        builder = builder.from(from.as_str());
    }

    if !cmd.to.is_empty() {
        builder = builder.to(addresses(&cmd.to));
    }
    if !cmd.cc.is_empty() {
        builder = builder.cc(addresses(&cmd.cc));
    }
    if !cmd.bcc.is_empty() {
        builder = builder.bcc(addresses(&cmd.bcc));
    }

    #[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
    let parsed_source = source.and_then(|raw| MessageParser::new().parse(raw));

    #[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
    let mut subject = cmd.subject.clone();
    #[cfg(not(any(feature = "imap", feature = "jmap", feature = "maildir")))]
    let subject = cmd.subject.clone();

    #[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
    let source_text: String = parsed_source
        .as_ref()
        .and_then(|m| m.body_text(0))
        .map(|c| c.into_owned())
        .unwrap_or_default();
    #[cfg(not(any(feature = "imap", feature = "jmap", feature = "maildir")))]
    let source_text = String::new();

    #[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
    if let Some(msg) = parsed_source.as_ref() {
        let prefix = if cmd.reply.is_some() { "Re: " } else { "Fwd: " };
        let src_subject = msg.subject().unwrap_or("");

        if subject.is_none() {
            subject = Some(if has_prefix(src_subject, prefix) {
                src_subject.to_string()
            } else {
                format!("{prefix}{src_subject}")
            });
        }

        if cmd.reply.is_some() && cmd.to.is_empty() {
            if let Some(addrs) = reply_recipients(msg) {
                builder = builder.to(addrs);
            }
        }

        if let Some(message_id) = msg.message_id() {
            if cmd.reply.is_some() {
                builder = builder.in_reply_to(vec![message_id.to_string()]);
            }
            let refs = compute_references(msg, message_id);
            if !refs.is_empty() {
                builder = builder.header("References", Raw::new(refs));
            }
        }
    }

    if let Some(s) = subject {
        builder = builder.subject(s);
    }

    let user_body = read_body(cmd)?;
    let signature = read_signature(cmd)?;
    let body = compose_body(
        &user_body,
        &source_text,
        cmd.quote_headline.as_deref().unwrap_or(""),
        signature.as_deref().unwrap_or(""),
        cmd.posting_style,
    );
    builder = builder.text_body(body);

    for path in &cmd.attach {
        let bytes = std::fs::read(path)
            .map_err(|err| anyhow!("read attachment {}: {err}", path.display()))?;
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "attachment".to_string());
        let mime = mime_for(path);
        builder = builder.attachment(mime, file_name, bytes);
    }

    let raw = builder
        .write_to_vec()
        .map_err(|err| anyhow!("serialize composed message: {err}"))?;
    Ok(raw)
}

fn addresses(values: &[String]) -> Address<'static> {
    Address::new_list(
        values
            .iter()
            .map(|s| Address::new_address(None::<&str>, s.clone()))
            .collect(),
    )
}

fn read_body(cmd: &MessagesComposeCommand) -> Result<String> {
    if let Some(body) = &cmd.body {
        return Ok(body.clone());
    }

    if let Some(path) = &cmd.body_file {
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

fn read_signature(cmd: &MessagesComposeCommand) -> Result<Option<String>> {
    if let Some(sig) = &cmd.signature {
        return Ok(Some(sig.clone()));
    }

    if let Some(path) = &cmd.signature_file {
        let s = std::fs::read_to_string(path)
            .map_err(|err| anyhow!("read signature file {}: {err}", path.display()))?;
        return Ok(Some(s));
    }

    Ok(None)
}

/// Builds the final text body from user input, optional source text
/// (reply/forward), an optional headline, an optional signature, and
/// the requested posting style.
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
        // drop trailing newline; sections are joined with "\n\n"
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

#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
fn has_prefix(subject: &str, prefix: &str) -> bool {
    let s = subject.trim_start();
    let p = prefix.trim_end_matches(' ').trim_end_matches(':');
    s.len() >= p.len() && s.get(..p.len()).map(|h| h.eq_ignore_ascii_case(p)) == Some(true)
}

#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
fn reply_recipients(msg: &mail_parser::Message<'_>) -> Option<Address<'static>> {
    use mail_parser::Address as ParserAddress;

    let header = msg
        .header("Reply-To")
        .or_else(|| msg.header("From"))
        .map(|h| h.clone());

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

#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
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

#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
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

fn mime_for(path: &std::path::Path) -> &'static str {
    #[cfg(feature = "maildir")]
    {
        let guess = mime_guess::from_path(path).first_or_octet_stream();
        // mime_guess returns owned String — leak to make 'static. Safe
        // because the few possible MIME strings recur indefinitely.
        let s = guess.essence_str().to_string();
        return Box::leak(s.into_boxed_str());
    }
    #[cfg(not(feature = "maildir"))]
    {
        let _ = path;
        "application/octet-stream"
    }
}

fn send_raw(
    config: Config,
    mut account_config: AccountConfig,
    backend: BackendArg,
    raw: Vec<u8>,
) -> Result<()> {
    #[cfg(any(feature = "smtp", feature = "jmap"))]
    use std::io::{Read, Write};

    #[cfg(any(feature = "smtp", feature = "jmap"))]
    const READ_BUFFER_SIZE: usize = 16 * 1024;

    #[cfg(feature = "smtp")]
    if backend.allows_smtp() {
        if let Some(smtp_config) = account_config.smtp.take() {
            use io_email::smtp::message_send::{MessageSend, MessageSendResult};
            use pimalaya_stream::std::smtp::SmtpSession;

            let account = crate::account::Account::new(config, account_config, smtp_config)?;
            let mut session = SmtpSession::new(
                account.backend.url.clone(),
                account.backend.tls.clone().try_into()?,
                account.backend.starttls,
                account.backend.sasl.clone().try_into()?,
            )?;

            let (reverse_path, forward_paths) = crate::messages::send::parse_envelope(&raw)?;
            let mut coroutine = MessageSend::new(reverse_path, forward_paths, raw);
            let mut buf = [0u8; READ_BUFFER_SIZE];
            let mut arg: Option<&[u8]> = None;

            return loop {
                match coroutine.resume(arg.take()) {
                    MessageSendResult::Ok => break Ok(()),
                    MessageSendResult::WantsRead => {
                        let n = session.stream.read(&mut buf)?;
                        arg = Some(&buf[..n]);
                    }
                    MessageSendResult::WantsWrite(bytes) => {
                        session.stream.write_all(&bytes)?;
                    }
                    MessageSendResult::Err(err) => bail!("{err}"),
                }
            };
        }
    }

    #[cfg(feature = "jmap")]
    if backend.allows_jmap() {
        if let Some(jmap_config) = account_config.jmap.take() {
            use crate::jmap::session::JmapSession;
            use io_email::jmap::message_send::{MessageSend, MessageSendResult};

            let identity_id = jmap_config.identity_id.clone().ok_or_else(|| {
                anyhow!(
                    "JMAP send requires `identity-id` in the [jmap] config; \
                     run `himalaya jmap identity get` to find one"
                )
            })?;
            let drafts_mailbox_id = jmap_config.drafts_mailbox_id.clone().ok_or_else(|| {
                anyhow!(
                    "JMAP send requires `drafts-mailbox-id` in the [jmap] config; \
                     run `himalaya jmap mailbox query --role drafts` to find one"
                )
            })?;
            let account = crate::account::Account::new(config, account_config, jmap_config)?;
            let mut session = JmapSession::new(
                account.backend.server.clone(),
                account.backend.tls.clone().try_into()?,
                account.backend.auth.clone().try_into()?,
            )?;

            let mut coroutine = MessageSend::new(
                &session.session,
                &session.http_auth,
                raw,
                identity_id,
                drafts_mailbox_id,
            )?;
            let mut buf = [0u8; READ_BUFFER_SIZE];
            let mut arg: Option<&[u8]> = None;

            return loop {
                match coroutine.resume(arg.take()) {
                    MessageSendResult::Ok => break Ok(()),
                    MessageSendResult::WantsRead => {
                        let n = session.stream.read(&mut buf)?;
                        arg = Some(&buf[..n]);
                    }
                    MessageSendResult::WantsWrite(bytes) => {
                        session.stream.write_all(&bytes)?;
                    }
                    MessageSendResult::Err(err) => bail!("{err}"),
                }
            };
        }
    }

    let _ = config;
    let _ = account_config;
    let _ = raw;
    bail!("no backend matching `{backend}` allows sending for this account")
}
