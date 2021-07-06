use super::attachment::Attachment;
use super::body::Body;
use super::envelope::Envelope;

use log::debug;

use imap::types::{Fetch, Flag, ZeroCopy};

use mailparse;

use crate::{
    config::model::Account,
    flag::model::Flags,
    table::{Cell, Row, Table},
};

#[cfg(not(test))]
use crate::input;

use serde::Serialize;

use lettre::message::{Attachment as lettre_Attachment, Mailbox, Message, MultiPart, SinglePart};

use std::convert::{From, TryFrom};
use std::fmt;

use colorful::Colorful;

error_chain::error_chain! {
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
        FromUtf8Error(std::string::FromUtf8Error);
    }
}

// =========
// Msg
// =========
/// This struct represents a whole mail/msg with its attachments, body-content
/// and its envelope.
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct Msg {
    /// All added attachments are listed in this vector.
    pub attachments: Vec<Attachment>,

    /// The flags of this mail.
    pub flags: Flags,

    /// All information of the envelope (sender, from, to and so on)
    // envelope: HashMap<HeaderName, Vec<String>>,
    pub envelope: Envelope,

    /// This variable stores the body of the msg.
    pub body: Body,

    /// The UID of the mail. It's only set from the server!
    uid: Option<u32>,

    /// The origination date field. Read [the RFC here] here for more
    /// information.
    ///
    /// [the RFC here]:
    /// https://www.rfc-editor.org/rfc/rfc5322.html#section-3.6.1
    date: Option<String>,

    /// The msg but in raw.
    raw: Vec<u8>,
}

impl Msg {
    /// Creates a completely new msg where two header fields are set:
    /// - [`from`]
    /// - and [`signature`]
    ///
    /// [`from`]: struct.Envelope.html#structfield.from
    /// [`signature`]: struct.Envelope.html#structfield.signature
    ///
    /// # Example
    ///
    /// <details>
    ///
    /// ```
    /// # use himalaya::msg::model::Msg;
    /// # use himalaya::msg::envelope::Envelope;
    /// # use himalaya::config::model::Account;
    /// # fn main() {
    /// // -------------
    /// // Accounts
    /// // -------------
    /// let account1 = Account::new(Some("Soywod"), "clement.douin@posteo.net");
    /// let account2 = Account::new(None, "tornax07@gmail.com");
    ///
    /// // ---------------------
    /// // Creating message
    /// // ---------------------
    /// let msg1 = Msg::new(&account1);
    /// let msg2 = Msg::new(&account2);
    ///
    /// let expected_envelope1 = Envelope {
    ///     from: vec![String::from("Soywod <clement.douin@posteo.net>")],
    ///     // the signature of the account is stored as well
    ///     signature: Some(String::from("Account Signature")),
    ///     .. Envelope::default()
    /// };
    ///
    /// let expected_envelope2 = Envelope {
    ///     from: vec![String::from("tornax07@gmail.com")],
    ///     signature: Some(String::from("Account Signature")),
    ///     .. Envelope::default()
    /// };
    ///
    /// assert_eq!(msg1.envelope, expected_envelope1);
    /// assert_eq!(msg2.envelope, expected_envelope2);
    /// # }
    /// ```
    ///
    /// </details>
    ///
    pub fn new(account: &Account) -> Self {
        Self::new_with_envelope(account, Envelope::default())
    }

    /// This function does the same as [`Msg::new`] but you can apply a custom
    /// [`envelope`] when calling the function instead of using the default one
    /// from the [`Msg::new`] function.
    ///
    /// [`Msg::new`]: struct.Msg.html#method.new
    /// [`envelope`]: struct.Envelope.html
    pub fn new_with_envelope(account: &Account, mut envelope: Envelope) -> Self {
        // --------------------------
        // Envelope credentials
        // --------------------------
        if envelope.from.is_empty() {
            envelope.from = vec![account.get_full_address()];
        }

        if let None = envelope.signature {
            envelope.signature = account.signature.clone();
        }
        // ---------------------
        // Body credentials
        // ---------------------
        let body = Body::from(envelope.signature.clone().unwrap_or_default());

        Self {
            attachments: Vec::new(),
            flags: Flags::new(&[]),
            envelope,
            body,
            // since the uid is set from the server, we will just set it to None
            uid: None,
            date: None,
            raw: Vec::new(),
        }
    }

