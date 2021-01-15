use lettre;
use mailparse::{self, MailHeaderMap};
use std::{fmt, ops, result};

use crate::Config;

// Error wrapper

#[derive(Debug)]
pub enum Error {
    ParseMsgError(mailparse::MailParseError),
    BuildEmailError(lettre::error::Error),
    TryError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(msg): ")?;
        match self {
            Error::ParseMsgError(err) => err.fmt(f),
            Error::BuildEmailError(err) => err.fmt(f),
            Error::TryError => write!(f, "cannot parse"),
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
        Error::BuildEmailError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Wrapper around mailparse::ParsedMail and lettre::Message

#[derive(Debug)]
pub struct Msg<'a>(mailparse::ParsedMail<'a>);

impl<'a> ops::Deref for Msg<'a> {
    type Target = mailparse::ParsedMail<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Msg<'a> {
    pub fn from(bytes: &'a [u8]) -> Result<Self> {
        Ok(Self(mailparse::parse_mail(bytes)?))
    }

    pub fn to_vec(&self) -> Result<Vec<u8>> {
        let headers = self.0.get_headers().get_raw_bytes().to_vec();
        let sep = "\r\n".as_bytes().to_vec();
        let body = self.0.get_body()?.as_bytes().to_vec();

        Ok(vec![headers, sep, body].concat())
    }

    pub fn to_sendable_msg(&self) -> Result<lettre::Message> {
        use lettre::message::header::{ContentTransferEncoding, ContentType};
        use lettre::message::{Message, SinglePart};

        let msg = self
            .0
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
                    .body(self.0.get_body_raw()?),
            )?;

        Ok(msg)
    }

    fn extract_parts_into(part: &mailparse::ParsedMail, parts: &mut Vec<(String, Vec<u8>)>) {
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
                    .for_each(|part| Self::extract_parts_into(part, parts));
            }
        }
    }

    pub fn extract_parts(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let mut parts = vec![];
        Self::extract_parts_into(&self.0, &mut parts);
        Ok(parts)
    }

    pub fn build_new_tpl(config: &Config) -> Result<String> {
        let mut tpl = vec![];

        // "From" header
        tpl.push(format!("From: {}", config.email_full()));

        // "To" header
        tpl.push("To: ".to_string());

        // "Subject" header
        tpl.push("Subject: ".to_string());

        Ok(tpl.join("\r\n"))
    }

    pub fn build_reply_tpl(&self, config: &Config) -> Result<String> {
        let msg = &self.0;
        let headers = msg.get_headers();
        let mut tpl = vec![];

        // "From" header
        tpl.push(format!("From: {}", config.email_full()));

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

    pub fn build_reply_all_tpl(&self, config: &Config) -> Result<String> {
        let msg = &self.0;
        let headers = msg.get_headers();
        let mut tpl = vec![];

        // "From" header
        tpl.push(format!("From: {}", config.email_full()));

        // "In-Reply-To" header
        if let Some(msg_id) = headers.get_first_value("message-id") {
            tpl.push(format!("In-Reply-To: {}", msg_id));
        }

        // "To" header
        // All addresses coming from original "To" …
        let email: lettre::Address = config.email.parse().unwrap();
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

    pub fn build_forward_tpl(&self, config: &Config) -> Result<String> {
        let msg = &self.0;
        let headers = msg.get_headers();
        let mut tpl = vec![];

        // "From" header
        tpl.push(format!("From: {}", config.email_full()));

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
