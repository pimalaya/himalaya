use error_chain::error_chain;
use lettre;
use log::warn;
use mailparse::{self, MailHeaderMap};
use rfc2047_decoder;
use serde::{
    ser::{self, SerializeStruct},
    Serialize,
};
use std::{borrow::Cow, fmt, fs, path::PathBuf, result};
use tree_magic;
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;

use crate::{
    config::model::{Account, Config},
    flag::model::{Flag, Flags},
    table::{Cell, Row, Table},
};

error_chain! {
    foreign_links {
        Mailparse(mailparse::MailParseError);
        Lettre(lettre::error::Error);
    }
}

// Template

#[derive(Debug)]
pub struct Tpl(String);

impl fmt::Display for Tpl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Tpl {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Tpl", 1)?;
        state.serialize_field("template", &self.0)?;
        state.end()
    }
}

// Attachments

#[derive(Debug)]
pub struct Attachment {
    pub filename: String,
    pub raw: Vec<u8>,
}

impl<'a> Attachment {
    // TODO: put in common with ReadableMsg
    pub fn from_part(part: &'a mailparse::ParsedMail) -> Self {
        Self {
            filename: part
                .get_content_disposition()
                .params
                .get("filename")
                .unwrap_or(&Uuid::new_v4().to_simple().to_string())
                .to_owned(),
            raw: part.get_body_raw().unwrap_or_default(),
        }
    }
}

#[derive(Debug)]
pub struct Attachments(pub Vec<Attachment>);

impl<'a> Attachments {
    fn extract_from_part(&'a mut self, part: &'a mailparse::ParsedMail) {
        if part.subparts.is_empty() {
            let ctype = part
                .get_headers()
                .get_first_value("content-type")
                .unwrap_or_default();
            if !ctype.starts_with("text") {
                self.0.push(Attachment::from_part(part));
            }
        } else {
            part.subparts
                .iter()
                .for_each(|part| self.extract_from_part(part));
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let msg = mailparse::parse_mail(bytes)?;
        let mut attachments = Self(vec![]);
        attachments.extract_from_part(&msg);
        Ok(attachments)
    }
}

// Readable message

#[derive(Debug)]
pub struct ReadableMsg {
    pub content: String,
    pub has_attachment: bool,
}

impl Serialize for ReadableMsg {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("ReadableMsg", 2)?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("hasAttachment", if self.has_attachment { &1 } else { &0 })?;
        state.end()
    }
}

impl fmt::Display for ReadableMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.content)
    }
}

impl<'a> ReadableMsg {
    fn flatten_parts(part: &'a mailparse::ParsedMail) -> Vec<&'a mailparse::ParsedMail<'a>> {
        if part.subparts.is_empty() {
            vec![part]
        } else {
            part.subparts
                .iter()
                .flat_map(Self::flatten_parts)
                .collect::<Vec<_>>()
        }
    }

    pub fn from_bytes(mime: &str, bytes: &[u8]) -> Result<Self> {
        let msg = mailparse::parse_mail(bytes)?;
        let (text_part, html_part, has_attachment) = Self::flatten_parts(&msg).into_iter().fold(
            (None, None, false),
            |(mut text_part, mut html_part, mut has_attachment), part| {
                let ctype = part
                    .get_headers()
                    .get_first_value("content-type")
                    .unwrap_or_default();

                if text_part.is_none() && ctype.starts_with("text/plain") {
                    text_part = part.get_body().ok();
                } else {
                    if html_part.is_none() && ctype.starts_with("text/html") {
                        html_part = part.get_body().ok();
                    } else {
                        has_attachment = true
                    };
                };

                (text_part, html_part, has_attachment)
            },
        );

        let content = if mime == "text/plain" {
            text_part.or(html_part).unwrap_or_default()
        } else {
            html_part.or(text_part).unwrap_or_default()
        };

        Ok(Self {
            content,
            has_attachment,
        })
    }
}

// Message

#[derive(Debug)]
pub struct Msg<'m> {
    pub uid: u32,
    pub flags: Flags<'m>,
    pub subject: String,
    pub sender: String,
    pub date: String,
    pub attachments: Vec<String>,
    pub raw: Vec<u8>,
}

impl<'a> Serialize for Msg<'a> {
    fn serialize<T>(&self, serializer: T) -> result::Result<T::Ok, T::Error>
    where
        T: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Msg", 7)?;
        state.serialize_field("uid", &self.uid)?;
        state.serialize_field("flags", &self.flags)?;
        state.serialize_field("subject", &self.subject)?;
        state.serialize_field(
            "subject_len",
            &UnicodeWidthStr::width(self.subject.as_str()),
        )?;
        state.serialize_field("sender", &self.sender)?;
        state.serialize_field("sender_len", &UnicodeWidthStr::width(self.sender.as_str()))?;
        state.serialize_field("date", &self.date)?;
        state.end()
    }
}

