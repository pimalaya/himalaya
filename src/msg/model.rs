use super::attachment::Attachment;
use super::envelope::Envelope;

use imap::types::{Fetch, Flag, ZeroCopy};

use mailparse;

use crate::{
    config::model::Account,
    flag::model::Flags,
    input,
    table::{Cell, Row, Table},
};

use serde::Serialize;

use lettre::message::{Attachment as lettre_Attachment, Mailbox, Message, MultiPart, SinglePart};

use std::collections::{HashMap, HashSet};
use std::convert::{From, TryFrom};
use std::fmt;

use colorful::Colorful;

use error_chain::error_chain;

error_chain! {
    errors {
        // An error appeared, when it tried to parse the body of the mail!
        ParseBody (err: String) {
            description("Couldn't get the body of the parsed mail."),
            display("Couldn't get the body of the parsed mail: {}", err),
        }

        /// Is mainly used in the "to_sendable_msg" function
        Header(error_msg: String, header_name: &'static str, header_input: String) {

            description("An error happened, when trying to parse a header-field."),
            display(concat![
                    "[{}] {}\n",
                    "Header-Field-Name: '{}'\n",
                    "The word which let this error occur: '{}'"], 
                    "Error".red(),
                    error_msg.clone().light_red(),
                    header_name.light_blue(),
                    header_input.clone().light_cyan()),
        }
    }

    links {
        Attachment(super::attachment::Error, super::attachment::ErrorKind);
        Envelope(super::envelope::Error, super::envelope::ErrorKind);
        Input(crate::input::Error, crate::input::ErrorKind);
    }

    foreign_links {
        MailParse(mailparse::MailParseError);
        Lettre(lettre::error::Error);
        LettreAddress(lettre::address::AddressError);
    }
}

// =========
// Msg
// =========
#[derive(Debug, Serialize)]
pub struct Msg {
    /// All added attachments are listed in this vector.
    attachments: Vec<Attachment>,

    /// The flags for this mail.
    flags: Flags,

    /// All information of the envelope (sender, from, to and so on)
    // envelope: HashMap<HeaderName, Vec<String>>,
    pub envelope: Envelope,

    /// The UID of the mail. It's only set from the server. So that's why it's
    /// not public: To avoid that it's set manually!
    uid: Option<u32>,

    date: Option<String>,
}

impl Msg {
    pub fn new(account: &Account) -> Self {
        Self::new_with_envelope(account, Envelope::default())
    }

    pub fn new_with_envelope(account: &Account, envelope: Envelope) -> Self {
        // --------------------------
        // Envelope credentials
        // --------------------------
        let name = account.name.clone().unwrap_or(String::new());

        // set the data of the envelope
        let envelope = Envelope {
            // "from" and "signature" will be always set automatically for you
            from: vec![format!("{} <{}>", name, account.email)],
            signature: account.signature.clone(),
            // override some fields if you want to use them
            ..envelope
        };

        // ---------------------
        // Body credentials
        // ---------------------
        // provide an empty body
        let body = Attachment::new("", "text/plain", envelope.to_string().into_bytes());

        Self {
            attachments: vec![body],
            flags: Flags::new(&[]),
            envelope,
            // since the uid is set from the server, we will just set it to None
            uid: None,
            date: None,
        }
    }

    pub fn new_with_pre_body(account: &Account, raw: Vec<u8>) -> Result<Self> {
        let mut mail = Self::new(&account);

        let parsed = mailparse::parse_mail(&raw)?;

        mail.attachments
            .push(Attachment::new("", "text/plain", parsed.get_body_raw()?));

        Ok(mail)
    }

