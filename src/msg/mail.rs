use super::attachment::Attachment;
use super::envelope::Envelope;

use imap::types::{Flag, Fetch};

use mailparse::{self, ParsedMail};

use crate::config::model::Account;
use crate::input;

use lettre::message::{
    Message,
    MultiPart,
    SinglePart,
    Attachment as lettre_Attachment,
};

use std::convert::TryFrom;
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
}

// ============
// Structs
// ============
#[derive(Debug)]
pub struct Mail<'mail> {

    /// All added attachments are listed in this vector.
    pub attachments: Vec<Attachment>,

    /// The flags for this mail.
    pub flags: Vec<Flag<'mail>>,

    /// All information of the envelope (sender, from, to and so on)
    pub envelope: Envelope,

    /// The parsed content of the mail which shoud make it easier to access
    pub parsed: Option<ParsedMail<'mail>>,
}

impl<'mail> Mail<'mail> {

    pub fn new(account: &Account) -> Self {
        Self::new_with_envelope(account, Envelope::default())
    }

    pub fn new_with_envelope(account: &Account, envelope: Envelope) -> Self {
        // --------------------------
        // Envelope credentials
        // --------------------------
        let name = account.name
            .clone()
            .unwrap_or(String::new());

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
            flags: Vec::new(),
            envelope,
            parsed: None,
        }
    }

    /// Converts the whole mail into a vector of bytes.
    pub fn into_bytes(&self) -> Result<Vec<u8>, MailError> {
        // parse the whole mail first
        let parsed = match self.to_sendable_msg() {
            Ok(parsed) => parsed,
            Err(_) => return Err(MailError::MakingSendable),
        };

        return Ok(parsed.formatted())
    }

    /// Let the user change the content of the mail. This function will change
    /// the first value of the `Mail.attachments` vector, since the first value
    /// of this vector represents the content of the mail.
    /// 
    /// # Hint
    /// It *won't* change/update/set `Mail.parsed`!
    pub fn edit_body(&mut self) {

        // get the old body of the mail
        let body = self.attachments[0].body_raw.clone();

        // TODO: Error handling
        // store the changes of the body
        self.attachments[0].body_raw = match input::open_editor_with_tpl(&body) {
            Ok(content) => content.into_bytes(),
            Err(_) => String::from("").into_bytes(),
        };
    }

    // TODO: Error handling
    pub fn add_attachment(&mut self, path: &str) {
        if let Ok(new_attachment) = Attachment::try_from(path) {
            self.attachments.push(new_attachment);
        }
    }

    pub fn get_flags_as_string(&self) -> String {
        let mut flags = String::new();
        let flag_symbols = vec![
            (Flag::Seen, '*'),
            (Flag::Answered, '↵'),
            (Flag::Flagged, '!')
        ];

        for symbol in &flag_symbols {
            if self.flags.contains(&symbol.0) {
                flags.push(symbol.1);
            }
        }

        flags
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
            msg = msg.from(mailaddress.parse().unwrap());
        }

        // add "to"
        for mailaddress in &self.envelope.to {
            msg = msg.to(mailaddress.parse().unwrap());
        }

        // --------------------
        // Optional fields
        // --------------------
        // add "sender"
        if let Some(sender) = &self.envelope.sender {
            for mailaddress in sender {
                msg = msg.sender(mailaddress.parse().unwrap());
            }
        }

        // add "reply-to"
        if let Some(reply_to) = &self.envelope.reply_to {
            for mailaddress in reply_to {
                msg = msg.reply_to(mailaddress.parse().unwrap());
            }
        }

        // add "cc"
        if let Some(cc) = &self.envelope.cc {
            for mailaddress in cc {
                msg = msg.cc(mailaddress.parse().unwrap());
            }
        }

        // add "bcc"
        if let Some(bcc) = &self.envelope.bcc {
            for mailaddress in bcc {
                msg = msg.bcc(mailaddress.parse().unwrap());
            }
        }

        // add "in_reply_to"
        if let Some(in_reply_to) = &self.envelope.in_reply_to {
            msg = msg.in_reply_to(in_reply_to.clone());
        }

        // -----------------------
        // Body + Attachments
        // -----------------------
        // In this part, we'll add the content of the mail. This means the body
        // and the attachments of the mail.

        // we will use this variable to iterate through our attachments
        let mut attachment_iter = self.attachments.iter();

        // get the content of the mail. Parse it and get the body afterwards
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

            // Get the values of the attachment and convert them
            let msg_attachment = lettre_Attachment::new(attachment.filename.clone());
            let msg_attachment = msg_attachment.body(
                attachment.body_raw.clone(), attachment.content_type.clone());

            // add the attachment to our attachment-list
            msg_parts = msg_parts.singlepart(msg_attachment);
        }

        // Last but not least: Add the attachments to the header of the mail and
        // return the finished mail!
        Ok(msg.multipart(msg_parts).unwrap())
    }
}

// ==================
// Common Traits
// ==================
impl<'mail> Default for Mail<'mail> {
    fn default() -> Self {
        Self {
            attachments: Vec::new(),
            flags: Vec::new(),
            envelope: Envelope::default(),
            parsed: None
        }
    }
}

// impl<'mail> fmt::Display for Mail<'mail> {
//     fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//         let result = String::new();
//
//         // ------------
//         // Headers
//         // ------------
//
//         write!(formatter, "{}", result)
//     }
// }

// ===========
// From's
// ===========
impl<'mail> TryFrom<&'mail Fetch> for Mail<'mail> {

    type Error = MailError;

    fn try_from(fetch: &'mail Fetch) -> Result<Mail, MailError> {

        // Here will be all attachments stored
        let mut attachments = Vec::new();

        // Get the flags of the mail
        let flags = fetch.flags().to_vec();

        // Well, get the data of the envelope from the mail
        let envelope = Envelope::from(fetch.envelope());

        // Get the parsed-version of the mail
        let parsed = match mailparse::parse_mail(fetch.body().unwrap_or(&[b' '])) {
            Ok(parsed) => parsed,
            Err(_) => return Err(MailError::ParseBody),
        };

        // Go through all subparts of the mail and look if they are attachments.
        // If they are attachments:
        //  1. Get their filename
        //  2. Get the content of the attachment
        for subpart in &parsed.subparts {
            if let Ok(attachment) = Attachment::try_from(subpart) {
                attachments.push(attachment);
            }
        }

        Ok(Self {
            attachments,
            flags,
            envelope,
            parsed: Some(parsed),
        })
    }
}
