use error_chain::error_chain;
use lettre;
use mailparse::{self, MailHeaderMap};
use rfc2047_decoder;
use serde::{
    ser::{self, SerializeStruct},
    Serialize,
};
use std::{fmt, result};
use uuid::Uuid;

use crate::config::model::{Account, Config};
use crate::flag::model::{Flag, Flags};
use crate::table::{self, DisplayRow, DisplayTable};

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
        write!(f, "{}", self.content)
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

// #[derive(Debug, Serialize, PartialEq)]
// #[serde(rename_all = "lowercase")]
// pub enum Flag {
//     Seen,
//     Answered,
//     Flagged,
// }

// impl Flag {
//     fn from_imap_flag(flag: &imap::types::Flag<'_>) -> Option<Self> {
//         match flag {
//             imap::types::Flag::Seen => Some(Self::Seen),
//             imap::types::Flag::Answered => Some(Self::Answered),
//             imap::types::Flag::Flagged => Some(Self::Flagged),
//             _ => None,
//         }
//     }
// }

#[derive(Debug, Serialize)]
pub struct Msg<'m> {
    pub uid: u32,
    pub flags: Flags<'m>,
    pub subject: String,
    pub sender: String,
    pub date: String,

    #[serde(skip_serializing)]
    raw: Vec<u8>,
}

impl<'m> From<Vec<u8>> for Msg<'m> {
    fn from(raw: Vec<u8>) -> Self {
        Self {
            uid: 0,
            flags: Flags::new(&[]),
            subject: String::from(""),
            sender: String::from(""),
            date: String::from(""),
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
                    .and_then(|subj| rfc2047_decoder::decode(subj).ok())
                    .unwrap_or_default(),
                sender: envelope
                    .from
                    .as_ref()
                    .and_then(|addrs| addrs.first()?.name)
                    .and_then(|name| rfc2047_decoder::decode(name).ok())
                    .unwrap_or_default(),
                date: fetch
                    .internal_date()
                    .map(|date| date.naive_local().to_string())
                    .unwrap_or_default(),
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
        use lettre::message::header::{ContentTransferEncoding, ContentType};
        use lettre::message::{Message, SinglePart};

        let parsed = self.parse()?;
        let msg = parsed
            .headers
            .iter()
            .fold(Message::builder(), |msg, h| {
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
                    _ => msg,
                }
            })
            .singlepart(
                SinglePart::builder()
                    .header(ContentType("text/plain; charset=utf-8".parse().unwrap()))
                    .header(ContentTransferEncoding::Base64)
                    .body(parsed.get_body_raw()?),
            )?;

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
        let mut tpl = vec![];

        // "Content" headers
        tpl.push("Content-Type: text/plain; charset=utf-8".to_string());
        tpl.push("Content-Transfer-Encoding: 8bit".to_string());

        // "From" header
        tpl.push(format!("From: {}", config.address(account)));

        // "To" header
        tpl.push("To: ".to_string());

        // "Subject" header
        tpl.push("Subject: ".to_string());

        Ok(Tpl(tpl.join("\r\n")))
    }

    pub fn build_reply_tpl(&self, config: &Config, account: &Account) -> Result<Tpl> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();
        let mut tpl = vec![];

        // "Content" headers
        tpl.push("Content-Type: text/plain; charset=utf-8".to_string());
        tpl.push("Content-Transfer-Encoding: 8bit".to_string());

        // "From" header
        tpl.push(format!("From: {}", config.address(account)));

        // "In-Reply-To" header
        if let Some(msg_id) = headers.get_first_value("message-id") {
            tpl.push(format!("In-Reply-To: {}", msg_id));
        }

        // "To" header
        let to = headers
            .get_first_value("reply-to")
            .or(headers.get_first_value("from"))
            .unwrap_or(String::new());
        tpl.push(format!("To: {}", to));

        // "Subject" header
        let subject = headers.get_first_value("subject").unwrap_or(String::new());
        tpl.push(format!("Subject: Re: {}", subject));

        // Separator between headers and body
        tpl.push(String::new());

        // Original msg prepend with ">"
        let thread = self
            .text_bodies("text/plain")?
            .replace("\r", "")
            .split("\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>()
            .join("\r\n");
        tpl.push(thread);

        Ok(Tpl(tpl.join("\r\n")))
    }