impl<'m> From<Vec<u8>> for Msg<'m> {
    fn from(raw: Vec<u8>) -> Self {
        Self {
            uid: 0,
            flags: Flags::new(&[]),
            subject: String::from(""),
            sender: String::from(""),
            date: String::from(""),
            attachments: vec![],
            raw,
        }
    }
}

impl<'m> From<String> for Msg<'m> {
    fn from(raw: String) -> Self {
        Self::from(raw.as_bytes().to_vec())
    }
}

impl<'m> From<&'m imap::types::Fetch> for Msg<'m> {
    fn from(fetch: &'m imap::types::Fetch) -> Self {
        match fetch.envelope() {
            None => Self::from(fetch.body().unwrap_or_default().to_vec()),
            Some(envelope) => Self {
                uid: fetch.uid.unwrap_or_default(),
                flags: Flags::new(fetch.flags()),
                subject: envelope
                    .subject
                    .as_ref()
                    .and_then(|subj| rfc2047_decoder::decode(subj).ok())
                    .unwrap_or_default(),
                sender: envelope
                    .from
                    .as_ref()
                    .and_then(|addrs| addrs.first())
                    .and_then(|addr| {
                        addr.name
                            .as_ref()
                            .and_then(|name| rfc2047_decoder::decode(name).ok())
                            .or_else(|| {
                                let mbox = addr
                                    .mailbox
                                    .as_ref()
                                    .and_then(|mbox| String::from_utf8(mbox.to_vec()).ok())
                                    .unwrap_or(String::from("unknown"));
                                let host = addr
                                    .host
                                    .as_ref()
                                    .and_then(|host| String::from_utf8(host.to_vec()).ok())
                                    .unwrap_or(String::from("unknown"));
                                Some(format!("{}@{}", mbox, host))
                            })
                    })
                    .unwrap_or(String::from("unknown")),
                date: fetch
                    .internal_date()
                    .map(|date| date.naive_local().to_string())
                    .unwrap_or_default(),
                attachments: vec![],
                raw: fetch.body().unwrap_or_default().to_vec(),
            },
        }
    }
}

