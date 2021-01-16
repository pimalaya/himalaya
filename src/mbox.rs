use imap;

use crate::table::{self, DisplayRow, DisplayTable};

pub struct Mbox {
    pub delim: String,
    pub name: String,
    pub attributes: Vec<String>,
}

impl Mbox {
    pub fn from_name(name: &imap::types::Name) -> Self {
        Self {
            delim: name.delimiter().unwrap_or_default().to_owned(),
            name: name.name().to_owned(),
            attributes: vec![], // TODO: set attributes
        }
    }
}

impl DisplayRow for Mbox {
    fn to_row(&self) -> Vec<table::Cell> {
        vec![
            table::Cell::new(&[table::BLUE], &self.delim),
            table::Cell::new(&[table::GREEN], &self.name),
            table::Cell::new(&[table::YELLOW], &self.attributes.join(", ")),
        ]
    }
}

impl<'a> DisplayTable<'a, Mbox> for Vec<Mbox> {
    fn cols() -> &'a [&'a str] {
        &["delim", "name", "attributes"]
    }

    fn rows(&self) -> &Vec<Mbox> {
        self
    }
}