    pub fn change_to_reply(&mut self, account: &Account, reply_all: bool) {
        // ------------------
        // Adjust header
        // ------------------
        // Pick up the current subject of the mail
        let old_subject = self.envelope.subject.clone().unwrap_or(String::new());

        // The new fields
        let mut to: Vec<String> = Vec::new();
        let mut cc = None;

        // If we have to reply everyone, then we're not only replying to the
        // addresses in the `Reply-To` or `From:` field, we're also replying to
        // the addresses in the `To:` field and the `Cc: ` field.
        if reply_all {
            // the email addr parsed of the user
            let email_addr: lettre::Address = account.email.parse().unwrap();

            // Reply to all mail-addresses in the `To:` field, except the
            // mail-address of the current user who wants to write this
            // reply-message
            for addr in self.envelope.to.iter() {
                // each email address in the to field should be valid, why
                // should be in this header than?
                let addr_parsed: Mailbox = addr.parse().unwrap();

                // make sure that the address is not the mail of the current
                // user, because why should he/she wants to have the mail which
                // he/her just sent by themself?
                if addr_parsed.email != email_addr {
                    to.push(addr.to_string());
                }
            }

            // Also use the addresses in the "Cc:" field
            cc = self.envelope.cc.clone();
        }

        // Now add the addresses in the `Reply-To:` Field or from the `From:`
        // field.
        if let Some(reply_to) = &self.envelope.reply_to {
            to.append(&mut reply_to.clone());
        } else {
            // if the "Reply-To" wasn't set from the sender, then we're just
            // replying to the addresses in the "From:" field
            to.append(&mut self.envelope.from.clone());
        };

        // the message id of the mail.
        let message_id = self.envelope.message_id.clone().unwrap_or(String::new());

        let new_envelope = Envelope {
            from: vec![Envelope::convert_to_address(&account)],
            to,
            cc,
            subject: Some(format!("Re: {}", old_subject)),
            in_reply_to: Some(message_id),
            // and clear the rest of the fields
            ..Envelope::default()
        };

        self.envelope = new_envelope;

        // -----------------------
        // Remove Attachments
        // -----------------------
        // keep only the body of the mail!
        self.attachments = vec![self.attachments[0].clone()];

        // -------------------------
        // Prepare body of mail
        // -------------------------
        // comment "out" the body of the mail, by adding the `>` characters to
        // each line which includes a string.
        let new_body: String = String::from_utf8(self.attachments[0].body_raw.clone())
            .unwrap()
            .split('\n')
            .map(|line| format!("> {}", line))
            .collect::<Vec<String>>()
            .concat();

        // now apply our new body
        self.attachments[0].body_raw = new_body.into_bytes();
    }

    pub fn change_to_forwarding(&mut self) {
        // -----------
        // Header
        // -----------
        let old_subject = self.envelope.subject.clone().unwrap_or(String::new());

        self.envelope = Envelope {
            subject: Some(format!("Fwd: {}", old_subject)),
            // and use the rest of the headers
            ..self.envelope.clone()
        };

        // ---------
        // Body
        // ---------
        // apply a line which should indicate where the forwarded message begins
        let new_body = String::from_utf8(self.attachments[0].body_raw.clone()).unwrap();
        let new_body = format!(
            "\r\n---------- Forwarded Message ----------\r\n{}",
            new_body
        );

        self.attachments[0].body_raw = new_body.into_bytes();
    }

    /// Converts the whole mail into a vector of bytes.
    pub fn into_bytes(&mut self) -> Result<Vec<u8>> {
        // parse the whole mail first
        let parsed = self.to_sendable_msg()?;

        return Ok(parsed.formatted());
    }

    /// Let the user change the content of the mail. This function will change
    /// the first value of the `Msg.attachments` vector, since the first value
    /// of this vector represents the content of the mail.
    ///
    /// # Hint
    /// It *won't* change/update/set `Msg.parsed`!
    pub fn edit_body(&mut self) -> Result<()> {
        // ----------------
        // Update body
        // ----------------
        // get the old body of the mail
        let body = self.attachments[0].body_raw.clone();

        // store the changes of the body
        let content = input::open_editor_with_tpl(&body)?;
        self.set_body(content.into_bytes());

        Ok(())
    }

    pub fn set_body(&mut self, body_raw: Vec<u8>) {
        self.attachments[0].body_raw = body_raw;
    }

    // Add an attachment to the mail from the given path
    pub fn add_attachment(&mut self, path: &str) {
        if let Ok(new_attachment) = Attachment::try_from(path) {
            self.attachments.push(new_attachment);
        }
    }

    pub fn add_flag(&mut self, flag: Flag<'static>) {
        self.flags.insert(flag);
    }

