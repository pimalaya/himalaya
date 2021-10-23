use anyhow::{Error, Result};
use serde::Serialize;
use std::{convert::TryFrom, ops::Deref};

use crate::{
    domain::{msg::Envelope, RawEnvelope},
    output::{Print, WriteWithColor},
    ui::Table,
};

pub type RawEnvelopes = imap::types::ZeroCopy<Vec<RawEnvelope>>;

/// Representation of a list of envelopes.
#[derive(Debug, Default, Serialize)]
pub struct Envelopes<'a>(pub Vec<Envelope<'a>>);

impl<'a> Deref for Envelopes<'a> {
    type Target = Vec<Envelope<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> TryFrom<&'a RawEnvelopes> for Envelopes<'a> {
    type Error = Error;

    fn try_from(fetches: &'a RawEnvelopes) -> Result<Self> {
        let mut envelopes = vec![];

        for fetch in fetches.iter().rev() {
            envelopes.push(Envelope::try_from(fetch)?);
        }

        Ok(Self(envelopes))
    }
}

impl<'a> Print for Envelopes<'a> {
    fn print<W: WriteWithColor>(&self, writter: &mut W) -> Result<()> {
        println!();
        Table::println(writter, &self)
    }
}
