use imap;
use mailparse::{self, MailHeaderMap};
use native_tls::{self, TlsConnector, TlsStream};
use rfc2047_decoder;
use std::{error, fmt, net::TcpStream, result};

use crate::config;
use crate::table;

// Email

pub struct Uid(u32);

impl table::DisplayCell for Uid {
    fn styles(&self) -> &[table::Style] {
        &[table::RED]
    }

    fn value(&self) -> String {
        self.0.to_string()
    }
}

pub struct Flags<'a>(Vec<imap::types::Flag<'a>>);

impl table::DisplayCell for Flags<'_> {
    fn styles(&self) -> &[table::Style] {
        &[table::WHITE]
    }

    fn value(&self) -> String {
        use imap::types::Flag::*;

        let Flags(flags) = self;
        let mut flags_str = String::new();

        flags_str.push_str(if !flags.contains(&Seen) { &"N" } else { &" " });
        flags_str.push_str(if flags.contains(&Answered) {
            &"R"
        } else {
            &" "
        });
        flags_str.push_str(if flags.contains(&Draft) { &"D" } else { &" " });
        flags_str.push_str(if flags.contains(&Flagged) { &"F" } else { &" " });

        flags_str
    }
}

pub struct Sender(String);

impl table::DisplayCell for Sender {
    fn styles(&self) -> &[table::Style] {
        &[table::BLUE]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

pub struct Subject(String);

impl table::DisplayCell for Subject {
    fn styles(&self) -> &[table::Style] {
        &[table::GREEN]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

pub struct Date(String);

impl table::DisplayCell for Date {
    fn styles(&self) -> &[table::Style] {
        &[table::YELLOW]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

pub struct Email<'a> {
    uid: Uid,
    flags: Flags<'a>,
    from: Sender,
    subject: Subject,
    date: Date,
}

impl Email<'_> {
    fn first_sender_from_fetch(fetch: &imap::types::Fetch) -> Option<String> {
        let addr = fetch.envelope()?.from.as_ref()?.first()?;

        addr.name
            .and_then(|bytes| rfc2047_decoder::decode(bytes).ok())
            .or_else(|| {
                let mbox = String::from_utf8(addr.mailbox?.to_vec()).ok()?;
                let host = String::from_utf8(addr.host?.to_vec()).ok()?;
                Some(format!("{}@{}", mbox, host))
            })
    }

    fn subject_from_fetch(fetch: &imap::types::Fetch) -> Option<String> {
        fetch
            .envelope()?
            .subject
            .and_then(|bytes| rfc2047_decoder::decode(bytes).ok())
            .and_then(|subject| Some(subject.replace("\r", "")))
            .and_then(|subject| Some(subject.replace("\n", "")))
    }

    fn date_from_fetch(fetch: &imap::types::Fetch) -> Option<String> {
        fetch
            .internal_date()
            .and_then(|date| Some(date.to_rfc3339()))
    }
}

impl<'a> table::DisplayRow for Email<'a> {
    fn to_row(&self) -> Vec<table::Cell> {
        use table::DisplayCell;

        vec![
            self.uid.to_cell(),
            self.flags.to_cell(),
            self.from.to_cell(),
            self.subject.to_cell(),
            self.date.to_cell(),
        ]
    }
}

impl<'a> table::DisplayTable<'a, Email<'a>> for Vec<Email<'a>> {
    fn cols() -> &'a [&'a str] {
        &["uid", "flags", "from", "subject", "date"]
    }

    fn rows(&self) -> &Vec<Email<'a>> {
        self
    }
}

// IMAP Connector

#[derive(Debug)]
pub struct ImapConnector {
    pub config: config::ServerInfo,
    pub sess: imap::Session<TlsStream<TcpStream>>,
}

impl ImapConnector {
    pub fn new(config: config::ServerInfo) -> Result<Self> {
        let tls = TlsConnector::new()?;
        let client = imap::connect(config.get_addr(), &config.host, &tls)?;
        let sess = client
            .login(&config.login, &config.password)
            .map_err(|res| res.0)?;

        Ok(Self { config, sess })
    }

    pub fn read_emails(&mut self, mbox: &str, query: &str) -> Result<Vec<Email<'_>>> {
        self.sess.select(mbox)?;

        let uids = self
            .sess
            .uid_search(query)?
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();

        let emails = self
            .sess
            .uid_fetch(
                uids[..20.min(uids.len())].join(","),
                "(UID ENVELOPE INTERNALDATE)",
            )?
            .iter()
            .map(|fetch| {
                let flags = fetch.flags().iter().fold(vec![], |mut flags, flag| {
                    use imap::types::Flag::*;

                    match flag {
                        Seen => flags.push(Seen),
                        Answered => flags.push(Answered),
                        Draft => flags.push(Draft),
                        Flagged => flags.push(Flagged),
                        _ => (),
                    };

                    flags
                });

                Email {
                    uid: Uid(fetch.uid.unwrap()),
                    from: Sender(Email::first_sender_from_fetch(fetch).unwrap_or(String::new())),
                    subject: Subject(Email::subject_from_fetch(fetch).unwrap_or(String::new())),
                    date: Date(Email::date_from_fetch(fetch).unwrap_or(String::new())),
                    flags: Flags(flags),
                }
            })
            .collect::<Vec<_>>();

        Ok(emails)
    }
}

// Error wrapper

#[derive(Debug)]
pub enum Error {
    CreateTlsConnectorError(native_tls::Error),
    CreateImapSession(imap::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CreateTlsConnectorError(err) => err.fmt(f),
            Error::CreateImapSession(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::CreateTlsConnectorError(ref err) => Some(err),
            Error::CreateImapSession(ref err) => Some(err),
        }
    }
}

impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Error {
        Error::CreateTlsConnectorError(err)
    }
}

