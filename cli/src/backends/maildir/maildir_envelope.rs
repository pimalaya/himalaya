use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use himalaya_lib::msg::Envelope;
use log::trace;

use crate::{
    backends::maildir_flags,
    msg::{from_slice_to_addrs, Addr},
};

/// Represents the raw envelope returned by the `maildir` crate.
pub type MaildirEnvelope = maildir::MailEntry;

pub fn from_maildir_entry(mut entry: MaildirEnvelope) -> Result<Envelope> {
    trace!(">> build envelope from maildir parsed mail");

    let mut envelope = Envelope::default();

    envelope.internal_id = entry.id().to_owned();
    envelope.id = format!("{:x}", md5::compute(&envelope.internal_id));
    envelope.flags = maildir_flags::from_maildir_entry(&entry);

    let parsed_mail = entry.parsed().context("cannot parse maildir mail entry")?;

    trace!(">> parse headers");
    for h in parsed_mail.get_headers() {
        let k = h.get_key();
        trace!("header key: {:?}", k);

        let v = rfc2047_decoder::decode(h.get_value_raw())
            .context(format!("cannot decode value from header {:?}", k))?;
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
                    .context(format!("cannot parse header {:?}", k))?
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
                    .ok_or_else(|| anyhow!("cannot find sender"))?;
            }
            _ => (),
        }
    }
    trace!("<< parse headers");

    trace!("envelope: {:?}", envelope);
    trace!("<< build envelope from maildir parsed mail");
    Ok(envelope)
}
