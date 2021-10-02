use anyhow::{anyhow, Context, Error, Result};
use imap::types::{Fetch, Flag, ZeroCopy};
use lettre::message::{
    header::ContentType, Attachment as lettre_Attachment, Mailbox, Message, MultiPart, SinglePart,
};
use log::debug;
use mailparse;
use serde::Serialize;
use std::{
    convert::{From, TryFrom},
    fmt,
};

use crate::{
    config::entity::Account,
    domain::msg::{
        attachment::entity::Attachment, body::entity::Body, header::entity::Headers, Flags,
    },
    ui::{
        editor,
        table::{Cell, Row, Table},
    },
};

/// Representation of a message.
#[derive(Debug, Default, Serialize)]
pub struct Msg {
    /// The sequence number of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.1.2
    id: u32,

    /// The internal date of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.3
    date: Option<String>,

    /// The flags attached to the message.
    pub flags: Flags,

    /// The headers of the message (sender, from, to…).
    pub headers: Headers,

    /// The `text/plain` part of the message.
    pub body_plain: Option<String>,

    /// The `text/html` part of the message.
    pub body_html: Option<String>,

    /// The list of non-textual parts of the message.
    pub attachments: Vec<Attachment>,

    /// The raw version of the message.
    #[serde(skip_serializing)]
    raw: Vec<u8>,
}

impl Msg {
    pub fn new(account: &Account) -> Self {
        let mut headers = Headers::default();
        headers.from = vec![account.address()];
        Self {
            headers,
            ..Self::default()
        }
    }

    /// Convert the message into a reply one.
    ///
    ///  - the `From` is replaced by the address from the user's account
    ///  - the `To` is replaced by all addresses in `ReplyTo` and `To`
    ///  - the `In-Reply-To` is replaced by `Message-ID`
    ///  - the `Subject` is replaced by `Re: {Subject}` unless the subject already starts by `Re:`
    ///  - the attachments are cleared
    ///
    /// In case `reply_all` is false:
    ///  - the `To` is replaced by the `ReplyTo` or the `From` instead
    ///  - the `Cc` and `Bcc` are cleared
    ///
    /// Previous body is prefixed by `>` and added between the user's body and the user's
    /// signature. You can see a discussion example directly on the [RFC5322].
    ///
    /// [RFC5322]: https://www.rfc-editor.org/rfc/rfc5322.html#page-46
    pub fn into_reply(mut self, reply_all: bool, account: &Account) -> Result<Self> {
        let mut headers = Headers::default();

        // From
        let account_addr: Mailbox = account.address().parse()?;
        headers.from = vec![account_addr.to_string()];

        // To
        headers.to = if let Some(reply_to) = self.headers.reply_to {
            reply_to
        } else {
            self.headers.from
        };
        if reply_all {
            for addr in self.headers.to.iter() {
                let addr: Mailbox = addr.parse()?;
                if addr != account_addr {
                    headers.to.push(addr.to_string());
                }
            }
        }

        // In-Reply-To
        headers.in_reply_to = self.headers.message_id;

        // Cc
        if reply_all {
            headers.cc = self.headers.cc.to_owned();
        }

        // Subject
        headers.subject = self.headers.subject.map(|sub| {
            if sub.starts_with("Re:") {
                sub.to_owned()
            } else {
                format!("Re: {}", sub)
            }
        });

        // Text/plain body
        self.body_plain = self.body_plain.map(|body| {
            body.lines()
                .map(|line| {
                    let glue = if line.starts_with(">") { "" } else { " " };
                    format!(">{}{}", glue, line)
                })
                .collect::<Vec<String>>()
                .join("\n")
        });

        // Text/html body
        self.body_html = self.body_html.map(|body| {
            body.lines()
                .map(|line| {
                    let glue = if line.starts_with(">") { "" } else { " " };
                    format!(">{}{}", glue, line)
                })
                .collect::<Vec<String>>()
                .join("\n")
        });

        // Attachments
        self.attachments.clear();

        self.headers = headers;
        Ok(self)
    }

    /// Convert the message into a forward one.
    ///
    ///  - the `From` is replaced by the address from the user's account
    ///  - the `To` is cleared
    ///  - the `Subject` is replaced by `Fwd: {Subject}` unless the subject already starts by `Fwd:`
    ///  - the `Cc` and `Bcc` are cleared
    ///  - the body is prefixed by the line "---------- Forwarded Message ----------"
    pub fn into_forward(mut self, account: &Account) -> Result<Self> {
        let mut headers = Headers::default();

        // From
        headers.from = vec![account.address()];

        // Subject
        headers.subject = self.headers.subject.map(|sub| {
            if sub.starts_with("Fwd:") {
                sub.to_owned()
            } else {
                format!("Fwd: {}", sub)
            }
        });

        // Text/plain body
        // TODO: add Subject, Date, From and To headers after "Forwarded Message"
        self.body_plain = self.body_plain.map(|body| {
            vec![
                "",
                "",
                "---------- Forwarded Message ----------",
                "",
                &body.replace("\r", ""),
            ]
            .join("\n")
        });

        // Text/html body
        // TODO: add Subject, Date, From and To headers after "Forwarded Message"
        self.body_html = self.body_html.map(|body| {
            vec![
                "",
                "",
                "---------- Forwarded Message ----------",
                "",
                &body.replace("\r", ""),
            ]
            .join("\n")
        });

        self.headers = headers;
        Ok(self)
    }

