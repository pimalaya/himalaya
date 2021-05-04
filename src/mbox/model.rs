use imap;
use serde::{
    ser::{self, SerializeSeq},
    Serialize,
};
use std::fmt;

use crate::table::{Cell, Row, Table};

// Attribute

#[derive(Debug, PartialEq)]
struct SerializableAttribute<'a>(&'a imap::types::NameAttribute<'a>);

impl<'a> Into<&'a str> for &'a SerializableAttribute<'a> {
    fn into(self) -> &'a str {
        match &self.0 {
            imap::types::NameAttribute::NoInferiors => "\\NoInferiors",
            imap::types::NameAttribute::NoSelect => "\\NoSelect",
            imap::types::NameAttribute::Marked => "\\Marked",
            imap::types::NameAttribute::Unmarked => "\\Unmarked",
            imap::types::NameAttribute::Custom(cow) => cow,
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

#[derive(Debug, PartialEq)]
pub struct Attributes<'a>(&'a [imap::types::NameAttribute<'a>]);

impl<'a> From<&'a [imap::types::NameAttribute<'a>]> for Attributes<'a> {
    fn from(attrs: &'a [imap::types::NameAttribute<'a>]) -> Self {
        Self(attrs)
    }
}

impl<'a> ToString for Attributes<'a> {
    fn to_string(&self) -> String {
        match self.0.len() {
            0 => String::new(),
            1 => {
                let attr = &SerializableAttribute(&self.0[0]);
                let attr: &str = attr.into();
                attr.to_owned()
            }
            _ => {
                let attr = &SerializableAttribute(&self.0[0]);
                let attr: &str = attr.into();
                format!("{}, {}", attr, Attributes(&self.0[1..]).to_string())
            }
        }
    }
}

impl<'a> ser::Serialize for Attributes<'a> {
    fn serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error>
    where
        T: ser::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for attr in self.0 {
            seq.serialize_element(&SerializableAttribute(attr))?;
        }

        seq.end()
    }
}

// Mailbox

#[derive(Debug, Serialize)]
pub struct Mbox<'a> {
    pub delim: String,
    pub name: String,
    pub attributes: Attributes<'a>,
}

impl<'a> From<&'a imap::types::Name> for Mbox<'a> {
    fn from(name: &'a imap::types::Name) -> Self {
        Self {
            delim: name.delimiter().unwrap_or_default().to_owned(),
            name: name.name().to_owned(),
            attributes: Attributes::from(name.attributes()),
        }
    }
}

impl<'a> Table for Mbox<'a> {
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

// Mboxes

#[derive(Debug, Serialize)]
pub struct Mboxes<'a>(pub Vec<Mbox<'a>>);

impl<'a> From<&'a imap::types::ZeroCopy<Vec<imap::types::Name>>> for Mboxes<'a> {
    fn from(names: &'a imap::types::ZeroCopy<Vec<imap::types::Name>>) -> Self {
        Self(names.iter().map(Mbox::from).collect::<Vec<_>>())
    }
}

impl fmt::Display for Mboxes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\n{}", Table::render(&self.0))
    }
}
