//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the envelope

use anyhow::{anyhow, Context, Error};
use log::{debug, info, trace};
use std::convert::{TryFrom, TryInto};

use crate::{
    domain::from_slice_to_addrs,
    msg::{Envelope, Envelopes},
};

/// Represents a list of raw envelopees returned by the `maildir` crate.
pub type RawMaildirEnvelopes = Vec<maildir::MailEntry>;

impl<'a> TryFrom<&'a mut RawMaildirEnvelopes> for Envelopes<'a> {
    type Error = Error;

    fn try_from(raw_envelopes: &'a mut RawMaildirEnvelopes) -> Result<Self, Self::Error> {
        let mut envelopes = vec![];
        for raw_envelope in raw_envelopes {
            envelopes.push(
                raw_envelope
                    .parsed()
                    .context("cannot parse maildir mail entry")?
                    .try_into()?,
            );
        }
        Ok(Envelopes(envelopes))
    }
}

/// Represents the raw envelope returned by the `maildir` crate.
pub type RawMaildirEnvelope<'a> = mailparse::ParsedMail<'a>;

impl<'a> TryFrom<RawMaildirEnvelope<'a>> for Envelope<'a> {
    type Error = Error;

    fn try_from(parsed_mail: RawMaildirEnvelope<'a>) -> Result<Self, Self::Error> {
        info!("begin: building envelope from parsed mail");
        trace!("parsed mail: {:?}", parsed_mail);

        let mut envelope = Self::default();

        debug!("parsing headers");
        for header in parsed_mail.get_headers() {
            let key = header.get_key();
            debug!("header key: {:?}", key);

            let val = header.get_value();
            let val = String::from_utf8(header.get_value_raw().to_vec())
                .map(|val| val.trim().to_string())
                .context(format!(
                    "cannot decode value {:?} from header {:?}",
                    key, val
                ))?;
            debug!("header value: {:?}", val);

            match key.to_lowercase().as_str() {
                "subject" => {
                    envelope.subject = val.clone().into();
                }
                "from" => {
                    envelope.sender = from_slice_to_addrs(val)
                        .context(format!("cannot parse header {:?}", key))?
                        .ok_or_else(|| anyhow!("cannot find sender"))?
                        .to_string()
                }
                _ => (),
            }
        }

        trace!("envelope: {:?}", envelope);
        info!("end: building envelope from parsed mail");
        Ok(envelope)
    }
}