    /// Let the user edit the body of the msg.
    ///
    /// It'll enter the headers of the headers into the draft-file *if they're
    /// not [`None`]!*.
    ///
    /// # Example
    /// ```no_run
    /// use himalaya::config::model::Account;
    /// use himalaya::msg::model::Msg;
    /// use himalaya::ctx::Ctx;
    ///
    /// fn main() {
    ///     let ctx = Ctx {
    ///         account: Account::new(Some("Name"), "some@msg.asdf"),
    ///         .. Ctx::default()
    ///     };
    ///     let mut msg = Msg::new(&ctx);
    ///
    ///     // In this case, only the header fields "From:" and "To:" are gonna
    ///     // be editable, because the other headers fields are set to "None"
    ///     // per default!
    ///     msg.edit_body().unwrap();
    /// }
    /// ```
    ///
    /// Now enable some headers:
    ///
    /// ```no_run
    /// use himalaya::config::model::Account;
    /// use himalaya::msg::{headers::Headers, model::Msg};
    /// use himalaya::ctx::Ctx;
    ///
    /// fn main() {
    ///     let ctx = Ctx {
    ///         account: Account::new(Some("Name"), "some@msg.asdf"),
    ///         .. Ctx::default()
    ///     };
    ///
    ///     let mut msg = Msg::new_with_headers(
    ///         &ctx,
    ///         Headers {
    ///             bcc: Some(Vec::new()),
    ///             cc: Some(Vec::new()),
    ///             ..Headers::default()
    ///         },
    ///     );
    ///
    ///     // The "Bcc:" and "Cc:" header fields are gonna be editable as well
    ///     msg.edit_body().unwrap();
    /// }
    /// ```
    ///
    /// # Errors
    /// In generel an error should appear if
    /// - The draft or changes couldn't be saved
    /// - The changed msg can't be parsed! (You wrote some things wrong...)
    pub fn edit_body(&mut self) -> Result<()> {
        // First of all, we need to create our template for the user. This
        // means, that the header needs to be added as well!
        let msg = self.to_string();

        // We don't let this line compile, if we're doing
        // tests, because we just need to look, if the headers are set
        // correctly
        let msg = editor::open_editor_with_bytes(msg.as_bytes())?;

        // refresh the state of the msg
        self.parse_from_str(&msg)?;

        Ok(())
    }

    /// Read the string of the argument `content` and store it's values into the
    /// struct. It stores the headers-fields and the body of the msg.
    ///
    /// **Hint: The signature can't be fetched of the content at the moment!**
    ///
    /// # Example
    /// ```
    /// use himalaya::config::model::Account;
    /// use himalaya::msg::model::Msg;
    /// use himalaya::ctx::Ctx;
    ///
    /// fn main() {
    ///     let content = concat![
    ///         "Subject: Himalaya is nice\n",
    ///         "To: Soywod <clement.douin@posteo.net>\n",
    ///         "From: TornaxO7 <tornax07@gmail.com>\n",
    ///         "Bcc: third_person@msg.com,rofl@yeet.com\n",
    ///         "\n",
    ///         "You should use himalaya, it's a nice program :D\n",
    ///         "\n",
    ///         "Sincerely\n",
    ///     ];
    ///
    ///     let ctx = Ctx {
    ///         account: Account::new(Some("Username"), "some@msg.com"),
    ///         .. Ctx::default()
    ///     };
    ///
    ///     // create the message
    ///     let mut msg = Msg::new(&ctx);
    ///
    ///     // store the information given by the `content` variable which
    ///     // represents our current msg
    ///     msg.parse_from_str(content);
    /// }
    /// ```
    pub fn parse_from_str(&mut self, content: &str) -> Result<()> {
        let parsed = mailparse::parse_mail(content.as_bytes())
            .context(format!("How the message looks like currently:\n{}", self))?;

        self.headers = Headers::from(&parsed);

        match parsed.get_body() {
            Ok(body) => self.body_plain = Some(body),
            Err(err) => return Err(anyhow!(err.to_string())),
        };

        Ok(())
    }