impl From<imap::Error> for Error {
    fn from(err: imap::Error) -> Error {
        Error::CreateImapSession(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// pub fn list_mailboxes(imap_sess: &mut ImapSession) -> imap::Result<()> {
//     let mboxes = imap_sess.list(Some(""), Some("*"))?;

//     let table_head = vec![
//         table::Cell::new(
//             vec![table::BOLD, table::UNDERLINE, table::WHITE],
//             String::from("DELIM"),
//         ),
//         table::Cell::new(
//             vec![table::BOLD, table::UNDERLINE, table::WHITE],
//             String::from("NAME"),
//         ),
//         table::Cell::new(
//             vec![table::BOLD, table::UNDERLINE, table::WHITE],
//             String::from("ATTRIBUTES"),
//         ),
//     ];

//     let mut table_rows = mboxes
//         .iter()
//         .map(|mbox| {
//             vec![
//                 table::Cell::new(
//                     vec![table::BLUE],
//                     mbox.delimiter().unwrap_or("").to_string(),
//                 ),
//                 table::Cell::new(vec![table::GREEN], mbox.name().to_string()),
//                 table::Cell::new(
//                     vec![table::YELLOW],
//                     mbox.attributes()
//                         .iter()
//                         .map(|a| format!("{:?}", a))
//                         .collect::<Vec<_>>()
//                         .join(", "),
//                 ),
//             ]
//         })
//         .collect::<Vec<_>>();

//     if table_rows.len() == 0 {
//         println!("No email found!");
//     } else {
//         table_rows.insert(0, table_head);
//         println!("{}", table::render(table_rows));
//     }

//     Ok(())
// }

fn extract_subparts_by_mime(mime: &str, part: &mailparse::ParsedMail, parts: &mut Vec<String>) {
    match part.subparts.len() {
        0 => {
            if part
                .get_headers()
                .get_first_value("content-type")
                .and_then(|v| if v.starts_with(mime) { Some(()) } else { None })
                .is_some()
            {
                parts.push(part.get_body().unwrap_or(String::new()))
            }
        }
        _ => {
            part.subparts
                .iter()
                .for_each(|p| extract_subparts_by_mime(mime, p, parts));
        }
    }
}

// pub fn read_email(
//     imap_sess: &mut ImapSession,
//     mbox: &str,
//     uid: &str,
//     mime: &str,
// ) -> imap::Result<()> {
//     imap_sess.select(mbox)?;

//     match imap_sess.uid_fetch(uid, "BODY[]")?.first() {
//         None => println!("No email found in mailbox {} with UID {}", mbox, uid),
//         Some(email_raw) => {
//             let email = mailparse::parse_mail(email_raw.body().unwrap_or(&[])).unwrap();
//             let mut parts = vec![];
//             extract_subparts_by_mime(mime, &email, &mut parts);

//             if parts.len() == 0 {
//                 println!("No {} content found for email {}!", mime, uid);
//             } else {
//                 println!("{}", parts.join("\r\n"));
//             }
//         }
//     }

//     Ok(())
// }
