use anyhow::{anyhow, Error, Result};
use imap::types::NameAttribute;
use log::debug;
use serde::{
    ser::{self, SerializeSeq},
    Serialize,
};
use std::collections::HashSet;
use std::fmt;
use std::{borrow::Cow, convert::TryFrom};

use crate::ui::table::{Cell, Row, Table};

// Attribute

#[derive(Debug, PartialEq)]
struct SerializableAttribute<'a>(&'a NameAttribute<'a>);

impl<'a> Into<&'a str> for &'a SerializableAttribute<'a> {
    fn into(self) -> &'a str {
        match &self.0 {
            NameAttribute::NoInferiors => "\\NoInferiors",
            NameAttribute::NoSelect => "\\NoSelect",
            NameAttribute::Marked => "\\Marked",
            NameAttribute::Unmarked => "\\Unmarked",
            NameAttribute::Custom(cow) => cow,
        }
    }
}

impl<'a> ser::Serialize for SerializableAttribute<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(self.into())
    }
}

/// Represents the attributes of a mailbox.
#[derive(Debug, PartialEq)]
pub struct Attributes(pub HashSet<NameAttribute<'static>>);

impl<'a> From<&[NameAttribute<'a>]> for Attributes {
    fn from(attrs: &[NameAttribute<'a>]) -> Self {
        Self(
            attrs
                .iter()
                .map(|attribute| convert_to_static(attribute).unwrap())
                .collect::<HashSet<NameAttribute<'static>>>(),
        )
    }
}

impl ToString for Attributes {
    fn to_string(&self) -> String {
        let mut attributes = String::new();

        for attribute in &self.0 {
            let attribute = SerializableAttribute(&attribute);
            attributes.push_str((&attribute).into());
            attributes.push_str(", ");
        }

        // remove the trailing whitespace with the comma
        attributes = attributes.trim_end_matches(' ').to_string();
        attributes.pop();

        attributes
    }
}

impl ser::Serialize for Attributes {
    fn serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error>
    where
        T: ser::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for attr in &self.0 {
            seq.serialize_element(&SerializableAttribute(attr))?;
        }

        seq.end()
    }
}

// --- Mailbox ---
/// Represents a general mailbox.
#[derive(Debug, Serialize)]
pub struct Mbox {
    /// The [hierarchie delimiter].
    ///
    /// [hierarchie delimiter]: https://docs.rs/imap/2.4.1/imap/types/struct.Name.html#method.delimiter
    pub delim: String,

    /// The name of the mailbox.
    pub name: String,

    /// Its attributes.
    pub attributes: Attributes,
}

impl Default for Mbox {
    fn default() -> Self {
        Self {
            delim: String::default(),
            name: String::default(),
            attributes: Attributes::from(&[] as &[NameAttribute]),
        }
    }
}

impl fmt::Display for Mbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<&str> for Mbox {
    fn from(mbox: &str) -> Self {
        debug!("init mailbox from `{:?}`", mbox);
        Self {
            name: mbox.to_owned(),
            ..Self::default()
        }
    }
}

impl<'a> From<&'a imap::types::Name> for Mbox {
    fn from(name: &'a imap::types::Name) -> Self {
        Self {
            delim: name.delimiter().unwrap_or_default().to_owned(),
            name: name.name().to_owned(),
            attributes: Attributes::from(name.attributes()),
        }
    }
}

impl TryFrom<Option<&str>> for Mbox {
    type Error = Error;

    fn try_from(mbox: Option<&str>) -> Result<Self, Self::Error> {
        debug!("init mailbox from `{:?}`", mbox);
        Ok(Self {
            name: mbox
                .ok_or(anyhow!("the target mailbox cannot be empty"))?
                .to_string(),
            ..Self::default()
        })
    }
}

impl Table for Mbox {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("DELIM").bold().underline().white())
            .cell(Cell::new("NAME").bold().underline().white())
            .cell(
                Cell::new("ATTRIBUTES")
                    .shrinkable()
                    .bold()
                    .underline()
                    .white(),
            )
    }

    fn row(&self) -> Row {
        Row::new()
            .cell(Cell::new(&self.delim).white())
            .cell(Cell::new(&self.name).green())
            .cell(Cell::new(&self.attributes.to_string()).shrinkable().blue())
    }
}

// --- Mboxes ---
/// A simple wrapper to acces a bunch of mboxes which are in this vector.
#[derive(Debug, Serialize)]
pub struct Mboxes(pub Vec<Mbox>);

impl<'a> From<&'a imap::types::ZeroCopy<Vec<imap::types::Name>>> for Mboxes {
    fn from(names: &'a imap::types::ZeroCopy<Vec<imap::types::Name>>) -> Self {
        Self(names.iter().map(Mbox::from).collect::<Vec<_>>())
    }
}

impl fmt::Display for Mboxes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\n{}", Table::render(&self.0))
    }
}

// == Helper Functions ==
fn convert_to_static<'func>(
    attribute: &'func NameAttribute<'func>,
) -> Result<NameAttribute<'static>, ()> {
    match attribute {
        NameAttribute::NoInferiors => Ok(NameAttribute::NoInferiors),
        NameAttribute::NoSelect => Ok(NameAttribute::NoSelect),
        NameAttribute::Marked => Ok(NameAttribute::Marked),
        NameAttribute::Unmarked => Ok(NameAttribute::Unmarked),
        NameAttribute::Custom(cow) => Ok(NameAttribute::Custom(Cow::Owned(cow.to_string()))),
    }
}
