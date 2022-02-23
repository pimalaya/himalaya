use ammonia;
use anyhow::{anyhow, Context, Error, Result};
use chrono::{DateTime, FixedOffset};
use html_escape;
use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use log::{debug, info, trace};
use regex::Regex;
use std::{collections::HashSet, convert::TryInto, env::temp_dir, fmt::Debug, fs, path::PathBuf};
use uuid::Uuid;

use crate::{
    backends::Backend,
    config::{AccountConfig, DEFAULT_SIG_DELIM},
    msg::{
        from_addrs_to_sendable_addrs, from_addrs_to_sendable_mbox, from_slice_to_addrs, msg_utils,
        Addrs, BinaryPart, Part, Parts, TextPlainPart, TplOverride,
    },
    output::PrinterService,
    smtp::SmtpService,
    ui::{
        choice::{self, PostEditChoice, PreEditChoice},
        editor,
    },
};

/// Representation of a message.
#[derive(Debug, Default)]
pub struct Msg {
    /// The sequence number of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.1.2
    pub id: u32,

    /// The subject of the message.
    pub subject: String,

    pub from: Option<Addrs>,
    pub reply_to: Option<Addrs>,
    pub to: Option<Addrs>,
    pub cc: Option<Addrs>,
    pub bcc: Option<Addrs>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,

    /// The internal date of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.3
    pub date: Option<DateTime<FixedOffset>>,
    pub parts: Parts,

    pub encrypt: bool,

    pub raw: Vec<u8>,
}

