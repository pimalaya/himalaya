use chrono::DateTime;
use log::trace;

use crate::{
    backend::{backend::Result, maildir_flags},
    msg::{from_slice_to_addrs, Addr, Envelope},
};

use super::MaildirError;

/// Represents the raw envelope returned by the `maildir` crate.
pub type MaildirEnvelope = maildir::MailEntry;

pub fn from_maildir_entry(mut entry: MaildirEnvelope) -> Result<Envelope> {
    trace!(">> build envelope from maildir parsed mail");

    let mut envelope = Envelope::default();

    envelope.internal_id = entry.id().to_owned();
    envelope.id = format!("{:x}", md5::compute(&envelope.internal_id));
    envelope.flags = maildir_flags::from_maildir_entry(&entry);

    let parsed_mail = entry.parsed().map_err(MaildirError::ParseMsgError)?;

    trace!(">> parse headers");
    for h in parsed_mail.get_headers() {
        let k = h.get_key();
        trace!("header key: {:?}", k);

        let v = rfc2047_decoder::decode(h.get_value_raw())
            .map_err(|err| MaildirError::DecodeHeaderError(err, k.to_owned()))?;
        trace!("header value: {:?}", v);

        match k.to_lowercase().as_str() {
            "date" => {
                envelope.date =
                    DateTime::parse_from_rfc2822(v.split_at(v.find(" (").unwrap_or(v.len())).0)
                        .map(|date| date.naive_local().to_string())
                        .ok()
            }
            "subject" => {
                envelope.subject = v.into();
            }
            "from" => {
                envelope.sender = from_slice_to_addrs(v)
                    .map_err(|err| MaildirError::ParseHeaderError(err, k.to_owned()))?
                    .and_then(|senders| {
                        if senders.is_empty() {
                            None
                        } else {
                            Some(senders)
                        }
                    })
                    .map(|senders| match &senders[0] {
                        Addr::Single(mailparse::SingleInfo { display_name, addr }) => {
                            display_name.as_ref().unwrap_or_else(|| addr).to_owned()
                        }
                        Addr::Group(mailparse::GroupInfo { group_name, .. }) => {
                            group_name.to_owned()
                        }
                    })
                    .ok_or_else(|| MaildirError::FindSenderError)?;
            }
            _ => (),
        }
    }
    trace!("<< parse headers");

    trace!("envelope: {:?}", envelope);
    trace!("<< build envelope from maildir parsed mail");
    Ok(envelope)
}
