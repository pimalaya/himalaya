//! Account module.
//!
//! This module contains the definition of the printable account,
//! which is only used by the "accounts" command to list all available
//! accounts from the config file.

use serde::Serialize;
use std::fmt;

use crate::ui::table::{Cell, Row, Table};

/// Represents the printable account.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Account {
    /// Represents the account name.
    pub name: String,
    /// Represents the backend name of the account.
    pub backend: String,
    /// Represents the default state of the account.
    pub default: bool,
}

impl Account {
    pub fn new(name: &str, backend: &str, default: bool) -> Self {
        Self {
            name: name.into(),
            backend: backend.into(),
            default,
        }
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Table for Account {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("NAME").shrinkable().bold().underline().white())
            .cell(Cell::new("BACKEND").bold().underline().white())
            .cell(Cell::new("DEFAULT").bold().underline().white())
    }

    fn row(&self) -> Row {
        let default = if self.default { "yes" } else { "" };
        Row::new()
            .cell(Cell::new(&self.name).shrinkable().green())
            .cell(Cell::new(&self.backend).blue())
            .cell(Cell::new(default).white())
    }
}
