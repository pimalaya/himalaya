use ammonia;
use anyhow::{anyhow, Context, Error, Result};
use chrono::{DateTime, FixedOffset};
use convert_case::{Case, Casing};
use html_escape;
use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use log::{debug, info, trace, warn};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    env::temp_dir,
    fmt::Debug,
    fs,
    path::PathBuf,
};
use uuid::Uuid;

use crate::{
    backends::Backend,
    config::{AccountConfig, DEFAULT_DRAFT_FOLDER, DEFAULT_SENT_FOLDER, DEFAULT_SIG_DELIM},
    msg::{
        from_addrs_to_sendable_addrs, from_addrs_to_sendable_mbox, from_slice_to_addrs, msg_utils,
        Addr, Addrs, BinaryPart, Part, Parts, TextPlainPart, TplOverride,
    },
    output::PrinterService,
    smtp::SmtpService,
    ui::{
        choice::{self, PostEditChoice, PreEditChoice},
        editor,
    },
};

/// Representation of a message.
#[derive(Debug, Clone, Default)]
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
    pub headers: HashMap<String, String>,

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

        // In-Reply-To
        self.in_reply_to = self.message_id.to_owned();

        // Message-Id
        self.message_id = None;

        // To
        let addrs = self
            .reply_to
            .as_deref()
            .or_else(|| self.from.as_deref())
            .map(|addrs| {
                addrs.iter().cloned().filter(|addr| match addr {
                    Addr::Group(_) => false,
                    Addr::Single(a) => match &account_addr {
                        Addr::Group(_) => false,
                        Addr::Single(b) => a.addr != b.addr,
                    },
                })
            });
        if all {
            self.to = addrs.map(|addrs| addrs.collect::<Vec<_>>().into());
        } else {
            self.to = addrs
                .and_then(|mut addrs| addrs.next())
                .map(|addr| vec![addr].into());
        }

        // Cc
        self.cc = if all {
            self.cc.as_deref().map(|addrs| {
                addrs
                    .iter()
                    .cloned()
                    .filter(|addr| match addr {
                        Addr::Group(_) => false,
                        Addr::Single(a) => match &account_addr {
                            Addr::Group(_) => false,
                            Addr::Single(b) => a.addr != b.addr,
                        },
                    })
                    .collect::<Vec<_>>()
                    .into()
            })
        } else {
            None
        };

        // Bcc
        self.bcc = None;

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

        // Subject
        if !self.subject.starts_with("Re:") {
            self.subject = format!("Re: {}", self.subject);
        }

        // From
        self.from = Some(vec![account_addr.clone()].into());

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
                    printer.print_str("Sending message…")?;
                    let sent_msg = smtp.send(account, &self)?;
                    let sent_folder = account
                        .mailboxes
                        .get("sent")
                        .map(|s| s.as_str())
                        .unwrap_or(DEFAULT_SENT_FOLDER);
                    printer
                        .print_str(format!("Adding message to the {:?} folder…", sent_folder))?;
                    backend.add_msg(&sent_folder, &sent_msg, "seen")?;
                    msg_utils::remove_local_draft()?;
                    printer.print_struct("Done!")?;
                    break;
                }
                Ok(PostEditChoice::Edit) => {
                    self.merge_with(self._edit_with_editor(account)?);
                    continue;
                }
                Ok(PostEditChoice::LocalDraft) => {
                    printer.print_struct("Message successfully saved locally")?;
                    break;
                }
                Ok(PostEditChoice::RemoteDraft) => {
                    let tpl = self.to_tpl(TplOverride::default(), account)?;
                    let draft_folder = account
                        .mailboxes
                        .get("draft")
                        .map(|s| s.as_str())
                        .unwrap_or(DEFAULT_DRAFT_FOLDER);
                    backend.add_msg(&draft_folder, tpl.as_bytes(), "seen draft")?;
                    msg_utils::remove_local_draft()?;
                    printer
                        .print_struct(format!("Message successfully saved to {}", draft_folder))?;
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
        self.from = msg.from;
        self.reply_to = msg.reply_to;
        self.to = msg.to;
        self.cc = msg.cc;
        self.bcc = msg.bcc;
        self.subject = msg.subject;

        if msg.message_id.is_some() {
            self.message_id = msg.message_id;
        }

        if msg.in_reply_to.is_some() {
            self.in_reply_to = msg.in_reply_to;
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
                    msg.subject = rfc2047_decoder::decode(val.as_bytes())?;
                }
                "date" => {
                    // TODO: use date format instead
                    // https://github.com/jonhoo/rust-imap/blob/afbc5118f251da4e3f6a1e560e749c0700020b54/src/types/fetch.rs#L16
                    msg.date = DateTime::parse_from_rfc2822(
                        val.split_at(val.find(" (").unwrap_or_else(|| val.len())).0,
                    )
                    .map_err(|err| {
                        warn!("cannot parse message date {:?}, skipping it", val);
                        err
                    })
                    .ok();
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
                key => {
                    msg.headers.insert(
                        key.to_lowercase(),
                        rfc2047_decoder::decode(val.as_bytes()).unwrap_or(val),
                    );
                }
            }
        }

        msg.parts = Parts::from_parsed_mail(config, &parsed_mail)
            .context("cannot parsed message mime parts")?;
        trace!("message: {:?}", msg);

        info!("end: building message from parsed mail");
        Ok(msg)
    }

    /// Transforms a message into a readable string. A readable
    /// message is like a template, except that:
    ///  - headers part is customizable (can be omitted if empty filter given in argument)
    ///  - body type is customizable (plain or html)
    pub fn to_readable_string(&self, text_mime: &str, headers: Vec<&str>) -> Result<String> {
        let mut readable_msg = String::new();

        for h in headers {
            match h.to_lowercase().as_str() {
                "message-id" => match self.message_id {
                    Some(ref message_id) if !message_id.is_empty() => {
                        readable_msg.push_str(&format!("Message-Id: {}\n", message_id));
                    }
                    _ => (),
                },
                "in-reply-to" => match self.in_reply_to {
                    Some(ref in_reply_to) if !in_reply_to.is_empty() => {
                        readable_msg.push_str(&format!("In-Reply-To: {}\n", in_reply_to));
                    }
                    _ => (),
                },
                "subject" => {
                    readable_msg.push_str(&format!("Subject: {}\n", self.subject));
                }
                "date" => {
                    if let Some(ref date) = self.date {
                        readable_msg.push_str(&format!("Date: {}\n", date));
                    }
                }
                "from" => match self.from {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_msg.push_str(&format!("From: {}\n", addrs));
                    }
                    _ => (),
                },
                "to" => match self.to {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_msg.push_str(&format!("To: {}\n", addrs));
                    }
                    _ => (),
                },
                "reply-to" => match self.reply_to {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_msg.push_str(&format!("Reply-To: {}\n", addrs));
                    }
                    _ => (),
                },
                "cc" => match self.cc {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_msg.push_str(&format!("Cc: {}\n", addrs));
                    }
                    _ => (),
                },
                "bcc" => match self.bcc {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_msg.push_str(&format!("Bcc: {}\n", addrs));
                    }
                    _ => (),
                },
                key => match self.headers.get(key) {
                    Some(ref val) if !val.is_empty() => {
                        readable_msg.push_str(&format!("{}: {}\n", key.to_case(Case::Train), val));
                    }
                    _ => (),
                },
            };
        }

        if !readable_msg.is_empty() {
            readable_msg.push_str("\n");
        }

        readable_msg.push_str(&self.fold_text_parts(text_mime));
        Ok(readable_msg)
    }
}

