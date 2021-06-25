use super::attachment::Attachment;
use super::envelope::Envelope;

use imap::types::{Fetch, Flag, ZeroCopy};

use mailparse::{self, MailParseError};

use crate::config::model::Account;
use crate::flag::model::Flags;
use crate::input;
use crate::table::{Cell, Row, Table};

use serde::Serialize;

use lettre::message::{
    Attachment as lettre_Attachment, Message, MultiPart, SinglePart,
};

use std::collections::HashMap;
use std::convert::{From, TryFrom};
use std::fmt;

// ===============
// Error enum
// ===============
/// This enums are used for errors which might happen when trying to parse the
/// mail from the given fetch.
#[derive(Debug)]
pub enum MailError {
    /// An error appeared, when it tried to parse the body of the mail!
    ParseBody,

    /// An error appeared, when it tried to parse/get an attachment!
    ParseAttachment,

    MakingSendable,

    ParseFetch,
}

// =========
// Mail
// =========
#[derive(Debug, Serialize)]
pub struct Mail<'mail> {
    /// All added attachments are listed in this vector.
    pub attachments: Vec<Attachment>,

    /// The flags for this mail.
    pub flags: Flags<'mail>,

    /// All information of the envelope (sender, from, to and so on)
    pub envelope: Envelope,

    /// The UID of the mail. It's only set from the server. So that's why it's
    /// not public: To avoid that it's set manually!
    uid: Option<u32>,

    date: Option<String>,
}

impl<'mail> Mail<'mail> {
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
        let body = Attachment::new(
            "",
            "text/plain",
            envelope.to_string().into_bytes(),
        );

