use ammonia;
use anyhow::{anyhow, Context, Error, Result};
use chrono::{DateTime, FixedOffset};
use htmlescape;
use imap::types::Flag;
use lettre::message::{Attachment, MultiPart, SinglePart};
use regex::Regex;
use rfc2047_decoder;
use serde::Serialize;
use std::{
    convert::{TryFrom, TryInto},
    fmt, fs,
    path::PathBuf,
};

use crate::{
    config::Account,
    domain::{
        imap::ImapServiceInterface,
        mbox::Mbox,
        msg::{msg_utils, Flags, Parts, TextHtmlPart, TextPlainPart, Tpl, TplOverride},
        smtp::SmtpServiceInterface,
    },
    output::OutputServiceInterface,
    ui::{
        choice::{self, PostEditChoice, PreEditChoice},
        editor,
    },
};

use super::{BinaryPart, Part};

type Addr = lettre::message::Mailbox;

/// Representation of a message.
#[derive(Debug, Default)]
pub struct Msg {
    /// The sequence number of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.1.2
    pub id: u32,

    /// The flags attached to the message.
    pub flags: Flags,

    /// The subject of the message.
    pub subject: String,

    pub from: Option<Vec<Addr>>,
    pub reply_to: Option<Vec<Addr>>,
    pub to: Option<Vec<Addr>>,
    pub cc: Option<Vec<Addr>>,
    pub bcc: Option<Vec<Addr>>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,

    /// The internal date of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.3
    pub date: Option<DateTime<FixedOffset>>,
    pub parts: Parts,
}

