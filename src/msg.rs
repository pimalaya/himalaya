use lettre;
use mailparse::{self, MailHeaderMap};
use serde::Serialize;
use std::{fmt, result};

use crate::config::{Account, Config};
use crate::table::{self, DisplayRow, DisplayTable};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    ParseMsgError(mailparse::MailParseError),
    BuildSendableMsgError(lettre::error::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(msg): ")?;
        match self {
            Error::ParseMsgError(err) => err.fmt(f),
            Error::BuildSendableMsgError(err) => err.fmt(f),
        }
    }
}

impl From<mailparse::MailParseError> for Error {
    fn from(err: mailparse::MailParseError) -> Error {
        Error::ParseMsgError(err)
    }
}

impl From<lettre::error::Error> for Error {
    fn from(err: lettre::error::Error) -> Error {
        Error::BuildSendableMsgError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Msg

#[derive(Debug, Serialize)]
pub struct Msg {
    pub uid: u32,
    pub flags: Vec<String>,

    #[serde(skip_serializing)]
    raw: Vec<u8>,
}

impl From<String> for Msg {
    fn from(item: String) -> Self {
        Self {
            uid: 0,
            flags: vec![],
            raw: item.as_bytes().to_vec(),
        }
    }
}

impl From<Vec<u8>> for Msg {
    fn from(item: Vec<u8>) -> Self {
        Self {
            uid: 0,
            flags: vec![],
            raw: item,
        }
    }
}

impl From<&imap::types::Fetch> for Msg {
    fn from(fetch: &imap::types::Fetch) -> Self {
        Self {
            uid: fetch.uid.unwrap_or_default(),
            flags: vec![],
            raw: fetch.body().unwrap_or_default().to_vec(),
        }
    }
}

impl<'a> Msg {
    pub fn parse(&'a self) -> Result<mailparse::ParsedMail<'a>> {
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

    fn extract_attachments_into(part: &mailparse::ParsedMail, parts: &mut Vec<(String, Vec<u8>)>) {
        match part.subparts.len() {
            0 => {
                let content_disp = part.get_content_disposition();
                let content_type = part
                    .get_headers()
                    .get_first_value("content-type")
                    .unwrap_or_default();

                let default_attachment_name = format!("attachment-{}", parts.len());
                let attachment_name = content_disp
                    .params
                    .get("filename")
                    .unwrap_or(&default_attachment_name)
                    .to_owned();

                if !content_type.starts_with("text") {
                    parts.push((attachment_name, part.get_body_raw().unwrap_or_default()))
                }
            }
            _ => {
                part.subparts
                    .iter()
                    .for_each(|part| Self::extract_attachments_into(part, parts));
            }
        }
    }

    pub fn extract_attachments(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let mut parts = vec![];
        Self::extract_attachments_into(&self.parse()?, &mut parts);
        Ok(parts)
    }

    pub fn build_new_tpl(config: &Config, account: &Account) -> Result<String> {
        let mut tpl = vec![];

        // "From" header
        tpl.push(format!("From: {}", config.address(account)));

        // "To" header
        tpl.push("To: ".to_string());

        // "Subject" header
        tpl.push("Subject: ".to_string());

        Ok(tpl.join("\r\n"))
    }

    pub fn build_reply_tpl(&self, config: &Config, account: &Account) -> Result<String> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();
        let mut tpl = vec![];

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
        let thread = msg
            .get_body()
            .unwrap()
            .split("\r\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>()
            .join("\r\n");
        tpl.push(thread);

        Ok(tpl.join("\r\n"))
    }

    pub fn build_reply_all_tpl(&self, config: &Config, account: &Account) -> Result<String> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();
        let mut tpl = vec![];

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
        let thread = msg
            .get_body()
            .unwrap()
            .split("\r\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>()
            .join("\r\n");
        tpl.push(thread);

        Ok(tpl.join("\r\n"))
    }

    pub fn build_forward_tpl(&self, config: &Config, account: &Account) -> Result<String> {
        let msg = &self.parse()?;
        let headers = msg.get_headers();
        let mut tpl = vec![];

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
        tpl.push(msg.get_body().unwrap_or(String::new()));

        Ok(tpl.join("\r\n"))
    }
}

impl DisplayRow for Msg {
    fn to_row(&self) -> Vec<table::Cell> {
        match self.parse() {
            Err(_) => vec![],
            Ok(parsed) => {
                let headers = parsed.get_headers();

                let uid = &self.uid.to_string();
                let flags = match self.extract_attachments().map(|vec| vec.is_empty()) {
                    Ok(false) => "",
                    _ => " ",
                };
                let sender = headers
                    .get_first_value("reply-to")
                    .or(headers.get_first_value("from"))
                    .unwrap_or_default();
                let subject = headers.get_first_value("subject").unwrap_or_default();
                let date = headers.get_first_value("date").unwrap_or_default();

                {
                    use crate::table::*;

                    vec![
                        Cell::new(&[RED], &uid),
                        Cell::new(&[WHITE], &flags),
                        Cell::new(&[BLUE], &sender),
                        FlexCell::new(&[GREEN], &subject),
                        Cell::new(&[YELLOW], &date),
                    ]
                }
            }
        }
    }
}

// Msgs

#[derive(Debug, Serialize)]
pub struct Msgs(pub Vec<Msg>);

impl<'a> DisplayTable<'a, Msg> for Msgs {
    fn header_row() -> Vec<table::Cell> {
        use crate::table::*;

        vec![
            Cell::new(&[BOLD, UNDERLINE, WHITE], "UID"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "FLAGS"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "SENDER"),
            FlexCell::new(&[BOLD, UNDERLINE, WHITE], "SUBJECT"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "DATE"),
        ]
    }

    fn rows(&self) -> &Vec<Msg> {
        &self.0
    }
}

impl fmt::Display for Msgs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_table())
    }
}