impl TryInto<lettre::address::Envelope> for Msg {
    type Error = Error;

    fn try_into(self) -> Result<lettre::address::Envelope> {
        (&self).try_into()
    }
}

impl TryInto<lettre::address::Envelope> for &Msg {
    type Error = Error;

    fn try_into(self) -> Result<lettre::address::Envelope> {
        let from = match self
            .from
            .as_ref()
            .and_then(|addrs| addrs.clone().extract_single_info())
        {
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

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use mailparse::SingleInfo;

    use crate::msg::Addr;

    use super::*;

    #[test]
    fn test_into_reply() {
        let config = AccountConfig {
            display_name: "Test".into(),
            email: "test-account@local".into(),
            ..AccountConfig::default()
        };

        // Checks that:
        //  - "message_id" moves to "in_reply_to"
        //  - "subject" starts by "Re: "
        //  - "to" is replaced by "from"
        //  - "from" is replaced by the address from the account config

        let msg = Msg {
            message_id: Some("msg-id".into()),
            subject: "subject".into(),
            from: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test-sender@local".into(),
                    display_name: None,
                })]
                .into(),
            ),
            ..Msg::default()
        }
        .into_reply(false, &config)
        .unwrap();

        assert_eq!(msg.message_id, None);
        assert_eq!(msg.in_reply_to.unwrap(), "msg-id");
        assert_eq!(msg.subject, "Re: subject");
        assert_eq!(
            msg.from.unwrap().to_string(),
            "\"Test\" <test-account@local>"
        );
        assert_eq!(msg.to.unwrap().to_string(), "test-sender@local");

        // Checks that:
        //  - "subject" does not contains additional "Re: "
        //  - "to" is replaced by reply_to
        //  - "to" contains one address when "all" is false
        //  - "cc" are empty when "all" is false

        let msg = Msg {
            subject: "Re: subject".into(),
            from: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test-sender@local".into(),
                    display_name: None,
                })]
                .into(),
            ),
            reply_to: Some(
                vec![
                    Addr::Single(SingleInfo {
                        addr: "test-sender-to-reply@local".into(),
                        display_name: Some("Sender".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-sender-to-reply-2@local".into(),
                        display_name: Some("Sender 2".into()),
                    }),
                ]
                .into(),
            ),
            cc: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test-cc@local".into(),
                    display_name: None,
                })]
                .into(),
            ),
            ..Msg::default()
        }
        .into_reply(false, &config)
        .unwrap();

        assert_eq!(msg.subject, "Re: subject");
        assert_eq!(
            msg.to.unwrap().to_string(),
            "\"Sender\" <test-sender-to-reply@local>"
        );
        assert_eq!(msg.cc, None);

        // Checks that:
        //  - "to" contains all addresses except for the sender when "all" is true
        //  - "cc" contains all addresses except for the sender when "all" is true

        let msg = Msg {
            from: Some(
                vec![
                    Addr::Single(SingleInfo {
                        addr: "test-sender-1@local".into(),
                        display_name: Some("Sender 1".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-sender-2@local".into(),
                        display_name: Some("Sender 2".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-account@local".into(),
                        display_name: Some("Test".into()),
                    }),
                ]
                .into(),
            ),
            cc: Some(
                vec![
                    Addr::Single(SingleInfo {
                        addr: "test-sender-1@local".into(),
                        display_name: Some("Sender 1".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-sender-2@local".into(),
                        display_name: Some("Sender 2".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-account@local".into(),
                        display_name: None,
                    }),
                ]
                .into(),
            ),
            ..Msg::default()
        }
        .into_reply(true, &config)
        .unwrap();

        assert_eq!(
            msg.to.unwrap().to_string(),
            "\"Sender 1\" <test-sender-1@local>, \"Sender 2\" <test-sender-2@local>"
        );
        assert_eq!(
            msg.cc.unwrap().to_string(),
            "\"Sender 1\" <test-sender-1@local>, \"Sender 2\" <test-sender-2@local>"
        );
    }

    #[test]
    fn test_to_readable() {
        let msg = Msg {
            parts: Parts(vec![Part::TextPlain(TextPlainPart {
                content: String::from("hello, world!"),
            })]),
            ..Msg::default()
        };

        // empty msg, empty headers
        assert_eq!(
            "hello, world!",
            msg.to_readable_string("plain", vec![]).unwrap()
        );
        // empty msg, basic headers
        assert_eq!(
            "hello, world!",
            msg.to_readable_string("plain", vec!["from", "date", "custom-header"])
                .unwrap()
        );
        // empty msg, subject header
        assert_eq!(
            "Subject: \n\nhello, world!",
            msg.to_readable_string("plain", vec!["subject"]).unwrap()
        );

        let msg = Msg {
            headers: HashMap::from_iter([("custom-header".into(), "custom value".into())]),
            message_id: Some("<message-id>".into()),
            from: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test@local".into(),
                    display_name: Some("Test".into()),
                })]
                .into(),
            ),
            cc: Some(vec![].into()),
            parts: Parts(vec![Part::TextPlain(TextPlainPart {
                content: String::from("hello, world!"),
            })]),
            ..Msg::default()
        };

        // header present in msg headers
        assert_eq!(
            "From: \"Test\" <test@local>\n\nhello, world!",
            msg.to_readable_string("plain", vec!["from"]).unwrap()
        );
        // header present but empty in msg headers
        assert_eq!(
            "hello, world!",
            msg.to_readable_string("plain", vec!["cc"]).unwrap()
        );
        // custom header present in msg headers
        assert_eq!(
            "Custom-Header: custom value\n\nhello, world!",
            msg.to_readable_string("plain", vec!["custom-header"])
                .unwrap()
        );
        // custom header present in msg headers (case insensitivity)
        assert_eq!(
            "Custom-Header: custom value\n\nhello, world!",
            msg.to_readable_string("plain", vec!["CUSTOM-hEaDer"])
                .unwrap()
        );
    }
}