    /// Add an attachment to the msg from the local machine by the given path.
    ///
    /// # Example
    /// ```
    /// use himalaya::config::model::Account;
    /// use himalaya::msg::headers::Headers;
    /// use himalaya::msg::model::Msg;
    /// use himalaya::ctx::Ctx;
    ///
    /// fn main() {
    ///     let ctx = Ctx {
    ///         account: Account::new(Some("Name"), "address@msg.com"),
    ///         .. Ctx::default()
    ///     };
    ///     let mut msg = Msg::new(&ctx);
    ///
    ///     // suppose we have a Screenshot saved in our home directory
    ///     // Remember: Currently himalaya can't expand tilde ('~') and shell variables
    ///     msg.add_attachment("/home/bruh/Screenshot.png");
    /// }
    /// ```
    ///
    // THOUGHT: Error handling?
    pub fn add_attachment(&mut self, path: &str) {
        if let Ok(new_attachment) = Attachment::try_from(path) {
            self.attachments.push(new_attachment);
        }
    }

    /// This function will use the information of the `Msg` struct and creates
    /// a sendable msg with it. It uses the `Msg.headers` and
    /// `Msg.attachments` fields for that.
    ///
    /// # Example
    /// ```no_run
    /// use himalaya::config::model::Account;
    /// use himalaya::smtp;
    ///
    /// use himalaya::msg::{body::Body, headers::Headers, model::Msg};
    ///
    /// use himalaya::imap::model::ImapConnector;
    ///
    /// use himalaya::ctx::Ctx;
    ///
    /// use imap::types::Flag;
    ///
    /// fn main() {
    ///     let ctx = Ctx {
    ///         account: Account::new(Some("Name"), "name@msg.net"),
    ///         .. Ctx::default()
    ///     };
    ///
    ///     let mut imap_conn = ImapConnector::new(&ctx.account).unwrap();
    ///     let mut msg = Msg::new_with_headers(
    ///         &ctx,
    ///         Headers {
    ///             to: vec!["someone <msg@address.net>".to_string()],
    ///             ..Headers::default()
    ///         },
    ///     );
    ///
    ///     msg.body = Body::new_with_text("A little text.");
    ///     let sendable_msg = msg.to_sendable_msg().unwrap();
    ///
    ///     // now send the msg. Hint: Do the appropriate error handling here!
    ///     smtp::send(&ctx.account, &sendable_msg).unwrap();
    ///
    ///     // also say to the server of the account user, that we've just sent
    ///     // new message
    ///     msg.flags.insert(Flag::Seen);
    ///     imap_conn.append_msg("Sent", &mut msg).unwrap();
    ///
    ///     imap_conn.logout();
    /// }
    /// ```
    pub fn to_sendable_msg(&mut self) -> Result<Message> {
        // == Header of Msg ==
        // This variable will hold all information of our msg
        let mut msg = Message::builder();

        // -- Must-have-fields --
        // add "from"
        for mailaddress in &self.headers.from {
            msg = msg.from(
                match mailaddress.parse().context("cannot parse `From` header") {
                    Ok(from) => from,
                    Err(err) => return Err(anyhow!(err.to_string())),
                },
            );
        }

        // add "to"
        for mailaddress in &self.headers.to {
            msg = msg.to(
                match mailaddress.parse().context("cannot parse `To` header") {
                    Ok(from) => from,
                    Err(err) => return Err(anyhow!(err.to_string())),
                },
            );
        }

        // -- Optional fields --
        // add "bcc"
        if let Some(bcc) = &self.headers.bcc {
            for mailaddress in bcc {
                msg = msg.bcc(
                    match mailaddress.parse().context("cannot parse `Bcc` header") {
                        Ok(from) => from,
                        Err(err) => return Err(anyhow!(err.to_string())),
                    },
                );
            }
        }

        // add "cc"
        if let Some(cc) = &self.headers.cc {
            for mailaddress in cc {
                msg = msg.cc(
                    match mailaddress.parse().context("cannot parse `Cc` header") {
                        Ok(from) => from,
                        Err(err) => return Err(anyhow!(err.to_string())),
                    },
                );
            }
        }

        // add "in_reply_to"
        if let Some(in_reply_to) = &self.headers.in_reply_to {
            msg = msg.in_reply_to(
                match in_reply_to
                    .parse()
                    .context("cannot parse `In-Reply-To` header")
                {
                    Ok(from) => from,
                    Err(err) => return Err(anyhow!(err.to_string())),
                },
            );
        }

        // add message-id if it exists
        msg = match self.headers.message_id.clone() {
            Some(message_id) => msg.message_id(Some(message_id)),
            None => {
                // extract the domain like "gmail.com"
                let mailbox: lettre::message::Mailbox = self.headers.from[0].parse()?;
                let domain = mailbox.email.domain();

                // generate a new UUID
                let new_msg_id = format!("{}@{}", uuid::Uuid::new_v4().to_string(), domain);

                msg.message_id(Some(new_msg_id))
            }
        };

        // add "reply-to"
        if let Some(reply_to) = &self.headers.reply_to {
            for mailaddress in reply_to {
                msg = msg.reply_to(
                    match mailaddress
                        .parse()
                        .context("cannot parse `Reply-To` header")
                    {
                        Ok(from) => from,
                        Err(err) => return Err(anyhow!(err.to_string())),
                    },
                );
            }
        }

        // add "sender"
        if let Some(sender) = &self.headers.sender {
            msg = msg.sender(
                match sender.parse().context("cannot parse `Sender` header") {
                    Ok(from) => from,
                    Err(err) => return Err(anyhow!(err.to_string())),
                },
            );
        }

        // add subject
        if let Some(subject) = &self.headers.subject {
            msg = msg.subject(subject);
        }

        // -- Body + Attachments --
        // In this part, we'll add the content of the msg. This means the body
        // and the attachments of the msg.

        // this variable will store all "sections" or attachments of the msg
        let mut msg_parts = MultiPart::mixed().build();

        // -- Body --
        if self.body_plain.is_some() && self.body_html.is_some() {
            msg_parts = msg_parts.multipart(MultiPart::alternative_plain_html(
                self.body_plain.clone().unwrap(),
                self.body_html.clone().unwrap(),
            ));
        } else {
            let msg_body = SinglePart::builder()
                .header(ContentType::TEXT_PLAIN)
                .header(self.headers.encoding)
                .body(self.body_plain.clone().unwrap_or_default());

            msg_parts = msg_parts.singlepart(msg_body);
        }

        // -- Attachments --
        for attachment in self.attachments.iter() {
            let msg_attachment = lettre_Attachment::new(attachment.filename.clone());
            let msg_attachment =
                msg_attachment.body(attachment.body_raw.clone(), attachment.content_type.clone());

            msg_parts = msg_parts.singlepart(msg_attachment);
        }

        Ok(msg
            .multipart(msg_parts)
            // whenever an error appears, print out the messge as well to see what might be the
            // error
            .context(format!("-- Current Message --\n{}", self))?)
    }

