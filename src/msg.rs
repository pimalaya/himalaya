use lettre;
use mailparse;
use std::{fmt, result};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    ParseMsgError(mailparse::MailParseError),
    BuildEmailError(lettre::error::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(msg): ")?;
        match self {
            Error::ParseMsgError(err) => err.fmt(f),
            Error::BuildEmailError(err) => err.fmt(f),
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

pub struct Msg(lettre::Message);

impl Msg {
    pub fn from_raw(bytes: &[u8]) -> Result<Msg> {
        use lettre::message::header::{ContentTransferEncoding, ContentType};
        use lettre::message::{Message, SinglePart};

        let parsed_msg = mailparse::parse_mail(bytes)?;
        let built_msg = parsed_msg
            .headers
            .iter()
            .fold(Message::builder(), |msg, h| {
                match h.get_key().to_lowercase().as_str() {
                    "from" => msg.from(h.get_value().parse().unwrap()),
                    "to" => msg.to(h.get_value().parse().unwrap()),
                    "cc" => match h.get_value().parse() {
                        Err(_) => msg,
                        Ok(addr) => msg.cc(addr),
                    },
                    "bcc" => match h.get_value().parse() {
                        Err(_) => msg,
                        Ok(addr) => msg.bcc(addr),
                    },
                    "subject" => msg.subject(h.get_value()),
                    _ => msg,
                }
            })
            .singlepart(
                SinglePart::builder()
                    .header(ContentType("text/plain; charset=utf-8".parse().unwrap()))
                    .header(ContentTransferEncoding::Base64)
                    .body(parsed_msg.get_body_raw()?),
            )?;

        Ok(Msg(built_msg))
    }

    pub fn as_sendable_msg(&self) -> &lettre::Message {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.formatted()
    }
}