    /// Converts the message into a Reply message. It'll set the headers
    /// differently depending on the value of `reply_all`.
    ///
    /// # Changes
    /// The value on the left side, represents the header *after* the function
    /// call, while the value on the right side shows the data *before* the
    /// function call. So if we pick up the first example of `reply_all =
    /// false`, then we can see, that the value of `ReplyTo:` is moved into the
    /// `To:` header field in this function call.
    ///
    /// - `reply_all = false`:
    ///     - `To:` = `ReplyTo:` otherwise from `From:`
    ///     - attachments => cleared
    ///     - `From:` = Emailaddress of the current user account
    ///     - `Subject:` = "Re:" + `Subject`
    ///     - `in_reply_to` = Old Message ID
    ///     - `Cc:` = cleared
    ///
    /// - `reply_all = true`:
    ///     - `To:` = `ReplyTo:` + Addresses in `To:`
    ///     - `Cc:` = All CC-Addresses
    ///     - The rest: Same as in `reply_all = false`
    ///
    /// It'll add for each line in the body the `>` character in the beginning
    /// of each line.
    ///
    /// # Example
    /// [Here] you can see an example how a discussion with replies could look
    /// like.
    ///
    /// [Here]: https://www.rfc-editor.org/rfc/rfc5322.html#page-46
    pub fn change_to_reply(&mut self, account: &Account, reply_all: bool) -> Result<()> {
        // ------------------
        // Adjust header
        // ------------------
        // Pick up the current subject of the mail
        let old_subject = self.envelope.subject.clone().unwrap_or(String::new());

        // The new fields
        let mut to: Vec<String> = Vec::new();
        let mut cc = None;

        if reply_all {
            let email_addr: lettre::Address = account.email.parse()?;

            for addr in self.envelope.to.iter() {
                let addr_parsed: Mailbox = addr.parse()?;

                // we don't want to receive the mail which we have just sent,
                // don't we?
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

        let message_id = self.envelope.message_id.clone().unwrap_or(String::new());

        let new_envelope = Envelope {
            from: vec![account.get_full_address()],
            to,
            cc,
            subject: Some(format!("Re: {}", old_subject)),
            in_reply_to: Some(message_id),
            // and clear the rest of the fields
            ..Envelope::default()
        };

        self.envelope = new_envelope;

        // remove the attachments
        self.attachments.clear();

        // -------------------------
        // Prepare body of mail
        // -------------------------
        // comment "out" the body of the mail, by adding the `>` characters to
        // each line which includes a string.
        let new_body: String = self
            .body
            .clone()
            .split('\n')
            .map(|line| format!("> {}\n", line))
            .collect::<Vec<String>>()
            .concat();

        // now apply our new body
        self.body = Body::from(new_body);

        Ok(())
    }

    /// Changes the msg/mail to a forwarding msg/mail.
    ///
    /// # Changes
    /// Calling this function will change apply the following to the current
    /// message:
    ///
    /// - `Subject:`: `"Fwd: "` will be added in front of the "old" subject
    /// - `"---------- Forwarded Message ----------"` will be added on top of
    ///     the body.
    ///
    /// # Example
    /// ```text
    /// Subject: Test subject
    /// ...
    ///
    /// Hi,
    /// I use Himalaya
    ///
    /// Sincerely
    /// ```
    ///
    /// will be changed to
    ///
    /// ```text
    /// Subject: Fwd: Test subject
    /// Sender: <Your@address>
    /// ...
    ///
    /// > Hi,
    /// > I use Himalaya
    /// >
    /// > Sincereley
    /// ```
    ///
    pub fn change_to_forwarding(&mut self, account: &Account) {
        // -----------
        // Header
        // -----------
        let old_subject = self.envelope.subject.clone().unwrap_or(String::new());

        self.envelope = Envelope {
            subject: Some(format!("Fwd: {}", old_subject)),
            sender: Some(account.get_full_address()),
            // and use the rest of the headers
            ..self.envelope.clone()
        };

        // ---------
        // Body
        // ---------
        // apply a line which should indicate where the forwarded message begins
        self.body = Body::from(format!(
            "\r\n---------- Forwarded Message ----------\r\n{}",
            &self.body,
        ));
    }

    /// Converts the mail into a **sendable message** (by calling the
    /// [`to_sendable_msg`] function) and converts it **afterwards** into a
    /// vector of bytes.
    ///
    /// [`to_sendable_msg`]: struct.Msg.html#method.to_sendable_msg
    pub fn into_bytes(&mut self) -> Result<Vec<u8>> {
        // parse the whole mail first
        let parsed = self.to_sendable_msg()?;

        return Ok(parsed.formatted());
    }

    /// Let the user edit the body of the mail.
    ///
    /// It'll enter the headers of the envelope into the draft-file *if they're
    /// not [`None`]!*.
    ///
    /// # Example
    /// ```no_run
    /// use himalaya::msg::model::Msg;
    /// use himalaya::config::model::Account;
    ///
    /// fn main() {
    ///     let account = Account::new(Some("Name"), "some@mail.asdf");
    ///     let mut msg = Msg::new(&account);
    ///
    ///     // In this case, only the header fields "From:" and "To:" are gonna
    ///     // be editable, because the other envelope fields are set to "None"
    ///     // per default!
    ///     msg.edit_body().unwrap();
    /// }
    /// ```
    ///
    /// Now enable some headers:
    ///
    /// ```no_run
    /// use himalaya::msg::{model::Msg, envelope::Envelope};
    /// use himalaya::config::model::Account;
    ///
    /// fn main() {
    ///     let account = Account::new(Some("Name"), "some@mail.asdf");
    ///     let mut msg = Msg::new_with_envelope(
    ///         &account,
    ///         Envelope {
    ///             bcc: Some(Vec::new()),
    ///             cc: Some(Vec::new()),
    ///             .. Envelope::default()
    ///         });
    ///
    ///     // The "Bcc:" and "Cc:" header fields are gonna be editable as well
    ///     msg.edit_body().unwrap();
    /// }
    /// ```
    ///
    /// # Errors
    /// In generel an error should appear if
    /// - The draft or changes couldn't be saved
    /// - The changed mail can't be parsed! (You wrote some things wrong...)
    pub fn edit_body(&mut self) -> Result<()> {
        // First of all, we need to create our template for the user. This
        // means, that the header needs to be added as well!
        let body = format!("{}\n{}", self.envelope.get_header_as_string(), self.body);

        // let's change the body! We don't let this line compile, if we're doing
        // tests, because we just need to look, if the headers are set
        // correctly
        #[cfg(not(test))]
        let body = input::open_editor_with_tpl(body.as_bytes())?;

        // now we have to split whole mail into their headers and the body
        self.parse_from_str(&body)?;

        Ok(())
    }

    pub fn parse_from_str(&mut self, content: &str) -> Result<()> {
        let parsed = mailparse::parse_mail(content.as_bytes())?;

        self.envelope = Envelope::from(&parsed);

        if let Ok(body) = parsed.get_body() {
            self.body = Body::from(body);
        }

        Ok(())
    }

    // Add an attachment to the mail from the given path
    // TODO: Error handling
    pub fn add_attachment(&mut self, path: &str) {
        if let Ok(new_attachment) = Attachment::try_from(path) {
            self.attachments.push(new_attachment);
        }
    }

    /// This function will use the information of the `Msg` struct and creates
    /// a sendable mail. It uses the `Msg.envelope` and `Msg.attachments`
    /// fields
    pub fn to_sendable_msg(&mut self) -> Result<Message> {
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
                Err(err) => {
                    return Err(
                        ErrorKind::Header(err.to_string(), "From", mailaddress.to_string()).into(),
                    )
                }
            });
        }