impl Msg {
    pub fn attachments(&self) -> Vec<BinaryPart> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Binary(part) => Some(part.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn join_text_plain_parts(&self) -> String {
        let text_parts = self
            .parts
            .iter()
            .filter_map(|part| match part {
                Part::TextPlain(part) => Some(part.content.to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let text_parts = ammonia::Builder::new()
            .tags(Default::default())
            .clean(&text_parts)
            .to_string();
        let text_parts = match htmlescape::decode_html(&text_parts) {
            Ok(text_parts) => text_parts,
            Err(_) => text_parts,
        };
        text_parts
    }

    pub fn join_text_html_parts(&self) -> String {
        let text_parts = self
            .parts
            .iter()
            .filter_map(|part| match part {
                Part::TextPlain(part) => Some(part.content.to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let text_parts = Regex::new(r"(\r?\n){2,}")
            .unwrap()
            .replace_all(&text_parts, "\n\n")
            .to_string();
        text_parts
    }

    pub fn join_text_parts(&self) -> String {
        let text_parts = self.join_text_plain_parts();
        if text_parts.is_empty() {
            self.join_text_html_parts()
        } else {
            text_parts
        }
    }

    pub fn into_reply(mut self, all: bool, account: &Account) -> Result<Self> {
        let account_addr: Addr = account.address().parse()?;

        // Message-Id
        self.message_id = None;

        // In-Reply-To
        self.in_reply_to = self.message_id.to_owned();

        // From
        self.from = Some(vec![account_addr.to_owned()]);

        // To
        let addrs = self
            .reply_to
            .as_ref()
            .or_else(|| self.from.as_ref())
            .map(|addrs| {
                addrs
                    .clone()
                    .into_iter()
                    .filter(|addr| addr != &account_addr)
            });
        if all {
            self.to = addrs.map(|addrs| addrs.collect());
        } else {
            self.to = addrs
                .and_then(|mut addrs| addrs.next())
                .map(|addr| vec![addr]);
        }

        // Cc & Bcc
        if !all {
            self.cc = None;
            self.bcc = None;
        }

        // Subject
        if !self.subject.starts_with("Re:") {
            self.subject = format!("Re: {}", self.subject);
        }

        // plaintext and html content
        let date = self
            .date
            .as_ref()
            .map(|date| date.format("%d %b %Y, at %H:%M").to_string())
            .unwrap_or("unknown date".into());
        let sender = self
            .reply_to
            .as_ref()
            .or(self.from.as_ref())
            .and_then(|addrs| addrs.first())
            .map(|addr| addr.name.to_owned().unwrap_or(addr.email.to_string()))
            .unwrap_or("unknown sender".into());

        let mut html_content = format!("\n\nOn {}, {} wrote:\n", date, sender);
        let mut plaintext_content = html_content.clone();

        let mut glue = "";
        for plaintext_line in self.join_text_plain_parts().trim().lines() {
            if plaintext_line == "-- \n" {
                break;
            }
            plaintext_content.push_str(glue);
            plaintext_content.push_str(">");
            plaintext_content.push_str(if plaintext_line.starts_with(">") { "" } else { " " });
            plaintext_content.push_str(plaintext_line);
            glue = "\n";
        }
        for html_line in self.join_text_html_parts().trim().lines() {
            if html_line == "-- \n" {
                break;
            }
            html_content.push_str(glue);
            html_content.push_str(">");
            html_content.push_str(if html_line.starts_with(">") { "" } else { " " });
            html_content.push_str(html_line);
            glue = "\n";
        }

        self.parts = Parts::default();

        if !plaintext_content.is_empty() {
            self.parts.push(Part::TextPlain(TextPlainPart {
                content: plaintext_content,
            }));
        }

        if !html_content.is_empty() {
            self.parts.push(Part::TextHtml(TextHtmlPart {
                content: html_content,
            }));
        }

        Ok(self)
    }

    pub fn into_forward(mut self, account: &Account) -> Result<Self> {
        let account_addr: Addr = account.address().parse()?;

        let prev_subject = self.subject.to_owned();
        let prev_date = self.date.to_owned();
        let prev_from = self.reply_to.to_owned().or_else(|| self.from.to_owned());
        let prev_to = self.to.to_owned();

        // Message-Id
        self.message_id = None;

        // In-Reply-To
        self.in_reply_to = None;

        // From
        self.from = Some(vec![account_addr.to_owned()]);

        // To
        self.to = Some(vec![]);

        // Cc
        self.cc = None;

        // Bcc
        self.bcc = None;

        // Subject
        if !self.subject.starts_with("Fwd:") {
            self.subject = format!("Fwd: {}", self.subject);
        }

        // Content for the plaintext and html parts
        let mut content = String::default();
        content.push_str("\n\n-------- Forwarded Message --------\n");
        content.push_str(&format!("Subject: {}\n", prev_subject));
        if let Some(date) = prev_date {
            content.push_str(&format!("Date: {}\n", date.to_rfc2822()));
        }
        if let Some(addrs) = prev_from.as_ref() {
            content.push_str("From: ");
            let mut glue = "";
            for addr in addrs {
                content.push_str(glue);
                content.push_str(&addr.to_string());
                glue = ", ";
            }
            content.push_str("\n");
        }
        if let Some(addrs) = prev_to.as_ref() {
            content.push_str("To: ");
            let mut glue = "";
            for addr in addrs {
                content.push_str(glue);
                content.push_str(&addr.to_string());
                glue = ", ";
            }
            content.push_str("\n");
        }
        content.push_str("\n");
        
        let mut html_content = content.clone();
        let mut plaintext_content = content;

        html_content.push_str(&self.join_text_html_parts());
        plaintext_content.push_str(&self.join_text_plain_parts());

        self.parts.replace_text_html_parts_with(TextHtmlPart { content: html_content });
        self.parts.replace_text_plain_parts_with(TextPlainPart { content: plaintext_content});

        Ok(self)
    }

    fn _edit_with_editor(&self, account: &Account) -> Result<Self> {
        let tpl = Tpl::from_msg(TplOverride::default(), self, account);
        let tpl = editor::open_with_tpl(tpl)?;
        Self::try_from(&tpl)
    }

    pub fn edit_with_editor<
        OutputService: OutputServiceInterface,
        ImapService: ImapServiceInterface,
        SmtpService: SmtpServiceInterface,
    >(
        mut self,
        account: &Account,
        output: &OutputService,
        imap: &mut ImapService,
        smtp: &mut SmtpService,
    ) -> Result<()> {
        let draft = msg_utils::local_draft_path();
        if draft.exists() {
            loop {
                match choice::pre_edit() {
                    Ok(choice) => match choice {
                        PreEditChoice::Edit => {
                            let tpl = editor::open_with_draft()?;
                            self.merge_with(Msg::try_from(&tpl)?);
                            break;
                        }
                        PreEditChoice::Discard => {
                            self.merge_with(self._edit_with_editor(account)?);
                            break;
                        }
                        PreEditChoice::Quit => return Ok(()),
                    },
                    Err(err) => {
                        println!("{}", err);
                        continue;
                    }
                }
            }
        } else {
            self.merge_with(self._edit_with_editor(account)?);
        }

        loop {
            match choice::post_edit() {
                Ok(PostEditChoice::Send) => {
                    let mbox = Mbox::from("Sent");
                    let sent_msg = smtp.send_msg(&self)?;
                    let flags = Flags::try_from(vec![Flag::Seen])?;
                    imap.append_raw_msg_with_flags(&mbox, &sent_msg.formatted(), flags)?;
                    msg_utils::remove_local_draft()?;
                    output.print("Message successfully sent")?;
                    break;
                }
                Ok(PostEditChoice::Edit) => {
                    self.merge_with(self._edit_with_editor(account)?);
                }
                Ok(PostEditChoice::LocalDraft) => {
                    output.print("Message successfully saved locally")?;
                    break;
                }
                Ok(PostEditChoice::RemoteDraft) => {
                    let mbox = Mbox::from("Drafts");
                    let flags = Flags::try_from(vec![Flag::Seen, Flag::Draft])?;
                    let tpl = Tpl::from_msg(TplOverride::default(), &self, account);
                    imap.append_raw_msg_with_flags(&mbox, tpl.as_bytes(), flags)?;
                    msg_utils::remove_local_draft()?;
                    output.print("Message successfully saved to Drafts")?;
                    break;
                }
                Ok(PostEditChoice::Discard) => {
                    msg_utils::remove_local_draft()?;
                    break;
                }
                Err(err) => {
                    println!("{}", err);
                }
            }
        }

        Ok(())
    }

    pub fn add_attachments(mut self, attachments_paths: Vec<&str>) -> Result<Self> {
        for path in attachments_paths {
            let path = shellexpand::full(path)
                .context(format!(r#"cannot expand attachment path "{}""#, path))?;
            let path = PathBuf::from(path.to_string());
            let filename: String = path
                .file_name()
                .ok_or(anyhow!("cannot get file name of attachment {:?}", path))?
                .to_string_lossy()
                .into();
            let content = fs::read(&path).context(format!("cannot read attachment {:?}", path))?;
            let mime = tree_magic::from_u8(&content);

            self.parts.push(Part::Binary(BinaryPart {
                filename,
                mime,
                content,
            }))
        }

        Ok(self)
    }

    pub fn merge_with(&mut self, msg: Msg) {
        if msg.from.is_some() {
            self.from = msg.from;
        }

        if msg.to.is_some() {
            self.to = msg.to;
        }

        if msg.cc.is_some() {
            self.cc = msg.cc;
        }

        if msg.bcc.is_some() {
            self.bcc = msg.bcc;
        }

        if !msg.subject.is_empty() {
            self.subject = msg.subject;
        }

        for part in msg.parts.0.into_iter() {
            match part {
                Part::Binary(_) => self.parts.push(part),
                Part::TextPlain(_) => {
                    self.parts.retain(|p| match p {
                        Part::TextPlain(_) => false,
                        _ => true,
                    });
                    self.parts.push(part);
                }
                Part::TextHtml(_) => {
                    self.parts.retain(|p| match p {
                        Part::TextHtml(_) => false,
                        _ => true,
                    });
                    self.parts.push(part);
                }
            }
        }
    }
}

impl TryFrom<&Tpl> for Msg {
    type Error = Error;

    fn try_from(tpl: &Tpl) -> Result<Msg> {
        let mut msg = Msg::default();

        let parsed_msg =
            mailparse::parse_mail(tpl.as_bytes()).context("cannot parse message from template")?;

        for header in parsed_msg.get_headers() {
            let key = header.get_key();
            let val = String::from_utf8(header.get_value_raw().to_vec())
                .map(|val| val.trim().to_string())?;

            match key.as_str() {
                "Message-Id" | _ if key.eq_ignore_ascii_case("message-id") => {
                    msg.message_id = Some(val.to_owned())
                }
                "From" | _ if key.eq_ignore_ascii_case("from") => {
                    msg.from = Some(
                        val.split(',')
                            .filter_map(|addr| addr.parse().ok())
                            .collect::<Vec<_>>(),
                    );
                }
                "To" | _ if key.eq_ignore_ascii_case("to") => {
                    msg.to = Some(
                        val.split(',')
                            .filter_map(|addr| addr.parse().ok())
                            .collect::<Vec<_>>(),
                    );
                }
                "Reply-To" | _ if key.eq_ignore_ascii_case("reply-to") => {
                    msg.reply_to = Some(
                        val.split(',')
                            .filter_map(|addr| addr.parse().ok())
                            .collect::<Vec<_>>(),
                    );
                }
                "In-Reply-To" | _ if key.eq_ignore_ascii_case("in-reply-to") => {
                    msg.in_reply_to = Some(val.to_owned())
                }
                "Cc" | _ if key.eq_ignore_ascii_case("cc") => {
                    msg.cc = Some(
                        val.split(',')
                            .filter_map(|addr| addr.parse().ok())
                            .collect::<Vec<_>>(),
                    );
                }
                "Bcc" | _ if key.eq_ignore_ascii_case("bcc") => {
                    msg.bcc = Some(
                        val.split(',')
                            .filter_map(|addr| addr.parse().ok())
                            .collect::<Vec<_>>(),
                    );
                }
                "Subject" | _ if key.eq_ignore_ascii_case("subject") => {
                    msg.subject = val;
                }
                _ => (),
            }
        }

        let content = parsed_msg
            .get_body_raw()
            .context("cannot get body from parsed message")?;
        let content = String::from_utf8(content).context("cannot decode body from utf-8")?;
        msg.parts.push(Part::TextPlain(TextPlainPart { content }));

        Ok(msg)
    }
}

impl TryInto<lettre::address::Envelope> for Msg {
    type Error = Error;

    fn try_into(self) -> Result<lettre::address::Envelope> {
        let from: Option<lettre::Address> = self
            .from
            .and_then(|addrs| addrs.into_iter().next())
            .map(|addr| addr.email);
        let to = self
            .to
            .map(|addrs| addrs.into_iter().map(|addr| addr.email).collect())
            .unwrap_or_default();
        let envelope =
            lettre::address::Envelope::new(from, to).context("cannot create envelope")?;

        Ok(envelope)
    }
}

impl TryInto<lettre::Message> for &Msg {
    type Error = Error;

    fn try_into(self) -> Result<lettre::Message> {
        let mut msg_builder = lettre::Message::builder()
            .message_id(self.message_id.to_owned())
            .subject(self.subject.to_owned());

        if let Some(id) = self.in_reply_to.as_ref() {
            msg_builder = msg_builder.in_reply_to(id.to_owned());
        };

        if let Some(addrs) = self.from.as_ref() {
            msg_builder = addrs
                .iter()
                .fold(msg_builder, |builder, addr| builder.from(addr.to_owned()))
        };

        if let Some(addrs) = self.to.as_ref() {
            msg_builder = addrs
                .iter()
                .fold(msg_builder, |builder, addr| builder.to(addr.to_owned()))
        };

        if let Some(addrs) = self.reply_to.as_ref() {
            msg_builder = addrs.iter().fold(msg_builder, |builder, addr| {
                builder.reply_to(addr.to_owned())
            })
        };

        if let Some(addrs) = self.cc.as_ref() {
            msg_builder = addrs
                .iter()
                .fold(msg_builder, |builder, addr| builder.cc(addr.to_owned()))
        };

        if let Some(addrs) = self.bcc.as_ref() {
            msg_builder = addrs
                .iter()
                .fold(msg_builder, |builder, addr| builder.bcc(addr.to_owned()))
        };

        let mut multipart =
            MultiPart::mixed().singlepart(SinglePart::plain(self.join_text_plain_parts()));

        for part in self.attachments() {
            let filename = part.filename;
            let content = part.content;
            let mime = part.mime.parse().context(format!(
                r#"cannot parse content type of attachment "{}""#,
                filename
            ))?;
            multipart = multipart.singlepart(Attachment::new(filename).body(content, mime))
        }

        msg_builder
            .multipart(multipart)
            .context("cannot build sendable message")
    }
}

impl TryInto<Vec<u8>> for &Msg {
    type Error = Error;

    fn try_into(self) -> Result<Vec<u8>> {
        let msg: lettre::Message = self.try_into()?;
        Ok(msg.formatted())
    }
}

impl<'a> TryFrom<&'a imap::types::Fetch> for Msg {
    type Error = Error;

    fn try_from(fetch: &'a imap::types::Fetch) -> Result<Msg> {
        let envelope = fetch
            .envelope()
            .ok_or(anyhow!("cannot get envelope of message {}", fetch.message))?;

        // Get the sequence number
        let id = fetch.message;

        // Get the flags
        let flags = Flags::try_from(fetch.flags())?;

        // Get the subject
        let subject = envelope
            .subject
            .as_ref()
            .ok_or(anyhow!("cannot get subject of message {}", fetch.message))
            .and_then(|subj| {
                rfc2047_decoder::decode(subj).context(format!(
                    "cannot decode subject of message {}",
                    fetch.message
                ))
            })?;

        // Get the sender(s) address(es)
        let from = match envelope
            .sender
            .as_ref()
            .or_else(|| envelope.from.as_ref())
            .map(parse_addrs)
        {
            Some(addrs) => Some(addrs?),
            None => None,
        };

        // Get the "Reply-To" address(es)
        let reply_to = parse_some_addrs(&envelope.reply_to).context(format!(
            r#"cannot parse "reply to" address of message {}"#,
            id
        ))?;

        // Get the recipient(s) address(es)
        let to = parse_some_addrs(&envelope.to)
            .context(format!(r#"cannot parse "to" address of message {}"#, id))?;

        // Get the "Cc" recipient(s) address(es)
        let cc = parse_some_addrs(&envelope.cc)
            .context(format!(r#"cannot parse "cc" address of message {}"#, id))?;

        // Get the "Bcc" recipient(s) address(es)
        let bcc = parse_some_addrs(&envelope.bcc)
            .context(format!(r#"cannot parse "bcc" address of message {}"#, id))?;

        // Get the "In-Reply-To" message identifier
        let in_reply_to = match envelope
            .in_reply_to
            .as_ref()
            .map(|cow| String::from_utf8(cow.to_vec()))
        {
            Some(id) => Some(id?),
            None => None,
        };

        // Get the message identifier
        let message_id = match envelope
            .message_id
            .as_ref()
            .map(|cow| String::from_utf8(cow.to_vec()))
        {
            Some(id) => Some(id?),
            None => None,
        };

        // Get the internal date
        let date = fetch.internal_date();

        // Get all parts
        let parts = Parts::from(
            &mailparse::parse_mail(
                fetch
                    .body()
                    .ok_or(anyhow!("cannot get body of message {}", id))?,
            )
            .context(format!("cannot parse body of message {}", id))?,
        );

        Ok(Self {
            id,
            flags,
            subject,
            message_id,
            from,
            reply_to,
            in_reply_to,
            to,
            cc,
            bcc,
            date,
            parts,
        })
    }
}

pub fn parse_addr(addr: &imap_proto::Address) -> Result<Addr> {
    let name = addr
        .name
        .as_ref()
        .map(|name| {
            rfc2047_decoder::decode(&name.to_vec())
                .context("cannot decode address name")
                .map(|name| Some(name))
        })
        .unwrap_or(Ok(None))?;
    let mbox = addr
        .mailbox
        .as_ref()
        .ok_or(anyhow!("cannot get address mailbox"))
        .and_then(|mbox| {
            rfc2047_decoder::decode(&mbox.to_vec()).context("cannot decode address mailbox")
        })?;
    let host = addr
        .host
        .as_ref()
        .ok_or(anyhow!("cannot get address host"))
        .and_then(|host| {
            rfc2047_decoder::decode(&host.to_vec()).context("cannot decode address host")
        })?;

    Ok(Addr::new(name, lettre::Address::new(mbox, host)?))
}

pub fn parse_addrs(addrs: &Vec<imap_proto::Address>) -> Result<Vec<Addr>> {
    let mut parsed_addrs = vec![];
    for addr in addrs {
        parsed_addrs
            .push(parse_addr(addr).context(format!(r#"cannot parse address "{:?}""#, addr))?);
    }
    Ok(parsed_addrs)
}

pub fn parse_some_addrs(addrs: &Option<Vec<imap_proto::Address>>) -> Result<Option<Vec<Addr>>> {
    Ok(match addrs.as_ref().map(parse_addrs) {
        Some(addrs) => Some(addrs?),
        None => None,
    })
}

#[derive(Debug, Serialize)]
pub struct PrintableMsg(pub String);

impl fmt::Display for PrintableMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.0)
    }
}
