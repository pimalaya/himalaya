use super::attachment::Attachment;
use super::envelope::Envelope;

use imap::types::Flag;

use mailparse::{ParsedMail, DispositionType};

// ===============
// Error enum
// ===============
/// This enums are used for errors which might happen when trying to parse the
/// mail from the given fetch.
pub enum MailError {

    /// An error appeared, when it tried to parse the body of the mail!
    ParseBody,

    /// An error appeared, when it tried to parse/get an attachment!
    ParseAttachment,
}

// ============
// Structs
// ============
/// The struct which will hold all information of a mail.
///
/// # NOTE
/// This struct is just a prototype and not used currently!
///
/// # Example (how it can be get and used)
/// ## Get
/// ```rust
/// // suppose we're implementing a new function for "ImapConnector"
/// // which should fetch the mail and store it in this struct which will be
/// // returned by this function
/// pub fn read_mail(&mut self, mbox: &str, uid: &str) -> Result<Mail> {
///     self.sess
///         .select(mbox)
///         .chain_err(|| format!("Could not select mailbox '{}'", mbox))?;
///
///     match self.sess
///         .uid_fetch(uid, "(FLAGS ENVELOPE BODY[])")
///         .chain_err(|| "Could not fetch the mail")?
///         .first()
///     {
///         None => Err(format!("Could not find message '{}'", uid).into()),
///         Some(fetch) => Ok(Mail::new(&fetch)?),
///     }
/// }
/// ```
///
/// ## Use
/// ```rust
/// // random getter
/// let mail = Mail::new(&fetch)?;
/// 
/// // Want to know the sender?
/// println!("{}", mail.envelope.sender);
///
/// // Want to know all attachments?
/// for attachment_name in &mail.attachments {
///     println!("{}", attachment_name);
/// }
///
/// // Want to get the body of the mail?
/// println!("{}", mail.parsed.get_body().unwrap_or_else(String::new()));
pub struct Mail<'mail> {

    /// All added attachments are listed in this vector.
    pub attachments: Vec<Attachment>,

    /// The flags for this mail.
    pub flags: Vec<Flag<'mail>>,

    /// All information of the envelope (sender, from, to and so on)
    pub envelope: Envelope,

    /// The parsed content of the mail which shoud make it easier to access
    /// some parts, like the headers and to know which mime-types they have.
    pub parsed: ParsedMail<'mail>,
}

impl<'mail> Mail<'mail> {

    // Create a new mail-representant which should make it easier to work with
    // the given data of the mail.
    pub fn new(fetch: &'mail imap::types::Fetch) -> Result<Self, MailError> {

        // Here will be all attachments stored
        let mut attachments = Vec::new();

        // Get the flags of the mail
        let flags = fetch.flags().to_vec();

        // Well, get the data of the envelope from the mail
        let envelope = Envelope::from(fetch.envelope());

        // Get the parsed-version of the mail
        let parsed = match mailparse::parse_mail(fetch.body().unwrap_or(&[b' '])) {
            Ok(parsed_mail) => parsed_mail,
            Err(_) => return Err(MailError::ParseBody),
        };

        // Go through all subparts of the mail and look if they are attachments.
        // If they are attachments:
        //  1. Get their filename
        //  2. Get the content of the attachment
        for subpart in &parsed.subparts {
            if subpart.get_content_disposition().disposition == DispositionType::Attachment {

                let disposition = subpart.get_content_disposition();

                let filename = match disposition.params.get("filename") {
                    Some(name) => name,
                    None => return Err(MailError::ParseAttachment),
                };

                let body_raw = subpart.get_body_raw().unwrap_or(Vec::new());
                attachments.push(Attachment::new(filename.to_string(), body_raw));
            }
        }

        Ok(Self {
            attachments,
            flags,
            envelope,
            parsed,
        })
    }
}
