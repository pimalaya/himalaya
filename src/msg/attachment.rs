use lettre::message::header::ContentType;

use mailparse::{DispositionType, ParsedMail};

use std::convert::TryFrom;
use std::fs;
use std::path::Path;

use serde::Serialize;

use error_chain::error_chain;

error_chain! {
    foreign_links {
        ContentType(lettre::message::header::ContentTypeErr);
        FileSytem(std::io::Error);
    }
}

// == Structs ==
/// This struct represents an attachment.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// Holds the filename of an attachment.
    pub filename: String,

    /// Holds the mime-type of the attachment. For example `text/plain`.
    pub content_type: ContentType,

    /// Holds the data of the attachment.
    pub body_raw: Vec<u8>,
}

impl Attachment {
    /// Creates a new attachment.
    ///
    /// # Example
    /// ```
    /// # use himalaya::msg::attachment::Attachment;
    /// let attachment = Attachment::new(
    ///     "VIP Text",
    ///     "text/plain",
    ///     "Some very important text".as_bytes().to_vec());
    ///
    /// ```
    pub fn new(filename: &str, content_type: &str, body_raw: Vec<u8>) -> Self {
        // Use the mime type `text/plain` per default
        let content_type: ContentType = match content_type.parse() {
            Ok(lettre_type) => lettre_type,
            Err(_) => ContentType::TEXT_PLAIN,
        };

        Self {
            filename: filename.to_string(),
            content_type,
            body_raw,
        }
    }

    /// This from function extracts one attachment of a parsed msg.
    /// If it couldn't create an attachment with the given parsed msg, than it will
    /// return `None`.
    ///
    /// # Example
    /// ```
    /// use himalaya::msg::attachment::Attachment;
    ///
    /// let parsed = mailparse::parse_mail(concat![
    ///     "Content-Type: text/plain; charset=utf-8\n",
    ///     "Content-Transfer-Encoding: quoted-printable\n",
    ///     "\n",
    ///     "A plaintext attachment.",
    /// ].as_bytes()).unwrap();
    ///
    /// let attachment = Attachment::from_parsed_mail(&parsed);
    /// ```
    pub fn from_parsed_mail(parsed_mail: &ParsedMail) -> Option<Self> {
        if parsed_mail.get_content_disposition().disposition == DispositionType::Attachment {
            let disposition = parsed_mail.get_content_disposition();
            let filename = disposition.params.get("filename").unwrap().to_string();
            let body_raw = parsed_mail.get_body_raw().unwrap_or(Vec::new());
            let content_type: ContentType = tree_magic::from_u8(&body_raw).parse().unwrap();

            return Some(Self {
                filename,
                content_type,
                body_raw,
            });
        }

        None
    }
}

// == Traits ==
/// Creates an Attachment with the follwing values:
///
/// ```no_run
/// # use himalaya::msg::attachment::Attachment;
/// use lettre::message::header::ContentType;
///
/// let attachment = Attachment {
///     filename:     String::new(),
///     content_type: ContentType::TEXT_PLAIN,
///     body_raw:     Vec::new(),
/// };
/// ```
impl Default for Attachment {
    fn default() -> Self {
        Self {
            filename: String::new(),
            content_type: ContentType::TEXT_PLAIN,
            body_raw: Vec::new(),
        }
    }
}

// -- From Implementations --
/// Tries to convert the given file (by the given path) into an attachment.
/// It'll try to detect the mime-type/data-type automatically.
///
/// # Example
/// ```no_run
/// use himalaya::msg::attachment::Attachment;
/// use std::convert::TryFrom;
///
/// let attachment = Attachment::try_from("/some/path.png");
/// ```
impl<'from> TryFrom<&'from str> for Attachment {
    type Error = Error;

    fn try_from(path: &'from str) -> Result<Self> {
        let path = Path::new(path);

        // -- Get attachment information --
        let filename = if let Some(filename) = path.file_name() {
            filename
                // `&OsStr` -> `Option<&str>`
                .to_str()
                // get rid of the `Option` wrapper
                .unwrap_or(&String::new())
                .to_string()
        } else {
            // use an empty string
            String::new()
        };

        let file_content = fs::read(&path)?;
        let content_type: ContentType = tree_magic::from_filepath(&path).parse()?;

        Ok(Self {
            filename,
            content_type,
            body_raw: file_content,
        })
    }
}