        Self {
            attachments: vec![body],
            flags: Flags::new(&[]),
            envelope,
            // since the uid is set from the server, we will just set it to None
            uid: None,
            date: None,
        }
    }

    /// Converts the whole mail into a vector of bytes.
    pub fn into_bytes(&self) -> Result<Vec<u8>, MailError> {
        // parse the whole mail first
        let parsed = match self.to_sendable_msg() {
            Ok(parsed) => parsed,
            Err(_) => return Err(MailError::MakingSendable),
        };

        return Ok(parsed.formatted());
    }

    /// Let the user change the content of the mail. This function will change
    /// the first value of the `Mail.attachments` vector, since the first value
    /// of this vector represents the content of the mail.
    ///
    /// # Hint
    /// It *won't* change/update/set `Mail.parsed`!
    pub fn edit_body(&mut self) -> Result<(), MailParseError> {
        // ----------------
        // Update body
        // ----------------
        // get the old body of the mail
        let body = self.attachments[0].body_raw.clone();

        // TODO: Error handling
        // store the changes of the body
        self.attachments[0].body_raw = match input::open_editor_with_tpl(&body)
        {
            Ok(content) => content.into_bytes(),
            Err(_) => String::from("").into_bytes(),
        };

        // ------------------------
        // Reevaluate Envelope
        // ------------------------
        // Since the user changed the content of the mail, he/she could also
        // change the headers by adding addresses for example to the `Cc:`
        // header, as a result, we have to update our envelope-status as well!

        // TODO: Reparse the mail and update the headers
        let parsed = match mailparse::parse_mail(&self.attachments[0].body_raw)
        {
            Ok(parsed) => parsed,
            Err(err) => return Err(err),
        };

        // now look which headers are given and update the values of the
        // envelope struct. We are creating a new envelope-template for that and
        // take only the important values with us which the user can't provide
        let mut new_envelope = Envelope {
            signature: self.envelope.signature.clone(),
            ..Envelope::default()
        };
        let header_iter = parsed.headers.iter();
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
                    new_envelope.from =
                        value.rsplit(',').map(|addr| addr.to_string()).collect()
                }

                "to" => {
                    new_envelope.to =
                        value.rsplit(',').map(|addr| addr.to_string()).collect()
                }

                "bcc" => {
                    new_envelope.bcc = Some(
                        value
                            .rsplit(',')
                            .map(|addr| addr.to_string())
                            .collect(),
                    )
                }

                "cc" => {
                    new_envelope.cc = Some(
                        value
                            .rsplit(',')
                            .map(|addr| addr.to_string())
                            .collect(),
                    )
                }

                "in_reply_to" => new_envelope.in_reply_to = Some(value),

                "reply_to" => {
                    new_envelope.reply_to = Some(
                        value
                            .rsplit(',')
                            .map(|addr| addr.to_string())
                            .collect(),
                    )
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
                    let mut updated_hashmap =
                        new_envelope.custom_headers.unwrap();

                    // now add the custom header to the hash table ..
                    updated_hashmap.insert(
                        custom_header,
                        value
                            .rsplit(',')
                            .map(|addr| addr.to_string())
                            .collect(),
                    );

                    // .. and apply the updated hashmap to the envelope struct
                    new_envelope.custom_headers = Some(updated_hashmap);
                }
            }
        }

        // apply the new envelope headers
        self.envelope = new_envelope;

        Ok(())
    }

    // TODO: Error handling
    pub fn add_attachment(&mut self, path: &str) {
        if let Ok(new_attachment) = Attachment::try_from(path) {
            self.attachments.push(new_attachment);
        }
    }

    /// This function will use the information of the `Mail` struct and creates
    /// a sendable mail. It uses the `Mail.envelope` and `Mail.attachments`
    /// fields
    pub fn to_sendable_msg(&self) -> Result<Message, MailError> {
        // ===================
        // Header of Mail
        // ===================
        // This variable will hold all information of our mail
        let mut msg = Message::builder();

        // ---------------------
        // Must-have-fields
        // ---------------------
        // add "from"
        for mailaddress in &self.envelope.from {
            msg = msg.from(match mailaddress.parse() {
                Ok(msg) => msg,
                Err(_) => {
                    error_msg_forgot_header("From");
                    return Err(MailError::MakingSendable);
                }
            });
        }

        // add "to"
        for mailaddress in &self.envelope.to {
            msg = msg.to(match mailaddress.parse() {
                Ok(msg) => msg,
                Err(_) => {
                    error_msg_forgot_header("To");
                    return Err(MailError::MakingSendable);
                }
            });
        }

        // --------------------
        // Optional fields
        // --------------------
        // add "sender"
        if let Some(sender) = &self.envelope.sender {
            msg = msg.sender(match sender.parse() {
                Ok(msg) => msg,
                Err(_) => {
                    error_msg_forgot_header("Sender");
                    return Err(MailError::MakingSendable);
                }
            });
        }

        // add "reply-to"
        if let Some(reply_to) = &self.envelope.reply_to {
            for mailaddress in reply_to {
                msg = msg.reply_to(match mailaddress.parse() {
                    Ok(msg) => msg,
                    Err(_) => {
                        error_msg_forgot_header("Reply-To");
                        return Err(MailError::MakingSendable);
                    }
                });
            }
        }

        // add "cc"
        if let Some(cc) = &self.envelope.cc {
            for mailaddress in cc {
                msg = msg.cc(match mailaddress.parse() {
                    Ok(msg) => msg,
                    Err(_) => {
                        error_msg_forgot_header("Cc");
                        return Err(MailError::MakingSendable);
                    }
                });
            }
        }

        // add "bcc"
        if let Some(bcc) = &self.envelope.bcc {
            for mailaddress in bcc {
                msg = msg.bcc(match mailaddress.parse() {
                    Ok(msg) => msg,
                    Err(_) => {
                        error_msg_forgot_header("Bcc");
                        return Err(MailError::MakingSendable);
                    }
                });
            }
        }

        // add "in_reply_to"
        if let Some(in_reply_to) = &self.envelope.in_reply_to {
            msg = msg.in_reply_to(match in_reply_to.parse() {
                Ok(msg) => msg,
                Err(_) => {
                    error_msg_forgot_header("In-Reply-To");
                    return Err(MailError::MakingSendable);
                }
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
        let body = mailparse::parse_mail(&mail_content.body_raw).unwrap();
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
            let msg_attachment =
                lettre_Attachment::new(attachment.filename.clone());
            let msg_attachment = msg_attachment.body(
                attachment.body_raw.clone(),
                attachment.content_type.clone(),
            );

            // add the attachment to our attachment-list
            msg_parts = msg_parts.singlepart(msg_attachment);
        }

        // Last but not least: Add the attachments to the header of the mail and
        // return the finished mail!
        match msg.multipart(msg_parts) {
            Ok(msg_prepared) => Ok(msg_prepared),
            Err(err) => {
                println!("{}", err);
                Err(MailError::MakingSendable)
            }
        }
    }

    /// Returns the uid of the mail.
    ///
    /// # Hint
    /// The uid is only set from the server! So you can only get a `Some(...)`
    /// from this function, if it's a fetched mail otherwise you'll get `None`.
    pub fn get_uid(&self) -> Option<u32> {
        self.uid
    }
}

// -----------
// Traits
// -----------
impl<'mail> Default for Mail<'mail> {
    fn default() -> Self {
        Self {
            attachments: Vec::new(),
            flags:       Flags::new(&[]),
            envelope:    Envelope::default(),
            uid:         None,
            date: None,
        }
    }
}

impl<'mail> Table for Mail<'mail> {
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
///     - UID
///     - FLAGS
///     - ENVELOPE
///     - INTERNALDATE
///     - BODY[] (optional)
impl<'mail> From<&'mail Fetch> for Mail<'mail> {
    fn from(fetch: &'mail Fetch) -> Mail {
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
            match Attachment::try_from(&parsed) {
                Ok(mail_body) => attachments.push(mail_body),
                Err(_) => {
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
                if let Ok(attachment) = Attachment::try_from(subpart) {
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
// Mails
// ==========
/// This is just a type-safety which represents a vector of mails.
#[derive(Debug, Serialize)]
pub struct Mails<'mails>(pub Vec<Mail<'mails>>);

impl<'mails> Mails<'mails> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

// -----------
// Traits
// -----------
impl<'mails> fmt::Display for Mails<'mails> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "\n{}", Table::render(&self.0))
    }
}

// -----------
// From's
// -----------
impl<'mails> From<&'mails ZeroCopy<Vec<Fetch>>> for Mails<'mails> {
    fn from(fetches: &'mails ZeroCopy<Vec<Fetch>>) -> Self {
        // the content of the Mails-struct
        let mut mails = Vec::new();

        for fetch in fetches.iter().rev() {
            mails.push(Mail::from(fetch));
        }

        Self(mails)
    }
}

// =====================
// Helper Functions
// =====================
/// # Usages
/// It's only used in the `Mail::to_sendable_msg` function and is called, if the
/// user forgot to enter a value into a header.
///
/// # Example
/// If you run `error_msg_forgot_header("From")`, then this message will be
/// printed out in the console:
///
///     [ERROR] Value is missing in the 'From:' header!
///     Please edit your mail again and enter a value in it!
fn error_msg_forgot_header(header_name: &str) {
    println!("[ERROR] Value is missing in the '{}:' header!", header_name);
    println!("Please edit your mail again and enter a value in it!");
}
