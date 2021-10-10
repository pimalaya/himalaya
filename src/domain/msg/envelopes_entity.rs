use anyhow::{Error, Result};
use imap::types::{Fetch, ZeroCopy};
use serde::Serialize;
use std::{
    convert::TryFrom,
    fmt::{self, Display},
    ops::Deref,
};

use crate::{domain::msg::Envelope, ui::Table};

/// Representation of a list of envelopes.
#[derive(Debug, Default, Serialize)]
pub struct Envelopes(pub Vec<Envelope>);

impl Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<ZeroCopy<Vec<Fetch>>> for Envelopes {
    type Error = Error;

    fn try_from(fetches: ZeroCopy<Vec<Fetch>>) -> Result<Self> {
        let mut envelopes = vec![];

        for fetch in fetches.iter().rev() {
            envelopes.push(Envelope::try_from(fetch)?);
        }

        Ok(Self(envelopes))
    }
}

impl Display for Envelopes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\n{}", Table::render(&self))
    }
}
