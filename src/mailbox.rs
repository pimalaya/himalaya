use imap;

use crate::table::{self, DisplayCell, DisplayRow, DisplayTable};

pub struct Delim(String);

impl Delim {
    pub fn from_name(name: &imap::types::Name) -> Self {
        Self(name.delimiter().unwrap_or("/").to_string())
    }
}

impl DisplayCell for Delim {
    fn styles(&self) -> &[table::Style] {
        &[table::BLUE]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

pub struct Name(String);

impl Name {
    pub fn from_name(name: &imap::types::Name) -> Self {
        Self(name.name().to_string())
    }
}

impl DisplayCell for Name {
    fn styles(&self) -> &[table::Style] {
        &[table::GREEN]
    }

    fn value(&self) -> String {
        self.0.to_owned()
    }
}

pub struct Attributes<'a>(Vec<imap::types::NameAttribute<'a>>);

impl Attributes<'_> {
    pub fn from_name(name: &imap::types::Name) -> Self {
        let attrs = name.attributes().iter().fold(vec![], |mut attrs, attr| {
            use imap::types::NameAttribute::*;

            match attr {
                NoInferiors => attrs.push(NoInferiors),
                NoSelect => attrs.push(NoSelect),
                Marked => attrs.push(Marked),
                Unmarked => attrs.push(Unmarked),
                _ => (),
            };

            attrs
        });

        Self(attrs)
    }
}

impl DisplayCell for Attributes<'_> {
    fn styles(&self) -> &[table::Style] {
        &[table::YELLOW]
    }

    fn value(&self) -> String {
        use imap::types::NameAttribute::*;

        self.0
            .iter()
            .map(|attr| match attr {
                NoInferiors => vec!["no inferiors"],
                NoSelect => vec!["no select"],
                Marked => vec!["marked"],
                Unmarked => vec!["unmarked"],
                _ => vec![],
            })
            .collect::<Vec<_>>()
            .concat()
            .join(", ")
    }
}

pub struct Mailbox<'a> {
    pub delim: Delim,
    pub name: Name,
    pub attributes: Attributes<'a>,
}

impl Mailbox<'_> {
    pub fn from_name(name: &imap::types::Name) -> Self {
        Self {
            delim: Delim::from_name(name),
            name: Name::from_name(name),
            attributes: Attributes::from_name(name),
        }
    }
}

impl<'a> DisplayRow for Mailbox<'a> {
    fn to_row(&self) -> Vec<table::Cell> {
        vec![
            self.delim.to_cell(),
            self.name.to_cell(),
            self.attributes.to_cell(),
        ]
    }
}

impl<'a> DisplayTable<'a, Mailbox<'a>> for Vec<Mailbox<'a>> {
    fn cols() -> &'a [&'a str] {
        &["delim", "name", "attributes"]
    }

    fn rows(&self) -> &Vec<Mailbox<'a>> {
        self
    }
}