    /// This function will use the information of the `Msg` struct and creates
    /// a sendable mail. It uses the `Msg.envelope` and `Msg.attachments`
    /// fields
    pub fn to_sendable_msg(&mut self) -> Result<Message> {
        // ==============
        // Preparing
        // ==============
        // Refresh the states of the envelope first before we're using them to
        // create the mail/msg.
        let parsed = mailparse::parse_mail(&self.attachments[0].body_raw)?;
        let refreshed_envelope = Envelope {
            // mailparse can't detect what the siganture is, so we just use the
            // old one again
            signature: self.envelope.signature.clone(),
            ..Self::parse_envelope(&parsed)
        };
        self.envelope = refreshed_envelope;

        dbg!("{:?}", &self.envelope);

        // ===================
        // Header of Msg
        // ===================
        // This variable will hold all information of our mail
        let mut msg = Message::builder();

        // ---------------------
        // Must-have-fields
        // ---------------------
        // add "from"
        for mailaddress in &self.envelope.from {
            msg = msg.from(match mailaddress.parse() {
                Ok(from) => from,
                Err(err) => return Err(ErrorKind::Header(
                        err.to_string(), "From", mailaddress.to_string()).into()),
            });
        }

        // add "to"
        for mailaddress in &self.envelope.to {
            msg = msg.to(match mailaddress.parse() {
                Ok(to) => to,
                Err(err) => return Err(ErrorKind::Header(
                        err.to_string(), "To", mailaddress.to_string()).into()),
            });
        }

        // --------------------
        // Optional fields
        // --------------------
        // add "sender"
        if let Some(sender) = &self.envelope.sender {
            msg = msg.sender(match sender.parse() {
                Ok(sender) => sender,
                Err(err) => return Err(ErrorKind::Header(
                        err.to_string(), "Sender", sender.to_string()).into()),
            });
        }

        // add "reply-to"
        if let Some(reply_to) = &self.envelope.reply_to {
            for mailaddress in reply_to {
                msg = msg.reply_to(match mailaddress.parse() {
                    Ok(reply_to) => reply_to,
                    Err(err) => return Err(ErrorKind::Header(
                            err.to_string(), "Reply-to", mailaddress.to_string()).into()),
                });
            }
        }

        // add "cc"
        if let Some(cc) = &self.envelope.cc {
            for mailaddress in cc {
                msg = msg.cc(match mailaddress.parse() {
                    Ok(cc) => cc,
                    Err(err) => return Err(ErrorKind::Header(
                            err.to_string(), "Cc", mailaddress.to_string()).into()),
                });
            }
        }

        // add "bcc"
        if let Some(bcc) = &self.envelope.bcc {
            for mailaddress in bcc {
                msg = msg.bcc(match mailaddress.parse() {
                    Ok(bcc) => bcc,
                    Err(err) => return Err(ErrorKind::Header(
                            err.to_string(), "Bcc", mailaddress.to_string()).into()),
                });
            }
        }

        // add "in_reply_to"
        if let Some(in_reply_to) = &self.envelope.in_reply_to {
            msg = msg.in_reply_to(match in_reply_to.parse() {
                Ok(in_reply_to) => in_reply_to,
                Err(err) => return Err(
                    ErrorKind::Header(err.to_string(), "In-Reply-To", in_reply_to.to_string()).into()),
            });
        }

        // -----------------------
        // Body + Attachments
        // -----------------------
        // In this part, we'll add the content of the mail. This means the body
        // and the attachments of the mail.

        // we will use this variable to iterate through our attachments
        let mut attachment_iter = self.attachments.iter();

        // get the content of the mail. Parse it and get the body afterwards.
        // Remember: The first value in the vector represents the body of the
        // mail, that's why we can just do `.next().unwrap()` to get the mail
        let mail_content = attachment_iter.next().unwrap();
        let body = mailparse::parse_mail(&mail_content.body_raw)?;
        let body = body.get_body_raw().unwrap();

        // this variable will store all "sections" or attachments of the mail
        let mut msg_parts = MultiPart::mixed().build();

        // add the body of the mail first
        let msg_body = SinglePart::builder()
            .header(mail_content.content_type.clone())
            .body(body.clone());
        msg_parts = msg_parts.singlepart(msg_body);

        // afterwards, add the rest of the attachments
        for attachment in attachment_iter {
            // Get the values of the attachment and convert them to the
            // Attachment-Struct of lettre.
            let msg_attachment = lettre_Attachment::new(attachment.filename.clone());
            let msg_attachment =
                msg_attachment.body(attachment.body_raw.clone(), attachment.content_type.clone());

            // add the attachment to our attachment-list
            msg_parts = msg_parts.singlepart(msg_attachment);
        }

        // Last but not least: Add the attachments to the header of the mail and
        // return the finished mail!
        Ok(msg.multipart(msg_parts)?)
    }