    /// Returns the raw mail as a string instead of a Vector of bytes.
    pub fn get_raw_as_string(&self) -> Result<String> {
        let raw_message = String::from_utf8(self.raw.clone())
            .context(format!("cannot parse raw message as string"))?;

        Ok(raw_message)
    }
}

impl fmt::Display for Msg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\n{}",
            self.headers,
            self.body_plain
                .to_owned()
                .or_else(|| self.body_html.to_owned())
                .unwrap_or_default(),
        )
    }
}

impl Table for Msg {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("ID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("FROM").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let is_seen = !self.flags.contains(&Flag::Seen);

        // The data which will be shown in the row
        let flags = self.flags.to_symbols_string();
        let subject = self.headers.subject.clone().unwrap_or_default();
        let mut from = String::new();
        let date = self.date.clone().unwrap_or(String::new());

        for from_addr in self.headers.from.iter() {
            let mut address_iter = from_addr.split_ascii_whitespace();

            if let Some(name) = address_iter.next() {
                from.push_str(&format!("{}, ", name));
            } else if let Some(address) = address_iter.next() {
                from.push_str(&format!("{}, ", address));
            } else {
                from.push_str("UNKNWON");
            }
        }

        // remove trailing whitespace + the ','
        let mut from = from.trim_end().to_string();
        from.pop();

        Row::new()
            .cell(Cell::new(&self.id.to_string()).bold_if(is_seen).red())
            .cell(Cell::new(&flags).bold_if(is_seen).white())
            .cell(Cell::new(&subject).shrinkable().bold_if(is_seen).green())
            .cell(Cell::new(&from).bold_if(is_seen).blue())
            .cell(Cell::new(&date).bold_if(is_seen).yellow())
    }
}

// -- From's --
/// Load the data from a fetched msg and store them in the msg-struct.
/// Please make sure that the fetch includes the following query:
///
/// - UID      (optional)
/// - FLAGS    (optional)
/// - ENVELOPE (optional)
/// - INTERNALDATE
/// - BODY[]   (optional)
impl TryFrom<&Fetch> for Msg {
    type Error = Error;

