use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use serde::Serialize;

use crate::config::model::Account;

use rfc2047_decoder;

use error_chain::error_chain;

error_chain! { }

// ============
// Structs
// ============
/// This struct is a wrapper for the [Envelope
/// struct](https://docs.rs/imap-proto/0.14.3/imap_proto/types/struct.Envelope.html)
/// of the [imap_proto](https://docs.rs/imap-proto/0.14.3/imap_proto/index.html)
/// crate. It's should mainly help to interact with the mails by using more
/// common data types like `Vec` or `String` since a `[u8]` array is a little
/// bit limited to use.
#[derive(Debug, Serialize, Clone)]
pub struct Envelope {
    // ----------------
    // Must-Fields
    // ---------------
    pub from: Vec<String>,
    pub to:   Vec<String>,

    // --------------------
    // Optional fields
    // --------------------
    pub bcc:            Option<Vec<String>>,
    pub cc:             Option<Vec<String>>,
    pub custom_headers: Option<HashMap<String, Vec<String>>>,
    pub in_reply_to:    Option<String>,
    pub message_id:     Option<String>,
    pub reply_to:       Option<Vec<String>>,
    pub sender:         Option<String>,
    pub signature:      Option<String>,
    pub subject:        Option<String>,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope::default()
    }

    /// This is a little helper-function like which uses the the name and email
    /// of the account to create a valid address for the header.
    ///
    /// # Example
    ///
    /// ## With name
    /// Suppose the name field in the account struct *has* a value:
    ///
    /// ```rust
    /// use himalaya::config::model::Account;
    /// use himalaya::msg::envelope::Envelope;
    ///
    /// fn main() {
    ///     let account = Account {
    ///         // we just need those two values
    ///         name: Some(String::from("Name")),
    ///         email: String::from("BestEmail@Ever.lol"),
    ///         ..Account::default()
    ///     };
    ///
    ///     // get the address of the account
    ///     let address = Envelope::convert_to_address(&account);
    ///
    ///     assert_eq!("Name <BestEmail@Ever.lol>".to_string(), address);
    /// }
    /// ```
    ///
    /// ## Without name
    /// Suppose the name field in the account-struct *hasn't* a value:
    ///
    /// ```rust
    /// use himalaya::config::model::Account;
    /// use himalaya::msg::envelope::Envelope;
    ///
    /// fn main() {
    ///     let account = Account {
    ///         // we just need those two values
    ///         name: None,
    ///         email: String::from("BestEmail@Ever.lol"),
    ///         ..Account::default()
    ///     };
    ///
    ///     // get the address of the account
    ///     let address = Envelope::convert_to_address(&account);
    ///
    ///     assert_eq!("<BestEmail@Ever.lol>".to_string(), address);
    /// }
    /// ```
    pub fn convert_to_address(account: &Account) -> String {
        if let Some(name) = &account.name {
            format!(
                "{} <{}>",
                name,
                account.email
            )
        } else {
            format!("<{}>", account.email)
        }
    }


    pub fn get_from(&self) -> Vec<String> {
        self.from.clone()
    }

    pub fn get_to(&self) -> Vec<String> {
        self.to.clone()
    }

    pub fn get_bcc(&self) -> Vec<String> {
        self.bcc.clone().unwrap_or(Vec::new())
    }

    pub fn get_cc(&self) -> Vec<String> {
        self.cc.clone().unwrap_or(Vec::new())
    }

    pub fn get_custom_headers(&self) -> HashMap<String, Vec<String>> {
        self.custom_headers.clone().unwrap_or(HashMap::new())
    }

    pub fn get_in_reply_to(&self) -> String {
        self.in_reply_to.clone().unwrap_or_default()
    }

    pub fn get_message_id(&self) -> String {
        self.message_id.clone().unwrap_or_default()
    }

    pub fn get_reply_to(&self) -> Vec<String> {
        self.reply_to.clone().unwrap_or(Vec::new())
    }

    pub fn get_sender(&self) -> String {
        self.sender.clone().unwrap_or_default()
    }

    pub fn get_signature(&self) -> String {
        self.sender.clone().unwrap_or_default()
    }

    pub fn get_subject(&self) -> String {
        self.subject.clone().unwrap_or_default()
    }

    pub fn get_header_as_string(&self) -> String {
        let mut header = String::new();

        // ---------------------
        // Must-Have-Fields
        // ---------------------
        // the "From: " header
        header.push_str(&merge_addresses_to_one_line("From", &self.from));

        // the "To: " header
        header.push_str(&merge_addresses_to_one_line("To", &self.to));

        // --------------------
        // Optional fields
        // --------------------
        // Here we are adding only the header parts which have a value (are not
        // None). That's why we are always checking here with "if let Some()".

        // Subject
        if let Some(subject) = &self.subject {
            header.push_str(&format!("Subject: {}\n", subject));
        }

        // in reply to
        if let Some(in_reply_to) = &self.in_reply_to {
            header.push_str(&format!("In-Reply-To: {}\n", in_reply_to));
        }

        // Sender
        if let Some(sender) = &self.sender {
            header.push_str(&format!("Sender: {}", sender));
        }

        // Message-ID
        if let Some(message_id) = &self.message_id {
            header.push_str(&format!("Message-ID: {}\n", message_id));
        }

        // reply_to
        if let Some(reply_to) = &self.reply_to {
            header
                .push_str(&merge_addresses_to_one_line("Reply-To", &reply_to));
        }

        // cc
        if let Some(cc) = &self.cc {
            header.push_str(&merge_addresses_to_one_line("Cc", &cc));
        }

        // bcc
        if let Some(bcc) = &self.bcc {
            header.push_str(&merge_addresses_to_one_line("Bcc", &bcc));
        }

        // custom headers
        if let Some(custom_headers) = &self.custom_headers {
            for (key, value) in custom_headers.iter() {
                header.push_str(&merge_addresses_to_one_line(key, &value));
            }
        }

        header
    }
}

