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

// ============
// Structs
// ============
/// This struct stores the information from an attachment:
///     1. It's filename
///     2. It's content
#[derive(Debug, Serialize, Clone)]
pub struct Attachment {
    pub filename: String,
    pub content_type: ContentType,
    pub body_raw: Vec<u8>,
}

impl Attachment {
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

    /// This from function extracts one attachment of a parsed mail.
    /// If it can't create an attachment with the given parsed mail, than it will
    /// return `None`.
    pub fn from_parsed_mail(parsed_mail: &ParsedMail) -> Option<Self> {
        if parsed_mail.get_content_disposition().disposition == DispositionType::Attachment {
            let disposition = parsed_mail.get_content_disposition();

            // get the filename of the attachment
            let filename = disposition.params.get("filename").unwrap().to_string();

            // get the body of the attachment
            let body_raw = parsed_mail.get_body_raw().unwrap_or(Vec::new());
            // now we need to find out, which mime-type it has.
            let content_type: ContentType = tree_magic::from_u8(&body_raw).parse().unwrap();

            // now we have all needed information!
            return Some(Self {
                filename,
                content_type,
                body_raw,
            });
        }

        None
    }
}

// ===========
// Traits
// ===========
impl Default for Attachment {
    fn default() -> Self {
        Self {
            filename: String::new(),
            content_type: ContentType::TEXT_PLAIN,
            body_raw: Vec::new(),
        }
    }
}

// =========================
// From Implementations
// =========================
/// Reads the file from the given path and parses it to an attachment, which
/// will be returned.
impl<'from> TryFrom<&'from str> for Attachment {
    type Error = Error;

    fn try_from(path: &'from str) -> Result<Self> {
        // -----------------
        // Preparations
        // -----------------
        // Get the path first
        let path = Path::new(path);

        // -------------------------------
        // Get attachment information
        // -------------------------------
        // get the filename.
        let filename = if let Some(filename) = path.file_name() {
            filename
                // convert `&OsStr` to `Option<&str>`
                .to_str()
                // get rid of the `Option` wrapper
                .unwrap_or(&String::new())
                // and get the string
                .to_string()
        } else {
            // well just return an empty string than...
            String::new()
        };

        // Open and read the content of the file (if possible)
        let file_content = fs::read(&path)?;

        // Get the filetype
        let content_type: ContentType = tree_magic::from_filepath(&path).parse()?;

        // Now we have all needed information
        Ok(Self {
            filename,
            content_type,
            body_raw: file_content,
        })
    }
}