impl Msg {
    pub fn attachments(&self) -> Vec<BinaryPart> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Binary(part) => Some(part.to_owned()),
                _ => None,
            })
            .collect()
    }

    /// Folds string body from all plain text parts into a single string body. If no plain text
    /// parts are found, HTML parts are used instead. The result is sanitized (all HTML markup is
    /// removed).
    pub fn fold_text_plain_parts(&self) -> String {
        let (plain, html) = self.parts.iter().fold(
            (String::default(), String::default()),
            |(mut plain, mut html), part| {
                match part {
                    Part::TextPlain(part) => {
                        let glue = if plain.is_empty() { "" } else { "\n\n" };
                        plain.push_str(glue);
                        plain.push_str(&part.content);
                    }
                    Part::TextHtml(part) => {
                        let glue = if html.is_empty() { "" } else { "\n\n" };
                        html.push_str(glue);
                        html.push_str(&part.content);
                    }
                    _ => (),
                };
                (plain, html)
            },
        );
        if plain.is_empty() {
            // Remove HTML markup
            let sanitized_html = ammonia::Builder::new()
                .tags(HashSet::default())
                .clean(&html)
                .to_string();
            // Merge new line chars
            let sanitized_html = Regex::new(r"(\r?\n\s*){2,}")
                .unwrap()
                .replace_all(&sanitized_html, "\n\n")
                .to_string();
            // Replace tabulations and &npsp; by spaces
            let sanitized_html = Regex::new(r"(\t|&nbsp;)")
                .unwrap()
                .replace_all(&sanitized_html, " ")
                .to_string();
            // Merge spaces
            let sanitized_html = Regex::new(r" {2,}")
                .unwrap()
                .replace_all(&sanitized_html, "  ")
                .to_string();
            // Decode HTML entities
            let sanitized_html = html_escape::decode_html_entities(&sanitized_html).to_string();

            sanitized_html
        } else {
            // Merge new line chars
            let sanitized_plain = Regex::new(r"(\r?\n\s*){2,}")
                .unwrap()
                .replace_all(&plain, "\n\n")
                .to_string();
            // Replace tabulations by spaces
            let sanitized_plain = Regex::new(r"\t")
                .unwrap()
                .replace_all(&sanitized_plain, " ")
                .to_string();
            // Merge spaces
            let sanitized_plain = Regex::new(r" {2,}")
                .unwrap()
                .replace_all(&sanitized_plain, "  ")
                .to_string();

            sanitized_plain
        }
    }

    /// Fold string body from all HTML parts into a single string body.
    fn fold_text_html_parts(&self) -> String {
        let text_parts = self
            .parts
            .iter()
            .filter_map(|part| match part {
                Part::TextHtml(part) => Some(part.content.to_owned()),
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

    /// Fold string body from all text parts into a single string body. The mime allows users to
    /// choose between plain text parts and html text parts.
    pub fn fold_text_parts(&self, text_mime: &str) -> String {
        if text_mime == "html" {
            self.fold_text_html_parts()
        } else {
            self.fold_text_plain_parts()
        }
    }

    pub fn into_reply(mut self, all: bool, account: &AccountConfig) -> Result<Self> {
        let account_addr = account.address()?;

        // Message-Id
        self.message_id = None;

        // In-Reply-To
        self.in_reply_to = self.message_id.to_owned();

        // From
        self.from = Some(vec![account_addr.clone()].into());

        // To
        let addrs = self
            .reply_to
            .as_deref()
            .or_else(|| self.from.as_deref())
            .map(|addrs| {
                addrs
                    .clone()
                    .into_iter()
                    .filter(|addr| addr != &account_addr)
            });
        if all {
            self.to = addrs.map(|addrs| addrs.collect::<Vec<_>>().into());
        } else {
            self.to = addrs
                .and_then(|mut addrs| addrs.next())
                .map(|addr| vec![addr].into());
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

        // Body
        let plain_content = {
            let date = self
                .date
                .as_ref()
                .map(|date| date.format("%d %b %Y, at %H:%M").to_string())
                .unwrap_or_else(|| "unknown date".into());
            let sender = self
                .reply_to
                .as_ref()
                .or_else(|| self.from.as_ref())
                .and_then(|addrs| addrs.clone().extract_single_info())
                .map(|addr| addr.display_name.clone().unwrap_or_else(|| addr.addr))
                .unwrap_or_else(|| "unknown sender".into());
            let mut content = format!("\n\nOn {}, {} wrote:\n", date, sender);

            let mut glue = "";
            for line in self.fold_text_parts("plain").trim().lines() {
                if line == DEFAULT_SIG_DELIM {
                    break;
                }
                content.push_str(glue);
                content.push('>');
                content.push_str(if line.starts_with('>') { "" } else { " " });
                content.push_str(line);
                glue = "\n";
            }

            content
        };

        self.parts = Parts(vec![Part::new_text_plain(plain_content)]);

        Ok(self)
    }

    pub fn into_forward(mut self, account: &AccountConfig) -> Result<Self> {
        let account_addr = account.address()?;

        let prev_subject = self.subject.to_owned();
        let prev_date = self.date.to_owned();
        let prev_from = self.reply_to.to_owned().or_else(|| self.from.to_owned());
        let prev_to = self.to.to_owned();

        // Message-Id
        self.message_id = None;

        // In-Reply-To
        self.in_reply_to = None;

        // From
        self.from = Some(vec![account_addr].into());

        // To
        self.to = Some(vec![].into());

        // Cc
        self.cc = None;

        // Bcc
        self.bcc = None;

        // Subject
        if !self.subject.starts_with("Fwd:") {
            self.subject = format!("Fwd: {}", self.subject);
        }

        // Body
        let mut content = String::default();
        content.push_str("\n\n-------- Forwarded Message --------\n");
        content.push_str(&format!("Subject: {}\n", prev_subject));
        if let Some(date) = prev_date {
            content.push_str(&format!("Date: {}\n", date.to_rfc2822()));
        }
        if let Some(addrs) = prev_from.as_ref() {
            content.push_str("From: ");
            content.push_str(&addrs.to_string());
            content.push('\n');
        }
        if let Some(addrs) = prev_to.as_ref() {
            content.push_str("To: ");
            content.push_str(&addrs.to_string());
            content.push('\n');
        }
        content.push('\n');
        content.push_str(&self.fold_text_parts("plain"));
        self.parts
            .replace_text_plain_parts_with(TextPlainPart { content });

        Ok(self)
    }

    fn _edit_with_editor(&self, account: &AccountConfig) -> Result<Self> {
        let tpl = self.to_tpl(TplOverride::default(), account)?;
        let tpl = editor::open_with_tpl(tpl)?;
        Self::from_tpl(&tpl)
    }

    pub fn edit_with_editor<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
        mut self,
        account: &AccountConfig,
        printer: &mut P,
        backend: Box<&'a mut B>,
        smtp: &mut S,
    ) -> Result<Box<&'a mut B>> {
        info!("start editing with editor");

        let draft = msg_utils::local_draft_path();
        if draft.exists() {
            loop {
                match choice::pre_edit() {
                    Ok(choice) => match choice {
                        PreEditChoice::Edit => {
                            let tpl = editor::open_with_draft()?;
                            self.merge_with(Msg::from_tpl(&tpl)?);
                            break;
                        }
                        PreEditChoice::Discard => {
                            self.merge_with(self._edit_with_editor(account)?);
                            break;
                        }
                        PreEditChoice::Quit => return Ok(backend),
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
                    let sent_msg = smtp.send_msg(account, &self)?;
                    backend.add_msg(&account.sent_folder, &sent_msg.formatted(), "seen")?;
                    msg_utils::remove_local_draft()?;
                    printer.print("Message successfully sent")?;
                    break;
                }
                Ok(PostEditChoice::Edit) => {
                    self.merge_with(self._edit_with_editor(account)?);
                    continue;
                }
                Ok(PostEditChoice::LocalDraft) => {
                    printer.print("Message successfully saved locally")?;
                    break;
                }
                Ok(PostEditChoice::RemoteDraft) => {
                    let tpl = self.to_tpl(TplOverride::default(), account)?;
                    backend.add_msg(&account.draft_folder, tpl.as_bytes(), "seen draft")?;
                    msg_utils::remove_local_draft()?;
                    printer.print(format!(
                        "Message successfully saved to {}",
                        account.draft_folder
                    ))?;
                    break;
                }
                Ok(PostEditChoice::Discard) => {
                    msg_utils::remove_local_draft()?;
                    break;
                }
                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            }
        }

        Ok(backend)
    }

    pub fn encrypt(mut self, encrypt: bool) -> Self {
        self.encrypt = encrypt;
        self
    }

    pub fn add_attachments(mut self, attachments_paths: Vec<&str>) -> Result<Self> {
        for path in attachments_paths {
            let path = shellexpand::full(path)
                .context(format!(r#"cannot expand attachment path "{}""#, path))?;
            let path = PathBuf::from(path.to_string());
            let filename: String = path
                .file_name()
                .ok_or_else(|| anyhow!("cannot get file name of attachment {:?}", path))?
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
                    self.parts.retain(|p| !matches!(p, Part::TextPlain(_)));
                    self.parts.push(part);
                }
                Part::TextHtml(_) => {
                    self.parts.retain(|p| !matches!(p, Part::TextHtml(_)));
                    self.parts.push(part);
                }
            }
        }
    }

    pub fn to_tpl(&self, opts: TplOverride, account: &AccountConfig) -> Result<String> {
        let account_addr: Addrs = vec![account.address()?].into();
        let mut tpl = String::default();

        tpl.push_str("Content-Type: text/plain; charset=utf-8\n");

        if let Some(in_reply_to) = self.in_reply_to.as_ref() {
            tpl.push_str(&format!("In-Reply-To: {}\n", in_reply_to))
        }

        // From
        tpl.push_str(&format!(
            "From: {}\n",
            opts.from
                .map(|addrs| addrs.join(", "))
                .unwrap_or_else(|| account_addr.to_string())
        ));

        // To
        tpl.push_str(&format!(
            "To: {}\n",
            opts.to
                .map(|addrs| addrs.join(", "))
                .or_else(|| self.to.clone().map(|addrs| addrs.to_string()))
                .unwrap_or_default()
        ));

        // Cc
        if let Some(addrs) = opts
            .cc
            .map(|addrs| addrs.join(", "))
            .or_else(|| self.cc.clone().map(|addrs| addrs.to_string()))
        {
            tpl.push_str(&format!("Cc: {}\n", addrs));
        }

        // Bcc
        if let Some(addrs) = opts
            .bcc
            .map(|addrs| addrs.join(", "))
            .or_else(|| self.bcc.clone().map(|addrs| addrs.to_string()))
        {
            tpl.push_str(&format!("Bcc: {}\n", addrs));
        }

        // Subject
        tpl.push_str(&format!(
            "Subject: {}\n",
            opts.subject.unwrap_or(&self.subject)
        ));

        // Headers <=> body separator
        tpl.push('\n');

        // Body
        if let Some(body) = opts.body {
            tpl.push_str(body);
        } else {
            tpl.push_str(&self.fold_text_plain_parts())
        }

        // Signature
        if let Some(sig) = opts.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        } else if let Some(ref sig) = account.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        }

        tpl.push('\n');

        trace!("template: {:?}", tpl);
        Ok(tpl)
    }

    pub fn from_tpl(tpl: &str) -> Result<Self> {
        info!("begin: building message from template");
        trace!("template: {:?}", tpl);

        let parsed_mail = mailparse::parse_mail(tpl.as_bytes()).context("cannot parse template")?;

        info!("end: building message from template");
        Self::from_parsed_mail(parsed_mail, &AccountConfig::default())
    }

    pub fn into_sendable_msg(&self, account: &AccountConfig) -> Result<lettre::Message> {
        let mut msg_builder = lettre::Message::builder()
            .message_id(self.message_id.to_owned())
            .subject(self.subject.to_owned());

        if let Some(id) = self.in_reply_to.as_ref() {
            msg_builder = msg_builder.in_reply_to(id.to_owned());
        };

        if let Some(addrs) = self.from.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.from(addr)
            }
        };

        if let Some(addrs) = self.to.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.to(addr)
            }
        };

        if let Some(addrs) = self.reply_to.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.reply_to(addr)
            }
        };

        if let Some(addrs) = self.cc.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.cc(addr)
            }
        };

        if let Some(addrs) = self.bcc.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.bcc(addr)
            }
        };

        let mut multipart = {
            let mut multipart =
                MultiPart::mixed().singlepart(SinglePart::plain(self.fold_text_plain_parts()));
            for part in self.attachments() {
                multipart = multipart.singlepart(Attachment::new(part.filename.clone()).body(
                    part.content,
                    part.mime.parse().context(format!(
                        "cannot parse content type of attachment {}",
                        part.filename
                    ))?,
                ))
            }
            multipart
        };

        if self.encrypt {
            let multipart_buffer = temp_dir().join(Uuid::new_v4().to_string());
            fs::write(multipart_buffer.clone(), multipart.formatted())?;
            let addr = self
                .to
                .as_ref()
                .and_then(|addrs| addrs.clone().extract_single_info())
                .map(|addr| addr.addr)
                .ok_or_else(|| anyhow!("cannot find recipient"))?;
            let encrypted_multipart = account
                .pgp_encrypt_file(&addr, multipart_buffer.clone())?
                .ok_or_else(|| anyhow!("cannot find pgp encrypt command in config"))?;
            trace!("encrypted multipart: {:#?}", encrypted_multipart);
            multipart = MultiPart::encrypted(String::from("application/pgp-encrypted"))
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::parse("application/pgp-encrypted").unwrap())
                        .body(String::from("Version: 1")),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::parse("application/octet-stream").unwrap())
                        .body(encrypted_multipart),
                )
        }

        msg_builder
            .multipart(multipart)
            .context("cannot build sendable message")
    }

    pub fn from_parsed_mail(
        parsed_mail: mailparse::ParsedMail<'_>,
        config: &AccountConfig,
    ) -> Result<Self> {
        info!("begin: building message from parsed mail");
        trace!("parsed mail: {:?}", parsed_mail);

        let mut msg = Msg::default();

        debug!("parsing headers");
        for header in parsed_mail.get_headers() {
            let key = header.get_key();
            debug!("header key: {:?}", key);

            let val = header.get_value();
            let val = String::from_utf8(header.get_value_raw().to_vec())
                .map(|val| val.trim().to_string())
                .context(format!(
                    "cannot decode value {:?} from header {:?}",
                    key, val
                ))?;
            debug!("header value: {:?}", val);

            match key.to_lowercase().as_str() {
                "message-id" => msg.message_id = Some(val),
                "in-reply-to" => msg.in_reply_to = Some(val),
                "subject" => {
                    msg.subject = val;
                }
                "from" => {
                    msg.from = from_slice_to_addrs(val)
                        .context(format!("cannot parse header {:?}", key))?
                }
                "to" => {
                    msg.to = from_slice_to_addrs(val)
                        .context(format!("cannot parse header {:?}", key))?
                }
                "reply-to" => {
                    msg.reply_to = from_slice_to_addrs(val)
                        .context(format!("cannot parse header {:?}", key))?
                }
                "cc" => {
                    msg.cc = from_slice_to_addrs(val)
                        .context(format!("cannot parse header {:?}", key))?
                }
                "bcc" => {
                    msg.bcc = from_slice_to_addrs(val)
                        .context(format!("cannot parse header {:?}", key))?
                }
                _ => (),
            }
        }

        msg.parts = Parts::from_parsed_mail(config, &parsed_mail)
            .context("cannot parsed message mime parts")?;
        trace!("message: {:?}", msg);

        info!("end: building message from parsed mail");
        Ok(msg)
    }
}

impl TryInto<lettre::address::Envelope> for Msg {
    type Error = Error;

    fn try_into(self) -> Result<lettre::address::Envelope> {
        let from = match self.from.and_then(|addrs| addrs.extract_single_info()) {
            Some(addr) => addr.addr.parse().map(Some),
            None => Ok(None),
        }?;
        let to = self
            .to
            .as_ref()
            .map(from_addrs_to_sendable_addrs)
            .unwrap_or(Ok(vec![]))?;
        Ok(lettre::address::Envelope::new(from, to).context("cannot create envelope")?)
    }
}
