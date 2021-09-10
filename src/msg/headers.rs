use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;

use log::{debug, warn};

use serde::Serialize;

use rfc2047_decoder;

use error_chain::error_chain;

use lettre::message::header::ContentTransferEncoding;

error_chain! {
    errors {
        Convertion(field: &'static str) {
            display("Couldn't get the data from the '{}:' field.", field),
        }
    }

    foreign_links {
        StringFromUtf8(std::string::FromUtf8Error);
        Rfc2047Decoder(rfc2047_decoder::Error);
    }
}

// == Structs ==
/// This struct is a wrapper for the [Envelope struct] of the [imap_proto]
/// crate. It's should mainly help to interact with the mails by using more
/// common data types like `Vec` or `String` since a `[u8]` array is a little
/// bit limited to use.
///
/// # Usage
/// The general idea is, that you create a new instance like that:
///
/// ```
/// use himalaya::msg::headers::Headers;
/// # fn main() {
///
/// let headers = Headers {
///     from: vec![String::from("From <address@example.com>")],
///     to: vec![String::from("To <address@to.com>")],
///     ..Headers::default()
/// };
///
/// # }
/// ```
///
/// We don't have a build-pattern here, because this is easy as well and we
/// don't need a dozens of functions, just to set some values.
///
/// [Envelope struct]: https://docs.rs/imap-proto/0.14.3/imap_proto/types/struct.Headers.html
/// [imap_proto]: https://docs.rs/imap-proto/0.14.3/imap_proto/index.html
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Headers {
    // -- Must-Fields --
    // These fields are the mininum needed to send a msg.
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub encoding: ContentTransferEncoding,

    // -- Optional fields --
    pub bcc: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub custom_headers: Option<HashMap<String, Vec<String>>>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,
    pub reply_to: Option<Vec<String>>,
    pub sender: Option<String>,
    pub signature: Option<String>,
    pub subject: Option<String>,
}