    fn try_from(fetch: &Fetch) -> Result<Msg> {
        // -- Preparations --
        // We're preparing the variables first, which will hold the data of the
        // fetched msg.

        let mut attachments = Vec::new();
        let flags = Flags::default();
        let headers = Headers::try_from(fetch.envelope())?;
        let id = fetch.message;

        let date = fetch
            .internal_date()
            .map(|date| date.naive_local().to_string());

        let raw = match fetch.body() {
            Some(body) => body.to_vec(),
            None => Vec::new(),
        };

        // Get the content of the msg. Here we have to look (important!) if
        // the fetch even includes a body or not, since the `BODY[]` query is
        // only *optional*!
        let parsed =
            // the empty array represents an invalid body, so we can enter the
            // `Err` arm if the body-query wasn't applied
            match mailparse::parse_mail(raw.as_slice()) {
                Ok(parsed) => {
                    debug!("Fetch has a body to parse.");
                    Some(parsed)
                },
                Err(_) => {
                    debug!("Fetch hasn't a body to parse.");
                    None
                },
            };

        // -- Storing the information (body) --
        let mut body = Body::new();
        if let Some(parsed) = parsed {
            // Ok, so some mails have their mody wrapped in a multipart, some
            // don't. This condition hits, if the body isn't in a multipart, so we can
            // immediately fetch the body from the first part of the mail.
            match parsed.ctype.mimetype.as_ref() {
                "text/plain" => body.plain = parsed.get_body().ok(),
                "text/html" => body.html = parsed.get_body().ok(),
                _ => (),
            };

            for subpart in &parsed.subparts {
                // now it might happen, that the body is *in* a multipart, if
                // that's the case, look, if we've already applied a body
                // (body.is_empty()) and set it, if needed
                if body.plain.is_none() && subpart.ctype.mimetype == "text/plain" {
                    body.plain = subpart.get_body().ok();
                } else if body.html.is_none() && subpart.ctype.mimetype == "text/html" {
                    body.html = subpart.get_body().ok();
                }
                // otherise it's a normal attachment, like a PNG file or
                // something like that
                else if let Some(attachment) = Attachment::from_parsed_mail(subpart) {
                    attachments.push(attachment);
                }
                // this shouldn't happen, since this would mean, that's neither an attachment nor
                // the body of the mail but something else. Log that!
                else {
                    println!(
                        "Unknown attachment with the following mime-type: {}",
                        subpart.ctype.mimetype,
                    );
                }
            }
        }

        Ok(Self {
            attachments,
            flags,
            headers,
            body_plain: body.plain,
            body_html: body.html,
            id,
            date,
            raw,
            ..Self::default()
        })
    }
}

impl TryFrom<&str> for Msg {
    type Error = Error;

    fn try_from(content: &str) -> Result<Self> {
        let mut msg = Msg::default();
        msg.parse_from_str(content)?;

        Ok(msg)
    }
}

// == Msgs ==
/// A Type-Safety struct which stores a vector of Messages.
#[derive(Debug, Default, Serialize)]
pub struct Msgs(pub Vec<Msg>);

// -- From's --
impl TryFrom<ZeroCopy<Vec<Fetch>>> for Msgs {
    type Error = Error;

    fn try_from(fetches: ZeroCopy<Vec<Fetch>>) -> Result<Self> {
        let mut msgs = Vec::new();

        for fetch in fetches.iter().rev() {
            msgs.push(Msg::try_from(fetch)?);
        }

        Ok(Self(msgs))
    }
}

// -- Traits --
impl fmt::Display for Msgs {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "\n{}", Table::render(&self.0))
    }
}

// FIXME: fix tests
// #[cfg(test)]
// mod tests {
//     use crate::{
//         ctx::Ctx,
//         domain::{account::entity::Account, config::entity::Config},
//         msg::{body::Body, headers::Headers, model::Msg},
//     };

//     #[test]
//     fn test_new() {
//         let ctx = Ctx {
//             account: Account::new_with_signature(None, "test@mail.com", None),
//             config: Config {
//                 name: String::from("Config Name"),
//                 ..Config::default()
//             },
//             ..Ctx::default()
//         };

//         let msg = Msg::new(&ctx);
//         let expected_headers = Headers {
//             from: vec![String::from("Config Name <test@mail.com>")],
//             ..Headers::default()
//         };

//         assert_eq!(
//             msg.headers, expected_headers,
//             "{:#?}, {:#?}",
//             msg.headers, expected_headers
//         );
//         assert!(msg.get_raw_as_string().unwrap().is_empty());
//     }

//     #[test]
//     fn test_new_with_account_name() {
//         let ctx = Ctx {
//             account: Account::new_with_signature(Some("Account Name"), "test@mail.com", None),
//             config: Config {
//                 name: String::from("Config Name"),
//                 ..Config::default()
//             },
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         let msg = Msg::new(&ctx);
//         let expected_headers = Headers {
//             from: vec![String::from("Account Name <test@mail.com>")],
//             ..Headers::default()
//         };

//         assert_eq!(
//             msg.headers, expected_headers,
//             "{:#?}, {:#?}",
//             msg.headers, expected_headers
//         );
//         assert!(msg.get_raw_as_string().unwrap().is_empty());
//     }

