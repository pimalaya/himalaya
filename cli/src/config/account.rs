//! Account module.
//!
//! This module contains the definition of the printable account,
//! which is only used by the "accounts" command to list all available
//! accounts from the config file.

use anyhow::Result;
use serde::Serialize;
use std::{
    collections::hash_map::Iter,
    fmt::{self, Display},
    ops::Deref,
};

use crate::{
    config::DeserializedAccountConfig,
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents the list of printable accounts.
#[derive(Debug, Default, Serialize)]
pub struct Accounts(pub Vec<Account>);

impl Deref for Accounts {
    type Target = Vec<Account>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PrintTable for Accounts {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}

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

impl Display for Account {
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

impl From<Iter<'_, String, DeserializedAccountConfig>> for Accounts {
    fn from(map: Iter<'_, String, DeserializedAccountConfig>) -> Self {
        let mut accounts: Vec<_> = map
            .map(|(name, config)| match config {
                #[cfg(feature = "imap-backend")]
                DeserializedAccountConfig::Imap(config) => {
                    Account::new(name, "imap", config.default.unwrap_or_default())
                }
                #[cfg(feature = "maildir-backend")]
                DeserializedAccountConfig::Maildir(config) => {
                    Account::new(name, "maildir", config.default.unwrap_or_default())
                }
                #[cfg(feature = "notmuch-backend")]
                DeserializedAccountConfig::Notmuch(config) => {
                    Account::new(name, "notmuch", config.default.unwrap_or_default())
                }
            })
            .collect();
        accounts.sort_by(|a, b| b.name.partial_cmp(&a.name).unwrap());
        Self(accounts)
    }
}
