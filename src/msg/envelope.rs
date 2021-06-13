use std::borrow::Cow;

// ============
// Structs
// ============
/// This struct is a wrapper for the [Envelope
/// struct](https://docs.rs/imap-proto/0.14.3/imap_proto/types/struct.Envelope.html)
/// of the [imap_proto](https://docs.rs/imap-proto/0.14.3/imap_proto/index.html)
/// crate. It's should mainly help to interact with the mails by using more
/// common data types like `Vec` or `String` since a `[u8]` array is a little
/// bit limited to use.
pub struct Envelope {
    pub date: String,
    pub subject: String,
    pub from: Vec<String>,
    pub sender: Vec<String>,
    pub reply_to: Vec<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub in_reply_to: String,
    pub message_id: u32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            date: String::new(),
            subject: String::new(),
            from: Vec::new(),
            sender: Vec::new(),
            reply_to: Vec::new(),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            in_reply_to: String::new(),
            message_id: 0,
        }
    }
}


impl From<Option<&imap_proto::types::Envelope<'_>>> for Envelope {
    fn from(from_envelope: Option<&imap_proto::types::Envelope<'_>>) -> Self {
        if let Some(from_envelope) = from_envelope {
            let date = convert_cow_u8_to_string(from_envelope.date.as_ref());

            let subject =
                convert_cow_u8_to_string(from_envelope.subject.as_ref());

            let from =
                convert_vec_address_to_string(from_envelope.from.as_ref());

            let sender =
                convert_vec_address_to_string(from_envelope.sender.as_ref());

            let reply_to =
                convert_vec_address_to_string(from_envelope.reply_to.as_ref());

            let to = convert_vec_address_to_string(from_envelope.to.as_ref());
            let cc = convert_vec_address_to_string(from_envelope.cc.as_ref());
            let bcc = convert_vec_address_to_string(from_envelope.bcc.as_ref());

            let in_reply_to =
                convert_cow_u8_to_string(from_envelope.in_reply_to.as_ref());

            let message_id: u32 =
                convert_cow_u8_to_string(from_envelope.message_id.as_ref())
                .parse()
                .expect("Couldn't get the UID of the mail.");

            Self {
                date,
                subject,
                from,
                sender,
                reply_to,
                to,
                cc,
                bcc,
                in_reply_to,
                message_id,
            }
        } else {
            Envelope::default()
        }
    }
}

// ---------------------
// Helper functions
// ---------------------
fn convert_cow_u8_to_string<'val>(value: Option<&Cow<'val, [u8]>>) -> String {
    String::from_utf8(
        value.unwrap_or(&Cow::Borrowed(&[b' '])).to_vec())
        .unwrap_or(String::new())
}

fn convert_vec_address_to_string<'val>(
    value: Option<&Vec<imap_proto::types::Address<'val>>>,
    ) -> Vec<String> {
    value
        .unwrap_or(&vec![])
        .iter()
        .map(|address| convert_cow_u8_to_string(address.name.as_ref()))
        .collect()
}