// ===========================
// Default implementation
// ===========================
impl Default for Envelope {
    fn default() -> Self {
        Self {
            // must-fields
            from: Vec::new(),
            to:   Vec::new(),

            // optional fields
            bcc:            None,
            cc:             None,
            custom_headers: None,
            in_reply_to:    None,
            message_id:     None,
            reply_to:       None,
            sender:         None,
            signature:      None,
            subject:        None,
        }
    }
}

// =========================
// From implementations
// =========================
impl From<Option<&imap_proto::types::Envelope<'_>>> for Envelope {
    fn from(from_envelope: Option<&imap_proto::types::Envelope<'_>>) -> Self {
        if let Some(from_envelope) = from_envelope {
            let subject = from_envelope
                .subject
                .as_ref()
                .and_then(|subj| rfc2047_decoder::decode(subj).ok());

            let from =
                convert_vec_address_to_string(from_envelope.from.as_ref())
                    .unwrap_or(Vec::new());

            // since we get a vector here, we just need the first value, because
            // there should be only one sender, otherwise we'll pass an empty
            // string there
            let sender =
                convert_vec_address_to_string(from_envelope.sender.as_ref());
            // pick up the first element (if it exists) otherwise just set it
            // to None because we might don't need it
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

            let message_id =
                convert_cow_u8_to_string(from_envelope.message_id.as_ref());

            let reply_to =
                convert_vec_address_to_string(from_envelope.reply_to.as_ref());

            let to = convert_vec_address_to_string(from_envelope.to.as_ref())
                .unwrap_or(Vec::new());
            let cc = convert_vec_address_to_string(from_envelope.cc.as_ref());
            let bcc = convert_vec_address_to_string(from_envelope.bcc.as_ref());

            let in_reply_to =
                convert_cow_u8_to_string(from_envelope.in_reply_to.as_ref());

            Self {
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
            }
        } else {
            Envelope::default()
        }
    }
}

// ==================
// Common Traits
// ==================
/// This trait just returns a string-header. So for example if our envelope is
/// like this:
///
///     Envelope {
///         date: 11.11.1111,
///         subject: String::from("Himalaya is cool"),
///         ...
///     }
///
/// Then this will return:
///
///     Date: 11-11-1111
///     Subject: Himalaya is cool
///     ...
///
impl fmt::Display for Envelope {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut header = self.get_header_as_string();

        // now add some space between the header and the signature
        header.push_str("\n\n\n");

        // and add the signature in the end
        header.push_str(&self.signature.clone().unwrap_or(String::new()));

        write!(formatter, "{}", header)
    }
}

// ---------------------
// Helper functions
// ---------------------
fn convert_cow_u8_to_string<'val>(
    value: Option<&Cow<'val, [u8]>>,
) -> Option<String> {
    if let Some(value) = value {
        // convert the `[u8]` list into a vector and try to get a string out of
        // it.
        match String::from_utf8(value.to_vec()) {
            // if everything worked fine, return the content of the list
            Ok(content) => Some(content),
            Err(_) => None,
        }
    } else {
        None
    }
}

fn convert_vec_address_to_string<'val>(
    value: Option<&Vec<imap_proto::types::Address<'val>>>,
) -> Option<Vec<String>> {
    if let Some(value) = value {
        let value = value
            .iter()
            .map(|address| {
                // try to get the name of the mail-address
                let address_name =
                    convert_cow_u8_to_string(address.name.as_ref());

                match address_name {
                    Some(address_name) => address_name,
                    None => String::new(),
                }
            })
            .collect();

        Some(value)
    } else {
        None
    }
}

/// This function is used, in order to merge multiple mail accounts into one
/// line. For example if you have multiple mails for the `Cc: ` header part,
/// than you can do the following:
///
/// ```rust
/// // our mail addresses for the "Cc" header
/// let mail_addresses = vec![
///     "TornaxO7 <tornax07@gmail.com>",
///     "Soywod <clement.douin@posteo.net>",
/// ];
///
/// let cc_header = merge_addresses_to_one_line("Cc", &mail_addresses);
///
/// assert_eq!(
///     cc_header,
///     "Cc: TornaxO7 <tornax07@gmail.com>, Soywod
/// <clement.douin@posteo.net>"
///         .to_string()
/// );
/// ```
fn merge_addresses_to_one_line(
    header: &str,
    addresses: &Vec<String>,
) -> String {
    let mut output = header.to_string();
    let mut address_iter = addresses.iter();

    // Convert the header to this (for example): `Cc: `
    output.push_str(": ");

    // the first email doesn't need a comma before, so we should append the mail
    // to it
    output.push_str(address_iter.next().unwrap_or(&String::new()));

    // add the rest of the emails. It should look like this after the for_each:
    //
    //  Addr1, Addr2, Addr2, ...
    address_iter.for_each(|address| output.push_str(&format!(", {}", address)));

    // end the header-line by using a newline character
    output.push('\n');

    output
}