    pub fn build_reply_all_tpl(&self, config: &Config, account: &Account) -> Result<Tpl> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();
        let mut tpl = vec![];

        // "Content" headers
        tpl.push("Content-Type: text/plain; charset=utf-8".to_string());
        tpl.push("Content-Transfer-Encoding: 8bit".to_string());

        // "From" header
        tpl.push(format!("From: {}", config.address(account)));

        // "In-Reply-To" header
        if let Some(msg_id) = headers.get_first_value("message-id") {
            tpl.push(format!("In-Reply-To: {}", msg_id));
        }

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
        tpl.push(format!("To: {}", vec![reply_to, to].concat().join(", ")));

        // "Cc" header
        let cc = headers
            .get_all_values("cc")
            .iter()
            .flat_map(|addrs| addrs.split(","))
            .map(|addr| addr.trim().to_string())
            .collect::<Vec<String>>();
        if !cc.is_empty() {
            tpl.push(format!("Cc: {}", cc.join(", ")));
        }

        // "Subject" header
        let subject = headers.get_first_value("subject").unwrap_or(String::new());
        tpl.push(format!("Subject: Re: {}", subject));

        // Separator between headers and body
        tpl.push(String::new());

        // Original msg prepend with ">"
        let thread = self
            .text_bodies("text/plain")?
            .split("\r\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>()
            .join("\r\n");
        tpl.push(thread);

        Ok(Tpl(tpl.join("\r\n")))
    }

    pub fn build_forward_tpl(&self, config: &Config, account: &Account) -> Result<Tpl> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();
        let mut tpl = vec![];

        // "Content" headers
        tpl.push("Content-Type: text/plain; charset=utf-8".to_string());
        tpl.push("Content-Transfer-Encoding: 8bit".to_string());

        // "From" header
        tpl.push(format!("From: {}", config.address(account)));

        // "To" header
        tpl.push("To: ".to_string());

        // "Subject" header
        let subject = headers.get_first_value("subject").unwrap_or(String::new());
        tpl.push(format!("Subject: Fwd: {}", subject));

        // Separator between headers and body
        tpl.push(String::new());

        // Original msg
        tpl.push("-------- Forwarded Message --------".to_string());
        tpl.push(self.text_bodies("text/plain")?);

        Ok(Tpl(tpl.join("\r\n")))
    }
}

impl<'m> DisplayRow for Msg<'m> {
    fn to_row(&self) -> Vec<table::Cell> {
        use crate::table::*;

        let unseen = if self.flags.contains(&Flag::Seen) {
            RESET
        } else {
            BOLD
        };

        vec![
            Cell::new(&[unseen.to_owned(), RED], &self.uid.to_string()),
            Cell::new(&[unseen.to_owned(), WHITE], &self.flags.to_string()),
            FlexCell::new(&[unseen.to_owned(), GREEN], &self.subject),
            Cell::new(&[unseen.to_owned(), BLUE], &self.sender),
            Cell::new(&[unseen.to_owned(), YELLOW], &self.date),
        ]
    }
}

// Msgs

#[derive(Debug, Serialize)]
pub struct Msgs<'m>(pub Vec<Msg<'m>>);

impl<'m> DisplayTable<'m, Msg<'m>> for Msgs<'m> {
    fn header_row() -> Vec<table::Cell> {
        use crate::table::*;

        vec![
            Cell::new(&[BOLD, UNDERLINE, WHITE], "UID"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "FLAGS"),
            FlexCell::new(&[BOLD, UNDERLINE, WHITE], "SUBJECT"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "SENDER"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "DATE"),
        ]
    }

    fn rows(&self) -> &Vec<Msg<'m>> {
        &self.0
    }
}

impl<'m> From<&'m imap::types::ZeroCopy<Vec<imap::types::Fetch>>> for Msgs<'m> {
    fn from(fetches: &'m imap::types::ZeroCopy<Vec<imap::types::Fetch>>) -> Self {
        Self(fetches.iter().map(Msg::from).collect::<Vec<_>>())
    }
}

impl<'m> fmt::Display for Msgs<'m> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n{}", self.to_table())
    }
}