impl Headers {
    /// This method works similiar to the [`Display Trait`] but it will only
    /// convert the header into a string **without** the signature.
    ///
    /// # Example
    ///
    /// <details>
    ///
    /// ```
    /// # use himalaya::msg::headers::Headers;
    /// # use std::collections::HashMap;
    /// # use lettre::message::header::ContentTransferEncoding;
    /// # fn main() {
    /// // our headers
    /// let headers = Headers {
    ///     from:           vec!["TornaxO7 <tornax07@gmail.com>".to_string()],
    ///     to:             vec!["Soywod <clement.douin@posteo.net>".to_string()],
    ///     encoding:       ContentTransferEncoding::Base64,
    ///     bcc:            Some(vec!["ThirdOne <some@msg.net>".to_string()]),
    ///     cc:             Some(vec!["CcAccount <cc@ccmail.net>".to_string()]),
    ///     custom_headers: None,
    ///     in_reply_to:    Some("1234@local.machine.example".to_string()),
    ///     message_id:     Some("123456789".to_string()),
    ///     reply_to:       Some(vec!["reply@msg.net".to_string()]),
    ///     sender:         Some("himalaya@secretary.net".to_string()),
    ///     signature:      Some("Signature of Headers".to_string()),
    ///     subject:        Some("Himalaya is cool".to_string()),
    /// };
    ///
    /// // get the header
    /// let headers_string = headers.get_header_as_string();
    ///
    /// // how the header part should look like
    /// let expected_output = concat![
    ///     "From: TornaxO7 <tornax07@gmail.com>\n",
    ///     "To: Soywod <clement.douin@posteo.net>\n",
    ///     "In-Reply-To: 1234@local.machine.example\n",
    ///     "Sender: himalaya@secretary.net\n",
    ///     "Message-ID: 123456789\n",
    ///     "Reply-To: reply@msg.net\n",
    ///     "Cc: CcAccount <cc@ccmail.net>\n",
    ///     "Bcc: ThirdOne <some@msg.net>\n",
    ///     "Subject: Himalaya is cool\n",
    /// ];
    ///
    /// assert_eq!(headers_string, expected_output,
    ///     "{}, {}",
    ///     headers_string, expected_output);
    /// # }
    /// ```
    ///
    /// </details>
    ///
    /// [`Display Trait`]: https://doc.rust-lang.org/std/fmt/trait.Display.html
    pub fn get_header_as_string(&self) -> String {
        let mut header = String::new();

        // -- Must-Have-Fields --
        // the "From: " header
        header.push_str(&merge_addresses_to_one_line("From", &self.from, ','));

        // the "To: " header
        header.push_str(&merge_addresses_to_one_line("To", &self.to, ','));

        // -- Optional fields --
        // Here we are adding only the header parts which have a value (are not
        // None). That's why we are always checking here with "if let Some()".

        // in reply to
        if let Some(in_reply_to) = &self.in_reply_to {
            header.push_str(&format!("In-Reply-To: {}\n", in_reply_to));
        }

        // Sender
        if let Some(sender) = &self.sender {
            header.push_str(&format!("Sender: {}\n", sender));
        }

        // Message-ID
        if let Some(message_id) = &self.message_id {
            header.push_str(&format!("Message-ID: {}\n", message_id));
        }

        // reply_to
        if let Some(reply_to) = &self.reply_to {
            header.push_str(&merge_addresses_to_one_line("Reply-To", &reply_to, ','));
        }

        // cc
        if let Some(cc) = &self.cc {
            header.push_str(&merge_addresses_to_one_line("Cc", &cc, ','));
        }

        // bcc
        if let Some(bcc) = &self.bcc {
            header.push_str(&merge_addresses_to_one_line("Bcc", &bcc, ','));
        }

        // custom headers
        if let Some(custom_headers) = &self.custom_headers {
            for (key, value) in custom_headers.iter() {
                header.push_str(&merge_addresses_to_one_line(key, &value, ','));
            }
        }

        // Subject
        if let Some(subject) = &self.subject {
            header.push_str(&format!("Subject: {}\n", subject));
        }

        header
    }
}

/// Returns a Headers with the following values:
///
/// ```no_run
/// # use himalaya::msg::headers::Headers;
/// # use lettre::message::header::ContentTransferEncoding;
/// Headers {
///     from:           Vec::new(),
///     to:             Vec::new(),
///     encoding:       ContentTransferEncoding::Base64,
///     bcc:            None,
///     cc:             None,
///     custom_headers: None,
///     in_reply_to:    None,
///     message_id:     None,
///     reply_to:       None,
///     sender:         None,
///     signature:      None,
///     subject:        None,
/// };
/// ```
impl Default for Headers {
    fn default() -> Self {
        Self {
            // must-fields
            from: Vec::new(),
            to: Vec::new(),
            encoding: ContentTransferEncoding::Base64,

            // optional fields
            bcc: None,
            cc: None,
            custom_headers: None,
            in_reply_to: None,
            message_id: None,
            reply_to: None,
            sender: None,
            signature: None,
            subject: None,
        }
    }
}

