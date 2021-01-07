use imap;
use mailparse::{self, MailHeaderMap};
use rfc2047_decoder;

use crate::table::{self, DisplayCell, DisplayRow, DisplayTable};

#[derive(Debug)]
pub struct Uid(pub u32);

impl Uid {
    pub fn from_fetch(fetch: &imap::types::Fetch) -> Self {
        Self(fetch.uid.unwrap())
    }
}

impl DisplayCell for Uid {
    fn styles(&self) -> &[table::Style] {
        &[table::RED]
    }

    fn value(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug)]
pub struct Flags<'a>(Vec<imap::types::Flag<'a>>);

impl Flags<'_> {
    pub fn from_fetch(fetch: &imap::types::Fetch) -> Self {
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

        Self(flags)
    }
}

impl DisplayCell for Flags<'_> {
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

#[derive(Debug)]
pub struct Sender(String);

impl Sender {
    fn try_from_fetch(fetch: &imap::types::Fetch) -> Option<String> {
        let addr = fetch.envelope()?.from.as_ref()?.first()?;

        addr.name
            .and_then(|bytes| rfc2047_decoder::decode(bytes).ok())
            .or_else(|| {
                let mbox = String::from_utf8(addr.mailbox?.to_vec()).ok()?;
                let host = String::from_utf8(addr.host?.to_vec()).ok()?;
                Some(format!("{}@{}", mbox, host))
            })
    }

    pub fn from_fetch(fetch: &imap::types::Fetch) -> Self {
        Self(Self::try_from_fetch(fetch).unwrap_or(String::new()))
    }
}

impl DisplayCell for Sender {
    fn styles(&self) -> &[table::Style] {
        &[table::BLUE]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

#[derive(Debug)]
pub struct Subject(String);

impl Subject {
    fn try_from_fetch(fetch: &imap::types::Fetch) -> Option<String> {
        fetch
            .envelope()?
            .subject
            .and_then(|bytes| rfc2047_decoder::decode(bytes).ok())
            .and_then(|subject| Some(subject.replace("\r", "")))
            .and_then(|subject| Some(subject.replace("\n", "")))
    }

    pub fn from_fetch(fetch: &imap::types::Fetch) -> Self {
        Self(Self::try_from_fetch(fetch).unwrap_or(String::new()))
    }
}

impl DisplayCell for Subject {
    fn styles(&self) -> &[table::Style] {
        &[table::GREEN]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

#[derive(Debug)]
pub struct Date(String);

impl Date {
    fn try_from_fetch(fetch: &imap::types::Fetch) -> Option<String> {
        fetch
            .internal_date()
            .and_then(|date| Some(date.to_rfc3339()))
    }

    pub fn from_fetch(fetch: &imap::types::Fetch) -> Self {
        Self(Self::try_from_fetch(fetch).unwrap_or(String::new()))
    }
}

impl DisplayCell for Date {
    fn styles(&self) -> &[table::Style] {
        &[table::YELLOW]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

#[derive(Debug)]
pub struct Email<'a> {
    pub uid: Uid,
    pub flags: Flags<'a>,
    pub from: Sender,
    pub subject: Subject,
    pub date: Date,
}

impl Email<'_> {
    pub fn from_fetch(fetch: &imap::types::Fetch) -> Self {
        Self {
            uid: Uid::from_fetch(fetch),
            from: Sender::from_fetch(fetch),
            subject: Subject::from_fetch(fetch),
            date: Date::from_fetch(fetch),
            flags: Flags::from_fetch(fetch),
        }
    }
}

impl<'a> DisplayRow for Email<'a> {
    fn to_row(&self) -> Vec<table::Cell> {
        vec![
            self.uid.to_cell(),
            self.flags.to_cell(),
            self.from.to_cell(),
            self.subject.to_cell(),
            self.date.to_cell(),
        ]
    }
}

impl<'a> DisplayTable<'a, Email<'a>> for Vec<Email<'a>> {
    fn cols() -> &'a [&'a str] {
        &["uid", "flags", "from", "subject", "date"]
    }

    fn rows(&self) -> &Vec<Email<'a>> {
        self
    }
}

// Utils

fn extract_text_bodies_into(mime: &str, part: &mailparse::ParsedMail, parts: &mut Vec<String>) {
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
                .for_each(|part| extract_text_bodies_into(mime, part, parts));
        }
    }
}

pub fn extract_text_bodies(mime: &str, email: &mailparse::ParsedMail) -> String {
    let mut parts = vec![];
    extract_text_bodies_into(mime, email, &mut parts);
    parts.join("\r\n")
}