impl<'m> Msg<'m> {
    pub fn parse(&'m self) -> Result<mailparse::ParsedMail<'m>> {
        Ok(mailparse::parse_mail(&self.raw)?)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>> {
        let parsed = self.parse()?;
        let headers = parsed.get_headers().get_raw_bytes().to_vec();
        let sep = "\r\n".as_bytes().to_vec();
        let body = parsed.get_body()?.as_bytes().to_vec();

        Ok(vec![headers, sep, body].concat())
    }

    pub fn to_sendable_msg(&self) -> Result<lettre::Message> {
        use lettre::message::{
            header::*,
            {Body, Message, MultiPart, SinglePart},
        };

        let mut encoding = ContentTransferEncoding::Base64;
        let parsed = self.parse()?;
        let msg_builder = parsed.headers.iter().fold(Message::builder(), |msg, h| {
            let value = String::from_utf8(h.get_value_raw().to_vec())
                .unwrap()
                .replace("\r", "");

            match h.get_key().to_lowercase().as_str() {
                "in-reply-to" => msg.in_reply_to(value.parse().unwrap()),
                "from" => match value.parse() {
                    Ok(addr) => msg.from(addr),
                    Err(_) => msg,
                },
                "to" => value
                    .split(",")
                    .fold(msg, |msg, addr| match addr.trim().parse() {
                        Ok(addr) => msg.to(addr),
                        Err(_) => msg,
                    }),
                "cc" => value
                    .split(",")
                    .fold(msg, |msg, addr| match addr.trim().parse() {
                        Ok(addr) => msg.cc(addr),
                        Err(_) => msg,
                    }),
                "bcc" => value
                    .split(",")
                    .fold(msg, |msg, addr| match addr.trim().parse() {
                        Ok(addr) => msg.bcc(addr),
                        Err(_) => msg,
                    }),
                "subject" => msg.subject(value),
                "content-transfer-encoding" => {
                    match value.to_lowercase().as_str() {
                        "8bit" => encoding = ContentTransferEncoding::EightBit,
                        "7bit" => encoding = ContentTransferEncoding::SevenBit,
                        "quoted-printable" => encoding = ContentTransferEncoding::QuotedPrintable,
                        "base64" => encoding = ContentTransferEncoding::Base64,
                        _ => warn!("unsupported encoding, default to base64"),
                    }
                    msg
                }
                _ => msg,
            }
        });

        let text_part = SinglePart::builder()
            .header(ContentType::TEXT_PLAIN)
            .header(encoding)
            .body(parsed.get_body_raw()?);

        let msg = if self.attachments.is_empty() {
            msg_builder.singlepart(text_part)
        } else {
            let mut parts = MultiPart::mixed().singlepart(text_part);

            for attachment in &self.attachments {
                let attachment_name = PathBuf::from(attachment);
                let attachment_name = attachment_name
                    .file_name()
                    .map(|fname| fname.to_string_lossy())
                    .unwrap_or(Cow::from(Uuid::new_v4().to_string()));
                let attachment_content = fs::read(attachment)
                    .chain_err(|| format!("Could not read attachment `{}`", attachment))?;
                let attachment_ctype = tree_magic::from_u8(&attachment_content);

                parts = parts.singlepart(
                    SinglePart::builder()
                        .content_type(attachment_ctype.parse().chain_err(|| {
                            format!("Could not parse content type `{}`", attachment_ctype)
                        })?)
                        .header(ContentDisposition::attachment(&attachment_name))
                        .body(Body::new(attachment_content)),
                );
            }

            msg_builder.multipart(parts)
        }?;

        Ok(msg)
    }

    fn extract_text_bodies_into(part: &mailparse::ParsedMail, mime: &str, parts: &mut Vec<String>) {
        match part.subparts.len() {
            0 => {
                let content_type = part
                    .get_headers()
                    .get_first_value("content-type")
                    .unwrap_or_default();

                if content_type.starts_with(mime) {
                    parts.push(part.get_body().unwrap_or_default())
                }
            }
            _ => {
                part.subparts
                    .iter()
                    .for_each(|part| Self::extract_text_bodies_into(part, mime, parts));
            }
        }
    }

    fn extract_text_bodies(&self, mime: &str) -> Result<Vec<String>> {
        let mut parts = vec![];
        Self::extract_text_bodies_into(&self.parse()?, mime, &mut parts);
        Ok(parts)
    }

    pub fn text_bodies(&self, mime: &str) -> Result<String> {
        let text_bodies = self.extract_text_bodies(mime)?;
        Ok(text_bodies.join("\r\n"))
    }

    pub fn build_new_tpl(config: &Config, account: &Account) -> Result<Tpl> {
        let msg_spec = MsgSpec {
            in_reply_to: None,
            to: None,
            cc: None,
            subject: None,
            default_content: None,
        };
        Msg::build_tpl(config, account, msg_spec)
    }

    pub fn build_reply_tpl(&self, config: &Config, account: &Account) -> Result<Tpl> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();
        let to = headers
            .get_first_value("reply-to")
            .or(headers.get_first_value("from"));
        let to = match to {
            Some(t) => Some(vec![t]),
            None => None,
        };

        let thread = self // Original msg prepend with ">"
            .text_bodies("text/plain")?
            .replace("\r", "")
            .split("\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>();

        let msg_spec = MsgSpec {
            in_reply_to: headers.get_first_value("message-id"),
            to,
            cc: None,
            subject: headers.get_first_value("subject"),
            default_content: Some(thread),
        };
        Msg::build_tpl(config, account, msg_spec)
    }

    pub fn build_reply_all_tpl(&self, config: &Config, account: &Account) -> Result<Tpl> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();

        // "To" header
        // All addresses coming from original "To" …
        let email: lettre::Address = account.email.parse().unwrap();
        let to = headers
            .get_all_values("to")
            .iter()
            .flat_map(|addrs| addrs.split(","))
            .fold(vec![], |mut mboxes, addr| {
                match addr.trim().parse::<lettre::message::Mailbox>() {
                    Err(_) => mboxes,
                    Ok(mbox) => {
                        // … except current user's one (from config) …
                        if mbox.email != email {
                            mboxes.push(mbox.to_string());
                        }
                        mboxes
                    }
                }
            });
        // … and the ones coming from either "Reply-To" or "From"
        let reply_to = headers
            .get_all_values("reply-to")
            .iter()
            .flat_map(|addrs| addrs.split(","))
            .map(|addr| addr.trim().to_string())
            .collect::<Vec<String>>();
        let reply_to = if reply_to.is_empty() {
            headers
                .get_all_values("from")
                .iter()
                .flat_map(|addrs| addrs.split(","))
                .map(|addr| addr.trim().to_string())
                .collect::<Vec<String>>()
        } else {
            reply_to
        };

        // "Cc" header
        let cc = Some(
            headers
                .get_all_values("cc")
                .iter()
                .flat_map(|addrs| addrs.split(","))
                .map(|addr| addr.trim().to_string())
                .collect::<Vec<String>>(),
        );

        // Original msg prepend with ">"
        let thread = self
            .text_bodies("text/plain")?
            .split("\r\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>();

        let msg_spec = MsgSpec {
            in_reply_to: headers.get_first_value("message-id"),
            cc,
            to: Some(vec![reply_to, to].concat()),
            subject: headers.get_first_value("subject"),
            default_content: Some(thread),
        };
        Msg::build_tpl(config, account, msg_spec)
    }

    pub fn build_forward_tpl(&self, config: &Config, account: &Account) -> Result<Tpl> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();

        let subject = format!(
            "Fwd: {}",
            headers
                .get_first_value("subject")
                .unwrap_or_else(String::new)
        );
        let original_msg = vec![
            "-------- Forwarded Message --------".to_string(),
            self.text_bodies("text/plain")?,
        ];

        let msg_spec = MsgSpec {
            in_reply_to: None,
            cc: None,
            to: None,
            subject: Some(subject),
            default_content: Some(original_msg),
        };
        Msg::build_tpl(config, account, msg_spec)
    }

    fn add_from_header(tpl: &mut Vec<String>, from: Option<String>) {
        tpl.push(format!("From: {}", from.unwrap_or_else(String::new)));
    }

    fn add_in_reply_to_header(tpl: &mut Vec<String>, in_reply_to: Option<String>) {
        if let Some(r) = in_reply_to {
            tpl.push(format!("In-Reply-To: {}", r));
        }
    }

    fn add_cc_header(tpl: &mut Vec<String>, cc: Option<Vec<String>>) {
        if let Some(c) = cc {
            tpl.push(format!("Cc: {}", c.join(", ")));
        }
    }

    fn add_to_header(tpl: &mut Vec<String>, to: Option<Vec<String>>) {
        tpl.push(format!(
            "To: {}",
            match to {
                Some(t) => {
                    t.join(", ")
                }
                None => {
                    String::new()
                }
            }
        ));
    }

    fn add_subject_header(tpl: &mut Vec<String>, subject: Option<String>) {
        tpl.push(format!("Subject: {}", subject.unwrap_or_else(String::new)));
    }

    fn add_content(tpl: &mut Vec<String>, content: Option<Vec<String>>) {
        if let Some(c) = content {
            tpl.push(String::new()); // Separator between headers and body
            tpl.extend(c);
        }
    }

    fn add_signature(tpl: &mut Vec<String>, config: &Config, account: &Account) {
        if let Some(sig) = config.signature(&account) {
            tpl.push(String::new());
            for line in sig.split("\n") {
                tpl.push(line.to_string());
            }
        }
    }

    fn build_tpl(config: &Config, account: &Account, msg_spec: MsgSpec) -> Result<Tpl> {
        let mut tpl = vec![];
        Msg::add_from_header(&mut tpl, Some(config.address(account)));
        Msg::add_in_reply_to_header(&mut tpl, msg_spec.in_reply_to);
        Msg::add_cc_header(&mut tpl, msg_spec.cc);
        Msg::add_to_header(&mut tpl, msg_spec.to);
        Msg::add_subject_header(&mut tpl, msg_spec.subject);
        Msg::add_content(&mut tpl, msg_spec.default_content);
        Msg::add_signature(&mut tpl, config, account);
        Ok(Tpl(tpl.join("\r\n")))
    }
}

struct MsgSpec {
    in_reply_to: Option<String>,
    to: Option<Vec<String>>,
    cc: Option<Vec<String>>,
    subject: Option<String>,
    default_content: Option<Vec<String>>,
}

impl<'m> Table for Msg<'m> {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("UID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let is_seen = !self.flags.contains(&Flag::Seen);
        Row::new()
            .cell(Cell::new(&self.uid.to_string()).bold_if(is_seen).red())
            .cell(Cell::new(&self.flags.to_string()).bold_if(is_seen).white())
            .cell(
                Cell::new(&self.subject)
                    .shrinkable()
                    .bold_if(is_seen)
                    .green(),
            )
            .cell(Cell::new(&self.sender).bold_if(is_seen).blue())
            .cell(Cell::new(&self.date).bold_if(is_seen).yellow())
    }
}

// Msgs

#[derive(Debug, Serialize)]
pub struct Msgs<'a>(pub Vec<Msg<'a>>);

impl<'a> From<&'a imap::types::ZeroCopy<Vec<imap::types::Fetch>>> for Msgs<'a> {
    fn from(fetches: &'a imap::types::ZeroCopy<Vec<imap::types::Fetch>>) -> Self {
        Self(fetches.iter().rev().map(Msg::from).collect::<Vec<_>>())
    }
}

impl Msgs<'_> {
    pub fn new() -> Self {
        Self(vec![])
    }
}

impl fmt::Display for Msgs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\n{}", Table::render(&self.0))
    }
}