// == From implementations ==
impl TryFrom<Option<&imap_proto::types::Envelope<'_>>> for Headers {
    type Error = Error;

    fn try_from(envelope: Option<&imap_proto::types::Envelope<'_>>) -> Result<Self> {
        if let Some(envelope) = envelope {
            debug!("Fetch has headers.");

            let subject = envelope
                .subject
                .as_ref()
                .and_then(|subj| rfc2047_decoder::decode(subj).ok());

            let from = match convert_vec_address_to_string(envelope.from.as_ref())? {
                Some(from) => from,
                None => return Err(ErrorKind::Convertion("From").into()),
            };

            // only the first address is used, because how should multiple machines send the same
            // mail?
            let sender = convert_vec_address_to_string(envelope.sender.as_ref())?;
            let sender = match sender {
                Some(tmp_sender) => Some(
                    tmp_sender
                        .iter()
                        .next()
                        .unwrap_or(&String::new())
                        .to_string(),
                ),
                None => None,
            };

            let message_id = convert_cow_u8_to_string(envelope.message_id.as_ref())?;
            let reply_to = convert_vec_address_to_string(envelope.reply_to.as_ref())?;
            let to = match convert_vec_address_to_string(envelope.to.as_ref())? {
                Some(to) => to,
                None => return Err(ErrorKind::Convertion("To").into()),
            };
            let cc = convert_vec_address_to_string(envelope.cc.as_ref())?;
            let bcc = convert_vec_address_to_string(envelope.bcc.as_ref())?;
            let in_reply_to = convert_cow_u8_to_string(envelope.in_reply_to.as_ref())?;

            Ok(Self {
                subject,
                from,
                sender,
                message_id,
                reply_to,
                to,
                cc,
                bcc,
                in_reply_to,
                custom_headers: None,
                signature: None,
                encoding: ContentTransferEncoding::Base64,
            })
        } else {
            debug!("Fetch hasn't headers.");
            Ok(Headers::default())
        }
    }
}

impl<'from> From<&mailparse::ParsedMail<'from>> for Headers {
    fn from(parsed_mail: &mailparse::ParsedMail<'from>) -> Self {
        let mut new_headers = Headers::default();

        let header_iter = parsed_mail.headers.iter();
        for header in header_iter {
            // get the value of the header. For example if we have this header:
            //
            //  Subject: I use Arch btw
            //
            // than `value` would be like that: `let value = "I use Arch btw".to_string()`
            let value = header.get_value().replace("\r", "");
            let header_name = header.get_key().to_lowercase();
            let header_name = header_name.as_str();

            // now go through all headers and look which values they have.
            match header_name {
                "from" => {
                    new_headers.from = value
                        .rsplit(',')
                        .map(|addr| addr.trim().to_string())
                        .collect()
                }

                "to" => {
                    new_headers.to = value
                        .rsplit(',')
                        .map(|addr| addr.trim().to_string())
                        .collect()
                }

                "bcc" => {
                    new_headers.bcc = Some(
                        value
                            .rsplit(',')
                            .map(|addr| addr.trim().to_string())
                            .collect(),
                    )
                }

                "cc" => {
                    new_headers.cc = Some(
                        value
                            .rsplit(',')
                            .map(|addr| addr.trim().to_string())
                            .collect(),
                    )
                }
                "in_reply_to" => new_headers.in_reply_to = Some(value),
                "reply_to" => {
                    new_headers.reply_to = Some(
                        value
                            .rsplit(',')
                            .map(|addr| addr.trim().to_string())
                            .collect(),
                    )
                }

                "sender" => new_headers.sender = Some(value),
                "subject" => new_headers.subject = Some(value),
                "message-id" => new_headers.message_id = Some(value),
                "content-transfer-encoding" => {
                    match value.to_lowercase().as_str() {
                        "8bit" => new_headers.encoding = ContentTransferEncoding::EightBit,
                        "7bit" => new_headers.encoding = ContentTransferEncoding::SevenBit,
                        "quoted-printable" => {
                            new_headers.encoding = ContentTransferEncoding::QuotedPrintable
                        }
                        "base64" => new_headers.encoding = ContentTransferEncoding::Base64,
                        _ => warn!("Unsupported encoding, default to QuotedPrintable"),
                    };
                }

                // it's a custom header => Add it to our
                // custom-header-hash-map
                _ => {
                    let custom_header = header.get_key();

                    // If we don't have a HashMap yet => Create one! Otherwise
                    // we'll keep using it, because why should we reset its
                    // values again?
                    if let None = new_headers.custom_headers {
                        new_headers.custom_headers = Some(HashMap::new());
                    }

                    let mut updated_hashmap = new_headers.custom_headers.unwrap();

                    updated_hashmap.insert(
                        custom_header,
                        value
                            .rsplit(',')
                            .map(|addr| addr.trim().to_string())
                            .collect(),
                    );

                    new_headers.custom_headers = Some(updated_hashmap);
                }
            }
        }

        new_headers
    }
}