        // add "to"
        for mailaddress in &self.envelope.to {
            msg = msg.to(match mailaddress.parse() {
                Ok(to) => to,
                Err(err) => {
                    return Err(
                        ErrorKind::Header(err.to_string(), "To", mailaddress.to_string()).into(),
                    )
                }
            });
        }

        // --------------------
        // Optional fields
        // --------------------
        // add "sender"
        if let Some(sender) = &self.envelope.sender {
            msg = msg.sender(match sender.parse() {
                Ok(sender) => sender,
                Err(err) => {
                    return Err(
                        ErrorKind::Header(err.to_string(), "Sender", sender.to_string()).into(),
                    )
                }
            });
        }

        // add "reply-to"
        if let Some(reply_to) = &self.envelope.reply_to {
            for mailaddress in reply_to {
                msg = msg.reply_to(match mailaddress.parse() {
                    Ok(reply_to) => reply_to,
                    Err(err) => {
                        return Err(ErrorKind::Header(
                            err.to_string(),
                            "Reply-to",
                            mailaddress.to_string(),
                        )
                        .into())
                    }
                });
            }
        }

        // add "cc"
        if let Some(cc) = &self.envelope.cc {
            for mailaddress in cc {
                msg = msg.cc(match mailaddress.parse() {
                    Ok(cc) => cc,
                    Err(err) => {
                        return Err(ErrorKind::Header(
                            err.to_string(),
                            "Cc",
                            mailaddress.to_string(),
                        )
                        .into())
                    }
                });
            }
        }