    /// This function will fetch the relevant information of the parsed mail for
    /// the envelopea and returns them.
    fn parse_envelope(parsed_mail: &mailparse::ParsedMail) -> Envelope {
        let mut new_envelope = Envelope::default();

        let header_iter = parsed_mail.headers.iter();
        for header in header_iter {
            // get the value of the header. For example if we have this header:
            //
            //  Subject: I use Arch btw
            //
            // than `value` would be like that: `let value = "I use Arch
            // btw".to_string()
            let value = header.get_value().replace("\r", "");
            let header_name = header.get_key().to_lowercase();
            let header_name = header_name.as_str();

            // now go through all headers and look which values they have.
            match header_name {
                "from" => {
                    new_envelope.from = value.rsplit(',').map(|addr| addr.to_string()).collect()
                }

                "to" => new_envelope.to = value.rsplit(',').map(|addr| addr.to_string()).collect(),

                "bcc" => {
                    new_envelope.bcc =
                        Some(value.rsplit(',').map(|addr| addr.to_string()).collect())
                }

                "cc" => {
                    new_envelope.cc = Some(value.rsplit(',').map(|addr| addr.to_string()).collect())
                }

                "in_reply_to" => new_envelope.in_reply_to = Some(value),

                "reply_to" => {
                    new_envelope.reply_to =
                        Some(value.rsplit(',').map(|addr| addr.to_string()).collect())
                }

                "sender" => new_envelope.sender = Some(value),

                "subject" => new_envelope.subject = Some(value),

                // it's a custom header => Add it to our
                // custom-header-hash-map
                _ => {
                    let custom_header = header.get_key();

                    // If we don't have a HashMap yet => Create one! Otherwise
                    // we'll keep using it, because why should we reset its
                    // values again?
                    if let None = new_envelope.custom_headers {
                        new_envelope.custom_headers = Some(HashMap::new());
                    }

                    // we can unwrap for sure, because with the if-condition
                    // above, we made sure, that the HashMap exists
                    let mut updated_hashmap = new_envelope.custom_headers.unwrap();

                    // now add the custom header to the hash table ..
                    updated_hashmap.insert(
                        custom_header,
                        value.rsplit(',').map(|addr| addr.to_string()).collect(),
                    );

                    // .. and apply the updated hashmap to the envelope struct
                    new_envelope.custom_headers = Some(updated_hashmap);
                }
            }
        }

        new_envelope
    }

    /// Returns the uid of the mail.
    ///
    /// # Hint
    /// The uid is only set from the server! So you can only get a `Some(...)`
    /// from this function, if it's a fetched mail otherwise you'll get `None`.
    pub fn get_uid(&self) -> Option<u32> {
        self.uid
    }

    pub fn get_body(&self) -> Result<String> {
        match String::from_utf8(self.attachments[0].body_raw.clone()) {
            Ok(body) => Ok(body),
            Err(err) => Err(ErrorKind::ParseBody(err.to_string()).into()),
        }
    }

    /// Returns an iterator which points to all attachments of the mail.
    pub fn get_attachments(&self) -> impl Iterator<Item = &Attachment> {
        // we are skipping the first attachment, because remember: The first
        // attachment is always the body of the mail! The rest in the vector are
        // other attachments, like images or something like that.
        self.attachments.iter().skip(1)
    }

    pub fn get_flags(&self) -> HashSet<Flag<'static>> {
        self.flags.clone()
    }

    pub fn get_flags_as_ref(&self) -> &HashSet<Flag> {
        &self.flags
    }
}

// -----------
// Traits
// -----------
impl Default for Msg {
    fn default() -> Self {
        Self {
            attachments: Vec::new(),
            flags: Flags::new(&[]),
            envelope: Envelope::default(),
            uid: None,
            date: None,
        }
    }
}

impl fmt::Display for Msg {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let body = match String::from_utf8(self.attachments[0].body_raw.clone()) {
            Ok(string) => string,
            Err(err) => {
                println!("[Error] Couldn't convert the body of the mail into a string:");
                println!("{}", err);
                String::from("Couldn't convert the body of the mail into a string.")
            }
        };

        // the signature
        let signature = &self.envelope.signature.clone().unwrap_or(String::new());

        writeln!(formatter, "{}\n{}\n{}", 
                 self.envelope.get_header_as_string(),
                 body,
                 signature)
    }
}

