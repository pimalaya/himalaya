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
        use crate::table::*;

        vec![
            Cell::new(&[BLUE], &self.delim),
            Cell::new(&[GREEN], &self.name),
            FlexCell::new(&[YELLOW], &self.attributes.join(", ")),
        ]
    }
}

impl<'a> DisplayTable<'a, Mbox> for Vec<Mbox> {
    fn header_row() -> Vec<table::Cell> {
        use crate::table::*;

        vec![
            Cell::new(&[BOLD, UNDERLINE, WHITE], "DELIM"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "NAME"),
            FlexCell::new(&[BOLD, UNDERLINE, WHITE], "ATTRIBUTES"),
        ]
    }

    fn rows(&self) -> &Vec<Mbox> {
        self
    }
}