        // add "bcc"
        if let Some(bcc) = &self.envelope.bcc {
            for mailaddress in bcc {
                msg = msg.bcc(match mailaddress.parse() {
                    Ok(bcc) => bcc,
                    Err(err) => {
                        return Err(ErrorKind::Header(
                            err.to_string(),
                            "Bcc",
                            mailaddress.to_string(),
                        )
                        .into())
                    }
                });
            }
        }

        // add "in_reply_to"
        if let Some(in_reply_to) = &self.envelope.in_reply_to {
            msg = msg.in_reply_to(match in_reply_to.parse() {
                Ok(in_reply_to) => in_reply_to,
                Err(err) => {
                    return Err(ErrorKind::Header(
                        err.to_string(),
                        "In-Reply-To",
                        in_reply_to.to_string(),
                    )
                    .into())
                }
            });
        }

        // -----------------------
        // Body + Attachments
        // -----------------------
        // In this part, we'll add the content of the mail. This means the body
        // and the attachments of the mail.

        // this variable will store all "sections" or attachments of the mail
        let mut msg_parts = MultiPart::mixed().build();

        // -- Body --
        // add the body of the mail first
        let msg_body = SinglePart::plain(self.body.get_content());
        msg_parts = msg_parts.singlepart(msg_body);

        // -- Attachments --
        // afterwards, add the rest of the attachments
        for attachment in self.attachments.iter() {
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

    /// Returns the uid of the mail.
    ///
    /// # Hint
    /// The uid is only set from the server! So you can only get a `Some(...)`
    /// from this function, if it's a fetched mail otherwise you'll get `None`.
    pub fn get_uid(&self) -> Option<u32> {
        self.uid
    }

    pub fn get_raw(&self) -> Result<String> {
        let raw_message = String::from_utf8(self.raw.clone()).chain_err(|| {
            format!(
                "[{}]: Couldn't get the raw body of the msg/mail.",
                "Error".red()
            )
        })?;

        Ok(raw_message)
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
            body: Body::default(),
            uid: None,
            date: None,
            raw: Vec::new(),
        }
    }
}

impl fmt::Display for Msg {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            formatter,
            "{}\n{}",
            self.envelope.get_header_as_string(),
            self.body,
        )
    }
}

impl Table for Msg {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("UID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("FROM").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let is_seen = !self.flags.contains(&Flag::Seen);

        // The data which will be shown in the row
        let uid = self.get_uid().unwrap_or(0);
        let flags = self.flags.to_string();
        let subject = self.envelope.subject.clone().unwrap_or_default();
        let mut from = String::new();
        let date = self.date.clone().unwrap_or(String::new());

        for from_addr in self.envelope.from.iter() {
            from.push_str(&from_addr);
        }

        Row::new()
            .cell(Cell::new(&uid.to_string()).bold_if(is_seen).red())
            .cell(Cell::new(&flags).bold_if(is_seen).white())
            .cell(Cell::new(&subject).shrinkable().bold_if(is_seen).green())
            .cell(Cell::new(&from).bold_if(is_seen).blue())
            .cell(Cell::new(&date).bold_if(is_seen).yellow())
    }
}

// -----------
// From's
// -----------
/// Load the data from a fetched mail and store them in the mail-struct.
/// Please make sure that the fetch includes the following query:
///
/// - UID      (optional)
/// - FLAGS    (optional)
/// - ENVELOPE (optional)
/// - INTERNALDATE
/// - BODY[]   (optional)
///
impl TryFrom<&Fetch> for Msg {
    type Error = Error;