// -- Common Traits --
/// This trait just returns the headers but as a string. But be careful! **The
/// signature is printed as well!!!**, so it isn't really useable to create the
/// content of a msg! Use [get_header_as_string] instead!
///
/// # Example
///
/// ```
/// # use himalaya::msg::headers::Headers;
/// # fn main() {
/// let headers = Headers {
///     subject: Some(String::from("Himalaya is cool")),
///     to: vec![String::from("Soywod <clement.douin@posteo.net>")],
///     from: vec![String::from("TornaxO7 <tornax07@gmail.com>")],
///     signature: Some(String::from("Signature of Headers")),
///     ..Headers::default()
/// };
///
/// // use the `fmt::Display` trait
/// let headers_output = format!("{}", headers);
///
/// // How the output of the `fmt::Display` trait should look like
/// let expected_output = concat![
///     "From: TornaxO7 <tornax07@gmail.com>\n",
///     "To: Soywod <clement.douin@posteo.net>\n",
///     "Subject: Himalaya is cool\n",
///     "\n\n\n",
///     "Signature of Headers",
/// ];
///
/// assert_eq!(headers_output, expected_output,
///     "{:#?}, {:#?}",
///     headers_output, expected_output);
/// # }
/// ```
///
/// [get_header_as_string]: struct.Headers.html#method.get_header_as_string
impl fmt::Display for Headers {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut header = self.get_header_as_string();

        // now add some space between the header and the signature
        header.push_str("\n\n\n");

        // and add the signature in the end
        header.push_str(&self.signature.clone().unwrap_or(String::new()));

        write!(formatter, "{}", header)
    }
}

// -- Helper functions --
/// This function is mainly used for the `imap_proto::types::Address` struct to
/// convert one field into a String. Take a look into the
/// `test_convert_cow_u8_to_string` test function to see it in action.
fn convert_cow_u8_to_string<'val>(value: Option<&Cow<'val, [u8]>>) -> Result<Option<String>> {
    if let Some(value) = value {
        // convert the `[u8]` list into a vector and try to get a string out of
        // it. If everything worked fine, return the content of the list
        Ok(Some(rfc2047_decoder::decode(&value.to_vec())?))
    } else {
        Ok(None)
    }
}

/// This function is mainly used for the `imap_proto::types::Address` struct as
/// well to change the Address into an address-string like this:
/// `TornaxO7 <tornax07@gmail.com>`.
///
/// If you provide two addresses as the function argument, then this functions
/// returns their "parsed" address in the same order. Take a look into the
/// `test_convert_vec_address_to_string` for an example.
fn convert_vec_address_to_string<'val>(
    addresses: Option<&Vec<imap_proto::types::Address<'val>>>,
) -> Result<Option<Vec<String>>> {
    if let Some(addresses) = addresses {
        let mut parsed_addresses: Vec<String> = Vec::new();

        for address in addresses.iter() {
            // This variable will hold the parsed version of the Address-struct,
            // like this:
            //
            //  "Name <msg@host>"
            let mut parsed_address = String::new();

            // -- Get the fields --
            // add the name field (if it exists) like this:
            // "Name"
            if let Some(name) = convert_cow_u8_to_string(address.name.as_ref())? {
                parsed_address.push_str(&name);
            }

            // add the mailaddress
            if let Some(mailbox) = convert_cow_u8_to_string(address.mailbox.as_ref())? {
                if let Some(host) = convert_cow_u8_to_string(address.host.as_ref())? {
                    let mail_address = format!("{}@{}", mailbox, host);

                    // some mail clients add a trailing space, after the address
                    let trimmed = mail_address.trim();

                    if parsed_address.is_empty() {
                        //  parsed_address = "msg@host"
                        parsed_address.push_str(&trimmed);
                    } else {
                        //  parsed_address = "Name <msg@host>"
                        parsed_address.push_str(&format!(" <{}>", trimmed));
                    }
                }
            }

            parsed_addresses.push(parsed_address);
        }

        Ok(Some(parsed_addresses))
    } else {
        Ok(None)
    }
}