impl Table for Msg {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("UID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let is_seen = !self.flags.contains(&Flag::Seen);

        // The data which will be shown in the row
        let uid = self.get_uid().unwrap_or(0);
        let flags = self.flags.to_string();
        let subject = self.envelope.subject.clone().unwrap_or(String::new());
        let sender = self.envelope.sender.clone().unwrap_or(String::new());
        let date = self.date.clone().unwrap_or(String::new());

        Row::new()
            .cell(Cell::new(&uid.to_string()).bold_if(is_seen).red())
            .cell(Cell::new(&flags).bold_if(is_seen).white())
            .cell(Cell::new(&subject).shrinkable().bold_if(is_seen).green())
            .cell(Cell::new(&sender).bold_if(is_seen).blue())
            .cell(Cell::new(&date).bold_if(is_seen).yellow())
    }
}

// -----------
// From's
// -----------
/// Load the data from a fetched mail and store them in the mail-struct.
/// Please make sure that the fetch includes the following query:
///
///     - UID      (optional)
///     - FLAGS    (optional)
///     - ENVELOPE (optional)
///     - INTERNALDATE
///     - BODY[]   (optional)
///
impl From<&Fetch> for Msg {
    fn from(fetch: &Fetch) -> Msg {
        // -----------------
        // Preparations
        // -----------------
        // We're preparing the variables first, which will hold the data of the
        // fetched mail.

        // Here will be all attachments stored (including the body of the mail
        // as the first value of this vector)
        let mut attachments = Vec::new();

        // Get the flags of the mail
        let flags = Flags::new(fetch.flags());

        // Well, get the data of the envelope from the mail
        let envelope = Envelope::from(fetch.envelope());

        // Get the uid of the fetched mail
        let uid = fetch.uid;

        let date = fetch
            .internal_date()
            .map(|date| date.naive_local().to_string());

        // Get the content of the mail. Here we have to look (important!) if
        // the fetch even includes a body or not, since the `BODY[]` query is
        // only *optional*!
        let parsed =
            // the empty array represents an invalid body, so we can enter the
            // `Err` arm if the body-query wasn't applied
            match mailparse::parse_mail(fetch.body().unwrap_or(&[b' '])) {
                Ok(parsed) => Some(parsed),
                Err(_) => None,
            };

        // ---------------------------------
        // Storing the information (body)
        // ---------------------------------
        // We have to add at least one attachment, which should represent the
        // body of the mail. Since the body-query is only optional, we might
        // need to add an "empty" attachment.
        if let Some(parsed) = parsed {
            // Ok, so the body-query was applied to the fetch! Let's extract the
            // body then!
            match Attachment::from_parsed_mail(&parsed) {
                Some(mail_body) => attachments.push(mail_body),
                None => {
                    // Ok, so this shouldn't happen in general: We failed to get
                    // the body of the mail! Let's create a dummy with the
                    // content that it failed to load the body
                    let attachment_dummy = Attachment::new(
                        "",
                        "text/plain",
                        b"Couldn't get the body of the mail.".to_vec(),
                    );

                    attachments.push(attachment_dummy);
                }
            };

            // Go through all subparts of the mail and look if they are
            // attachments. If they are attachments:
            //  1. Get their filename
            //  2. Get the content of the attachment
            for subpart in &parsed.subparts {
                if let Some(attachment) = Attachment::from_parsed_mail(subpart) {
                    attachments.push(attachment);
                }
            }
        } else {
            // So the body-query wasn't applied. As a result we're gonna add an
            // empty body here, just for completeness and to make sure that each
            // access to the attachments-vector isn't invalid.
            attachments.push(Attachment::new("", "text/plain", Vec::new()));
        }

        Self {
            attachments,
            flags,
            envelope,
            uid,
            date,
        }
    }
}

// ==========
// Msgs
// ==========
/// This is just a type-safety which represents a vector of mails.
#[derive(Debug, Serialize)]
pub struct Msgs(pub Vec<Msg>);

impl Msgs {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

// -----------
// Traits
// -----------
impl fmt::Display for Msgs {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "\n{}", Table::render(&self.0))
    }
}

// -----------
// From's
// -----------
impl<'mails> From<&'mails ZeroCopy<Vec<Fetch>>> for Msgs {
    fn from(fetches: &'mails ZeroCopy<Vec<Fetch>>) -> Self {
        // the content of the Msgs-struct
        let mut mails = Vec::new();

        for fetch in fetches.iter().rev() {
            mails.push(Msg::from(fetch));
        }

        Self(mails)
    }
}
