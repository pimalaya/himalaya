use imap;
use serde::Serialize;
use std::fmt;

use crate::{
    output::fmt::{get_output_fmt, OutputFmt, Response},
    table::{self, DisplayRow, DisplayTable},
};

// Mbox

#[derive(Debug, Serialize)]
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

// Mboxes

#[derive(Debug, Serialize)]
pub struct Mboxes(pub Vec<Mbox>);

impl<'a> DisplayTable<'a, Mbox> for Mboxes {
    fn header_row() -> Vec<table::Cell> {
        use crate::table::*;

        vec![
            Cell::new(&[BOLD, UNDERLINE, WHITE], "DELIM"),
            Cell::new(&[BOLD, UNDERLINE, WHITE], "NAME"),
            FlexCell::new(&[BOLD, UNDERLINE, WHITE], "ATTRIBUTES"),
        ]
    }

    fn rows(&self) -> &Vec<Mbox> {
        &self.0
    }
}

impl fmt::Display for Mboxes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match get_output_fmt() {
                &OutputFmt::Plain => {
                    writeln!(f, "\n{}", self.to_table())
                }
                &OutputFmt::Json => {
                    let res = serde_json::to_string(&Response::new(self)).unwrap();
                    write!(f, "{}", res)
                }
            }
        }
    }
}
