use imap;
use serde::Serialize;
use std::fmt;

use crate::{
    output::fmt::{get_output_fmt, OutputFmt, Response},
    table::{Cell, Row, Table},
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
            .cell(Cell::new(&self.delim).red())
            .cell(Cell::new(&self.name).green())
            .cell(Cell::new(&self.attributes.join(", ")).shrinkable().yellow())
    }
}

// Mboxes

#[derive(Debug, Serialize)]
pub struct Mboxes(pub Vec<Mbox>);

impl fmt::Display for Mboxes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match get_output_fmt() {
                &OutputFmt::Plain => {
                    writeln!(f, "\n{}", Table::render(&self.0))
                }
                &OutputFmt::Json => {
                    let res = serde_json::to_string(&Response::new(self)).unwrap();
                    write!(f, "{}", res)
                }
            }
        }
    }
}