    fn try_from(fetch: &Fetch) -> Result<Msg> {
        // -----------------
        // Preparations
        // -----------------
        // We're preparing the variables first, which will hold the data of the
        // fetched mail.

        // Here will be all attachments stored
        let mut attachments = Vec::new();

        // Get the flags of the mail
        let flags = Flags::new(fetch.flags());

        // Well, get the data of the envelope from the mail
        let envelope = Envelope::try_from(fetch.envelope())?;

        // Get the uid of the fetched mail
        let uid = fetch.uid;

        let date = fetch
            .internal_date()
            .map(|date| date.naive_local().to_string());

        // IDEA: Store raw body here
        // println!("{}", String::from_utf8(fetch.body().unwrap().to_vec()).unwrap());
        let raw = match fetch.body() {
            Some(body) => body.to_vec(),
            None => Vec::new(),
        };

        // Get the content of the mail. Here we have to look (important!) if
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

        // ---------------------------------
        // Storing the information (body)
        // ---------------------------------
        let mut body = String::new();
        if let Some(parsed) = parsed {
            // Ok, so some mails have their mody wrapped in a multipart, some
            // don't. This condition hits, if the body isn't in a multipart
            if parsed.ctype.mimetype == "text/plain" {
                // Apply the body (if there exists one)
                if let Ok(parsed_body) = parsed.get_body() {
                    debug!("Stored the body of the mail.");
                    body = parsed_body;
                }
            }

            // Here we're going through the multi-/subparts of the mail
            for subpart in &parsed.subparts {
                // now it might happen, that the body is *in* a multipart, if
                // that's the case, look, if we've already applied a body
                // (body.is_empty()) and set it, if needed
                if body.is_empty() && subpart.ctype.mimetype == "text/plain" {
                    if let Ok(subpart_body) = subpart.get_body() {
                        body = subpart_body;
                    }
                }
                // otherise it's a normal attachment, like a PNG file or
                // something like that
                else if let Some(attachment) = Attachment::from_parsed_mail(subpart) {
                    attachments.push(attachment);
                }
            }
        }

        Ok(Self {
            attachments,
            flags,
            envelope,
            body: Body::from(body),
            uid,
            date,
            raw,
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
// From's
// -----------
impl<'mails> TryFrom<&'mails ZeroCopy<Vec<Fetch>>> for Msgs {
    type Error = Error;

    fn try_from(fetches: &'mails ZeroCopy<Vec<Fetch>>) -> Result<Self> {
        // the content of the Msgs-struct
        let mut mails = Vec::new();

        for fetch in fetches.iter().rev() {
            mails.push(Msg::try_from(fetch)?);
        }

        Ok(Self(mails))
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

#[cfg(test)]
mod tests {
    use crate::config::model::Account;
    use crate::msg::body::Body;
    use crate::msg::envelope::Envelope;
    use crate::msg::model::Msg;

    #[test]
    fn test_new() {
        // -------------
        // Accounts
        // -------------
        let account1 = Account::new(Some("Soywod"), "clement.douin@posteo.net");
        let account2 = Account::new(None, "tornax07@gmail.com");

        // ---------------------
        // Creating message
        // ---------------------
        let msg1 = Msg::new(&account1);
        let msg2 = Msg::new(&account2);

        // ---------------------
        // Expected outputs
        // ---------------------
        let expected_envelope1 = Envelope {
            from: vec![String::from("Soywod <clement.douin@posteo.net>")],
            signature: Some(String::from("Account Signature")),
            ..Envelope::default()
        };

        let expected_envelope2 = Envelope {
            from: vec![String::from("tornax07@gmail.com")],
            signature: Some(String::from("Account Signature")),
            ..Envelope::default()
        };

        // ----------
        // Tests
        // ----------
        assert_eq!(msg1.envelope, expected_envelope1);
        assert_eq!(msg2.envelope, expected_envelope2);

        assert!(msg1.get_raw().unwrap().is_empty());
        assert!(msg2.get_raw().unwrap().is_empty());
    }

    #[test]
    fn test_new_with_envelope() {
        let account = Account::new(Some("Name"), "test@mail.asdf");

        // ------------------
        // Test-Messages
        // ------------------
        let msg_with_custom_from = Msg::new_with_envelope(
            &account,
            Envelope {
                from: vec![String::from("Someone <Else@mail.asdf>")],
                ..Envelope::default()
            },
        );

        let msg_with_custom_signature = Msg::new_with_envelope(
            &account,
            Envelope {
                signature: Some(String::from("Awesome Signature!")),
                ..Envelope::default()
            },
        );

        // -----------------
        // Expectations
        // -----------------
        let expected_with_custom_from = Msg {
            envelope: Envelope {
                // the Msg::new_with_envelope function should use the from
                // address in the envelope struct, not the from address of the
                // account
                from: vec![String::from("Someone <Else@mail.asdf>")],
                signature: Some(String::from("Account Signature")),
                ..Envelope::default()
            },
            // The signature should be added automatically
            body: Body::from("Account Signature"),
            ..Msg::default()
        };

        let expected_with_custom_signature = Msg {
            envelope: Envelope {
                from: vec![String::from("Name <test@mail.asdf>")],
                signature: Some(String::from("Awesome Signature!")),
                ..Envelope::default()
            },
            body: Body::from("Awesome Signature!"),
            ..Msg::default()
        };

        // ------------
        // Testing
        // ------------
        assert_eq!(msg_with_custom_from, expected_with_custom_from);
        assert_eq!(msg_with_custom_signature, expected_with_custom_signature);
    }

    #[test]
    fn test_change_to_reply() {
        // -----------------
        // Preparations
        // -----------------
        let account = Account::new(Some("Name"), "some@address.asdf");
        let mut msg_normal = Msg::new_with_envelope(
            &account,
            Envelope {
                from: vec!["Boss <someone@boss.asdf>".to_string()],
                to: vec![
                    "mail@1.asdf".to_string(),
                    "mail@2.asdf".to_string(),
                    "Name <some@address.asdf>".to_string(),
                ],
                cc: Some(vec![
                    "test@testing".to_string(),
                    "test2@testing".to_string(),
                ]),
                message_id: Some("RandomID123".to_string()),
                reply_to: Some(vec!["Reply@Mail.rofl".to_string()]),
                subject: Some("Have you heard of himalaya?".to_string()),
                ..Envelope::default()
            },
        );

        msg_normal.body = Body::from(concat![
            "I can just recommend you to use himalaya!\n",
            "\n",
            "Sincereley",
        ]);

        // -- missing reply to --
        let mut msg_missing_reply_to = msg_normal.clone();
        msg_missing_reply_to.envelope = Envelope {
            reply_to: None,
            ..msg_missing_reply_to.envelope.clone()
        };

        // --------------------
        // Expected output
        // --------------------
        let expected_not_reply_all = Msg {
            envelope: Envelope {
                from: vec!["Name <some@address.asdf>".to_string()],
                to: vec!["Reply@Mail.rofl".to_string()],
                cc: None,
                in_reply_to: Some("RandomID123".to_string()),
                subject: Some("Re: Have you heard of himalaya?".to_string()),
                ..Envelope::default()
            },
            body: Body::from(concat![
                "> I can just recommend you to use himalaya!\n",
                "> \n",
                "> Sincereley\n",
            ]),
            ..Msg::default()
        };

        let expected_reply_all = Msg {
            envelope: Envelope {
                from: vec!["Name <some@address.asdf>".to_string()],
                to: vec![
                    "mail@1.asdf".to_string(),
                    "mail@2.asdf".to_string(),
                    "Reply@Mail.rofl".to_string(),
                ],
                cc: Some(vec![
                    "test@testing".to_string(),
                    "test2@testing".to_string(),
                ]),
                in_reply_to: Some("RandomID123".to_string()),
                subject: Some("Re: Have you heard of himalaya?".to_string()),
                ..Envelope::default()
            },
            body: Body::from(concat![
                "> I can just recommend you to use himalaya!\n",
                "> \n",
                "> Sincereley\n",
            ]),
            ..Msg::default()
        };

        let expected_missing_reply_to = Msg {
            envelope: Envelope {
                from: vec!["Name <some@address.asdf>".to_string()],
                to: vec!["Boss <someone@boss.asdf>".to_string()],
                cc: None,
                in_reply_to: Some("RandomID123".to_string()),
                subject: Some("Re: Have you heard of himalaya?".to_string()),
                ..Envelope::default()
            },
            body: Body::from(concat![
                "> I can just recommend you to use himalaya!\n",
                "> \n",
                "> Sincereley\n",
            ]),
            ..Msg::default()
        };

        let expected_missing_reply_to_reply_all = Msg {
            envelope: Envelope {
                from: vec!["Name <some@address.asdf>".to_string()],
                to: vec![
                    "mail@1.asdf".to_string(),
                    "mail@2.asdf".to_string(),
                    "Boss <someone@boss.asdf>".to_string(),
                ],
                cc: Some(vec![
                    "test@testing".to_string(),
                    "test2@testing".to_string(),
                ]),
                in_reply_to: Some("RandomID123".to_string()),
                subject: Some("Re: Have you heard of himalaya?".to_string()),
                ..Envelope::default()
            },
            body: Body::from(concat![
                "> I can just recommend you to use himalaya!\n",
                "> \n",
                "> Sincereley\n",
            ]),
            ..Msg::default()
        };

        // ------------
        // Testing
        // ------------
        let mut msg1 = msg_normal.clone();
        let mut msg2 = msg_normal.clone();
        let mut msg_missing_reply_to1 = msg_missing_reply_to.clone();
        let mut msg_missing_reply_to2 = msg_missing_reply_to.clone();

        msg1.change_to_reply(&account, false).unwrap();
        msg2.change_to_reply(&account, true).unwrap();
        msg_missing_reply_to1
            .change_to_reply(&account, false)
            .unwrap();
        msg_missing_reply_to2
            .change_to_reply(&account, true)
            .unwrap();

        assert_eq!(msg1, expected_not_reply_all);
        assert_eq!(msg2, expected_reply_all);

        assert_eq!(msg_missing_reply_to1, expected_missing_reply_to);
        assert_eq!(msg_missing_reply_to2, expected_missing_reply_to_reply_all);
    }

    #[test]
    fn test_change_to_forwarding() {
        // -----------------
        // Preparations
        // -----------------
        let account = Account::new(Some("Name"), "some@address.asdf");
        let mut msg = Msg::new_with_envelope(
            &account,
            Envelope {
                from: vec![String::from("ThirdPerson <some@mail.asdf>")],
                subject: Some(String::from("Test subject")),
                ..Envelope::default()
            },
        );

        msg.body = Body::from(concat![
            "The body text, nice!\n",
            "Himalaya is nice!",
        ]);

        // ---------------------
        // Expected Results
        // ---------------------
        let expected_msg = Msg {
            envelope: Envelope {
                from: vec![String::from("ThirdPerson <some@mail.asdf>")],
                sender: Some(String::from("Name <some@address.asdf>")),
                signature: Some(String::from("Account Signature")),
                subject: Some(String::from("Fwd: Test subject")),
                .. Envelope::default()
            },
            body: Body::from(concat![
                "\r\n---------- Forwarded Message ----------\r\n",
                "The body text, nice!\n",
                "Himalaya is nice!\n",
            ]),
            .. Msg::default()
        };

        // ----------
        // Tests
        // ----------
        msg.change_to_forwarding(&account);
        assert_eq!(msg, expected_msg);
    }

    #[test]
    fn test_edit_body() {
        // -----------------
        // Preparations
        // -----------------
        let account = Account::new(Some("Name"), "some@address.asdf");
        let mut msg = Msg::new_with_envelope(
            &account,
            Envelope {
                bcc: Some(Vec::new()),
                cc: Some(Vec::new()),
                subject: Some(String::new()),
                .. Envelope::default()
            },
        );

        // ---------------------
        // Expected Results
        // ---------------------
        let expected_msg = Msg {
            envelope: Envelope {
                from: vec![String::from("Name <some@address.asdf>")],
                to: vec![String::from("")],
                // these fields should exist now
                subject: Some(String::from("")),
                bcc: Some(vec![String::from("")]),
                cc: Some(vec![String::from("")]),
                .. Envelope::default()
            },
            body: Body::from("Account Signature\n"),
            .. Msg::default()
        };

        // ----------
        // Tests
        // ----------
        msg.edit_body().unwrap();
        assert_eq!(msg, expected_msg);
    }

    #[test]
    fn test_parse_from_str() {
        // -----------------
        // Preparations
        // -----------------
        let account = Account::new(Some("Name"), "some@address.asdf");
        let msg = Msg::new(&account);

        let new_content = concat![
            "From: Some <user@mail.sf>\n",
            "Suject: Awesome Subject\n",
            "Bcc: mail1@rofl.lol, name <rofl@lol.asdf>\n",
        ];
    }
}