//     #[test]
//     fn test_new_with_headers() {
//         let ctx = Ctx {
//             account: Account::new(Some("Account Name"), "test@mail.com"),
//             config: Config {
//                 name: String::from("Config Name"),
//                 ..Config::default()
//             },
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         let msg_with_custom_from = Msg::new_with_headers(
//             &ctx,
//             Headers {
//                 from: vec![String::from("Account Name <test@mail.com>")],
//                 ..Headers::default()
//             },
//         );
//         let expected_with_custom_from = Msg {
//             headers: Headers {
//                 // the Msg::new_with_headers function should use the from
//                 // address in the headers struct, not the from address of the
//                 // account
//                 from: vec![String::from("Account Name <test@mail.com>")],
//                 ..Headers::default()
//             },
//             // The signature should be added automatically
//             body: Body::new_with_text("\n"),
//             ..Msg::default()
//         };

//         assert_eq!(
//             msg_with_custom_from, expected_with_custom_from,
//             "Left: {:#?}, Right: {:#?}",
//             msg_with_custom_from, expected_with_custom_from
//         );
//     }

//     #[test]
//     fn test_new_with_headers_and_signature() {
//         let ctx = Ctx {
//             account: Account::new_with_signature(
//                 Some("Account Name"),
//                 "test@mail.com",
//                 Some("Signature"),
//             ),
//             config: Config {
//                 name: String::from("Config Name"),
//                 ..Config::default()
//             },
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         let msg_with_custom_signature = Msg::new_with_headers(&ctx, Headers::default());

//         let expected_with_custom_signature = Msg {
//             headers: Headers {
//                 from: vec![String::from("Account Name <test@mail.com>")],
//                 signature: Some(String::from("\n-- \nSignature")),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text("\n\n-- \nSignature"),
//             ..Msg::default()
//         };

//         assert_eq!(
//             msg_with_custom_signature,
//             expected_with_custom_signature,
//             "Left: {:?}, Right: {:?}",
//             dbg!(&msg_with_custom_signature),
//             dbg!(&expected_with_custom_signature)
//         );
//     }

//     #[test]
//     fn test_change_to_reply() {
//         // in this test, we are gonna reproduce the same situation as shown
//         // here: https://datatracker.ietf.org/doc/html/rfc5322#appendix-A.2

//         // == Preparations ==
//         // -- rfc test --
//         // accounts for the rfc test
//         let config = Config {
//             name: String::from("Config Name"),
//             ..Config::default()
//         };

//         let john_doe = Ctx {
//             account: Account::new(Some("John Doe"), "jdoe@machine.example"),
//             config: config.clone(),
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         let mary_smith = Ctx {
//             account: Account::new(Some("Mary Smith"), "mary@example.net"),
//             config: config.clone(),
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         let msg_rfc_test = Msg {
//             headers: Headers {
//                 from: vec!["John Doe <jdoe@machine.example>".to_string()],
//                 to: vec!["Mary Smith <mary@example.net>".to_string()],
//                 subject: Some("Saying Hello".to_string()),
//                 message_id: Some("<1234@local.machine.example>".to_string()),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text(concat![
//                 "This is a message just to say hello.\n",
//                 "So, \"Hello\".",
//             ]),
//             ..Msg::default()
//         };

//         // -- for general tests --
//         let ctx = Ctx {
//             account: Account::new(Some("Name"), "some@address.asdf"),
//             config,
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         // -- for reply_all --
//         // a custom test to look what happens, if we want to reply to all addresses.
//         // Take a look into the doc of the "change_to_reply" what should happen, if we
//         // set "reply_all" to "true".
//         let mut msg_reply_all = Msg {
//             headers: Headers {
//                 from: vec!["Boss <someone@boss.asdf>".to_string()],
//                 to: vec![
//                     "msg@1.asdf".to_string(),
//                     "msg@2.asdf".to_string(),
//                     "Name <some@address.asdf>".to_string(),
//                 ],
//                 cc: Some(vec![
//                     "test@testing".to_string(),
//                     "test2@testing".to_string(),
//                 ]),
//                 message_id: Some("RandomID123".to_string()),
//                 reply_to: Some(vec!["Reply@Mail.rofl".to_string()]),
//                 subject: Some("Have you heard of himalaya?".to_string()),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text(concat!["A body test\n", "\n", "Sincerely",]),
//             ..Msg::default()
//         };

//         // == Expected output(s) ==
//         // -- rfc test --
//         // the first step
//         let expected_rfc1 = Msg {
//             headers: Headers {
//                 from: vec!["Mary Smith <mary@example.net>".to_string()],
//                 to: vec!["John Doe <jdoe@machine.example>".to_string()],
//                 reply_to: Some(vec![
//                     "\"Mary Smith: Personal Account\" <smith@home.example>".to_string(),
//                 ]),
//                 subject: Some("Re: Saying Hello".to_string()),
//                 message_id: Some("<3456@example.net>".to_string()),
//                 in_reply_to: Some("<1234@local.machine.example>".to_string()),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text(concat![
//                 "> This is a message just to say hello.\n",
//                 "> So, \"Hello\".",
//             ]),
//             ..Msg::default()
//         };

//         // then the response the the first respone above
//         let expected_rfc2 = Msg {
//             headers: Headers {
//                 to: vec!["\"Mary Smith: Personal Account\" <smith@home.example>".to_string()],
//                 from: vec!["John Doe <jdoe@machine.example>".to_string()],
//                 subject: Some("Re: Saying Hello".to_string()),
//                 message_id: Some("<abcd.1234@local.machine.test>".to_string()),
//                 in_reply_to: Some("<3456@example.net>".to_string()),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text(concat![
//                 ">> This is a message just to say hello.\n",
//                 ">> So, \"Hello\".",
//             ]),
//             ..Msg::default()
//         };

//         // -- reply all --
//         let expected_reply_all = Msg {
//             headers: Headers {
//                 from: vec!["Name <some@address.asdf>".to_string()],
//                 to: vec![
//                     "msg@1.asdf".to_string(),
//                     "msg@2.asdf".to_string(),
//                     "Reply@Mail.rofl".to_string(),
//                 ],
//                 cc: Some(vec![
//                     "test@testing".to_string(),
//                     "test2@testing".to_string(),
//                 ]),
//                 in_reply_to: Some("RandomID123".to_string()),
//                 subject: Some("Re: Have you heard of himalaya?".to_string()),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text(concat!["> A body test\n", "> \n", "> Sincerely"]),
//             ..Msg::default()
//         };

//         // == Testing ==
//         // -- rfc test --
//         // represents the message for the first reply
//         let mut rfc_reply_1 = msg_rfc_test.clone();
//         rfc_reply_1.change_to_reply(&mary_smith, false).unwrap();

//         // the user would enter this normally
//         rfc_reply_1.headers = Headers {
//             message_id: Some("<3456@example.net>".to_string()),
//             reply_to: Some(vec![
//                 "\"Mary Smith: Personal Account\" <smith@home.example>".to_string(),
//             ]),
//             ..rfc_reply_1.headers.clone()
//         };

//         // represents the message for the reply to the reply
//         let mut rfc_reply_2 = rfc_reply_1.clone();
//         rfc_reply_2.change_to_reply(&john_doe, false).unwrap();
//         rfc_reply_2.headers = Headers {
//             message_id: Some("<abcd.1234@local.machine.test>".to_string()),
//             ..rfc_reply_2.headers.clone()
//         };

//         assert_eq!(
//             rfc_reply_1,
//             expected_rfc1,
//             "Left: {:?}, Right: {:?}",
//             dbg!(&rfc_reply_1),
//             dbg!(&expected_rfc1)
//         );

//         assert_eq!(
//             rfc_reply_2,
//             expected_rfc2,
//             "Left: {:?}, Right: {:?}",
//             dbg!(&rfc_reply_2),
//             dbg!(&expected_rfc2)
//         );

//         // -- custom tests -—
//         msg_reply_all.change_to_reply(&ctx, true).unwrap();
//         assert_eq!(
//             msg_reply_all,
//             expected_reply_all,
//             "Left: {:?}, Right: {:?}",
//             dbg!(&msg_reply_all),
//             dbg!(&expected_reply_all)
//         );
//     }

//     #[test]
//     fn test_change_to_forwarding() {
//         // == Preparations ==
//         let ctx = Ctx {
//             account: Account::new_with_signature(Some("Name"), "some@address.asdf", Some("lol")),
//             config: Config {
//                 name: String::from("Config Name"),
//                 ..Config::default()
//             },
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         let mut msg = Msg::new_with_headers(
//             &ctx,
//             Headers {
//                 from: vec![String::from("ThirdPerson <some@msg.asdf>")],
//                 subject: Some(String::from("Test subject")),
//                 ..Headers::default()
//             },
//         );

//         msg.body = Body::new_with_text(concat!["The body text, nice!\n", "Himalaya is nice!",]);

//         // == Expected Results ==
//         let expected_msg = Msg {
//             headers: Headers {
//                 from: vec![String::from("ThirdPerson <some@msg.asdf>")],
//                 sender: Some(String::from("Name <some@address.asdf>")),
//                 signature: Some(String::from("\n-- \nlol")),
//                 subject: Some(String::from("Fwd: Test subject")),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text(concat![
//                 "\n",
//                 "---------- Forwarded Message ----------\n",
//                 "The body text, nice!\n",
//                 "Himalaya is nice!\n",
//                 "\n-- \nlol"
//             ]),
//             ..Msg::default()
//         };

//         // == Tests ==
//         msg.change_to_forwarding(&ctx);
//         assert_eq!(
//             msg,
//             expected_msg,
//             "Left: {:?}, Right: {:?}",
//             dbg!(&msg),
//             dbg!(&expected_msg)
//         );
//     }

//     #[test]
//     fn test_edit_body() {
//         // == Preparations ==
//         let ctx = Ctx {
//             account: Account::new_with_signature(Some("Name"), "some@address.asdf", None),
//             ..Ctx::default()
//         };

//         let mut msg = Msg::new_with_headers(
//             &ctx,
//             Headers {
//                 bcc: Some(vec![String::from("bcc <some@mail.com>")]),
//                 cc: Some(vec![String::from("cc <some@mail.com>")]),
//                 subject: Some(String::from("Subject")),
//                 ..Headers::default()
//             },
//         );

//         // == Expected Results ==
//         let expected_msg = Msg {
//             headers: Headers {
//                 from: vec![String::from("Name <some@address.asdf>")],
//                 to: vec![String::new()],
//                 // these fields should exist now
//                 subject: Some(String::from("Subject")),
//                 bcc: Some(vec![String::from("bcc <some@mail.com>")]),
//                 cc: Some(vec![String::from("cc <some@mail.com>")]),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text("\n"),
//             ..Msg::default()
//         };

//         // == Tests ==
//         msg.edit_body().unwrap();

//         assert_eq!(
//             msg, expected_msg,
//             "Left: {:#?}, Right: {:#?}",
//             msg, expected_msg
//         );
//     }

//     #[test]
//     fn test_parse_from_str() {
//         use std::collections::HashMap;

//         // == Preparations ==
//         let ctx = Ctx {
//             account: Account::new_with_signature(Some("Name"), "some@address.asdf", None),
//             config: Config {
//                 name: String::from("Config Name"),
//                 ..Config::default()
//             },
//             mbox: String::from("INBOX"),
//             ..Ctx::default()
//         };

//         let msg_template = Msg::new(&ctx);

//         let normal_content = concat![
//             "From: Some <user@msg.sf>\n",
//             "Subject: Awesome Subject\n",
//             "Bcc: mail1@rofl.lol,name <rofl@lol.asdf>\n",
//             "To: To <name@msg.rofl>\n",
//             "\n",
//             "Account Signature\n",
//         ];

//         let content_with_custom_headers = concat![
//             "From: Some <user@msg.sf>\n",
//             "Subject: Awesome Subject\n",
//             "Bcc: mail1@rofl.lol,name <rofl@lol.asdf>\n",
//             "To: To <name@msg.rofl>\n",
//             "CustomHeader1: Value1\n",
//             "CustomHeader2: Value2\n",
//             "\n",
//             "Account Signature\n",
//         ];

//         // == Expected outputs ==
//         let expect = Msg {
//             headers: Headers {
//                 from: vec![String::from("Some <user@msg.sf>")],
//                 subject: Some(String::from("Awesome Subject")),
//                 bcc: Some(vec![
//                     String::from("name <rofl@lol.asdf>"),
//                     String::from("mail1@rofl.lol"),
//                 ]),
//                 to: vec![String::from("To <name@msg.rofl>")],
//                 ..Headers::default()
//             },
//             body: Body::new_with_text("Account Signature\n"),
//             ..Msg::default()
//         };

//         // -- with custom headers --
//         let mut custom_headers: HashMap<String, Vec<String>> = HashMap::new();
//         custom_headers.insert("CustomHeader1".to_string(), vec!["Value1".to_string()]);
//         custom_headers.insert("CustomHeader2".to_string(), vec!["Value2".to_string()]);

//         let expect_custom_header = Msg {
//             headers: Headers {
//                 from: vec![String::from("Some <user@msg.sf>")],
//                 subject: Some(String::from("Awesome Subject")),
//                 bcc: Some(vec![
//                     String::from("name <rofl@lol.asdf>"),
//                     String::from("mail1@rofl.lol"),
//                 ]),
//                 to: vec![String::from("To <name@msg.rofl>")],
//                 custom_headers: Some(custom_headers),
//                 ..Headers::default()
//             },
//             body: Body::new_with_text("Account Signature\n"),
//             ..Msg::default()
//         };

//         // == Testing ==
//         let mut msg1 = msg_template.clone();
//         let mut msg2 = msg_template.clone();

//         msg1.parse_from_str(normal_content).unwrap();
//         msg2.parse_from_str(content_with_custom_headers).unwrap();

//         assert_eq!(
//             msg1,
//             expect,
//             "Left: {:?}, Right: {:?}",
//             dbg!(&msg1),
//             dbg!(&expect)
//         );

//         assert_eq!(
//             msg2,
//             expect_custom_header,
//             "Left: {:?}, Right: {:?}",
//             dbg!(&msg2),
//             dbg!(&expect_custom_header)
//         );
//     }
// }