/// This function is used, in order to merge multiple msg accounts into one
/// line. Take a look into the `test_merge_addresses_to_one_line` test-function
/// to see an example how to use it.
fn merge_addresses_to_one_line(header: &str, addresses: &Vec<String>, separator: char) -> String {
    let mut output = header.to_string();
    let mut address_iter = addresses.iter();

    // Convert the header to this (for example): `Cc: `
    output.push_str(": ");

    // the first emsg doesn't need a comma before, so we should append the msg
    // to it
    output.push_str(address_iter.next().unwrap_or(&String::new()));

    // add the rest of the emails. It should look like this after the for_each:
    //
    //  Addr1, Addr2, Addr2, ...
    address_iter.for_each(|address| output.push_str(&format!("{}{}", separator, address)));

    // end the header-line by using a newline character
    output.push('\n');

    output
}

// ==========
// Tests
// ==========
/// This tests only test the helper functions.
#[cfg(test)]
mod tests {

    #[test]
    fn test_merge_addresses_to_one_line() {
        use super::merge_addresses_to_one_line;
        // In this function, we want to create the following Cc header:
        //
        //  Cc: TornaxO7 <tornax07@gmail.com>, Soywod <clement.douin@posteo.net>
        //
        // by a vector of email-addresses.

        // our msg addresses for the "Cc" header
        let mail_addresses = vec![
            "TornaxO7 <tornax07@gmail.com>".to_string(),
            "Soywod <clement.douin@posteo.net>".to_string(),
        ];

        let cc_header = merge_addresses_to_one_line("Cc", &mail_addresses, ',');

        let expected_output = concat![
            "Cc: TornaxO7 <tornax07@gmail.com>",
            ",",
            "Soywod <clement.douin@posteo.net>\n",
        ];

        assert_eq!(
            cc_header, expected_output,
            "{:#?}, {:#?}",
            cc_header, expected_output
        );
    }

    #[test]
    fn test_convert_cow_u8_to_string() {
        use super::convert_cow_u8_to_string;
        use std::borrow::Cow;

        let output1 = convert_cow_u8_to_string(None);
        let output2 = convert_cow_u8_to_string(Some(&Cow::Owned(b"Test".to_vec())));

        // test output1
        if let Ok(output1) = output1 {
            assert!(output1.is_none());
        } else {
            assert!(false);
        }

        // test output2
        if let Ok(output2) = output2 {
            if let Some(string) = output2 {
                assert_eq!(String::from("Test"), string);
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_convert_vec_address_to_string() {
        use super::convert_vec_address_to_string;
        use imap_proto::types::Address;
        use std::borrow::Cow;

        let addresses = vec![
            Address {
                name: Some(Cow::Owned(b"Name1".to_vec())),
                adl: None,
                mailbox: Some(Cow::Owned(b"Mailbox1".to_vec())),
                host: Some(Cow::Owned(b"Host1".to_vec())),
            },
            Address {
                name: None,
                adl: None,
                mailbox: Some(Cow::Owned(b"Mailbox2".to_vec())),
                host: Some(Cow::Owned(b"Host2".to_vec())),
            },
        ];

        // the expected addresses
        let expected_output = vec![
            String::from("Name1 <Mailbox1@Host1>"),
            String::from("Mailbox2@Host2"),
        ];

        if let Ok(converted) = convert_vec_address_to_string(Some(&addresses)) {
            assert_eq!(converted, Some(expected_output));
        } else {
            assert!(false);
        }
    }
}
