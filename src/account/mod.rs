pub mod arg;
pub mod command;
pub mod config;
pub(crate) mod wizard;

use anyhow::Result;
use serde::Serialize;
use std::{collections::hash_map::Iter, fmt, ops::Deref};

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::table::{Cell, Row, Table},
};

use self::config::TomlAccountConfig;

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
            .cell(Cell::new("BACKENDS").bold().underline().white())
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

impl From<Iter<'_, String, TomlAccountConfig>> for Accounts {
    fn from(map: Iter<'_, String, TomlAccountConfig>) -> Self {
        let mut accounts: Vec<_> = map
            .map(|(name, account)| {
                let mut backends = String::new();

                #[cfg(feature = "imap")]
                if account.imap.is_some() {
                    backends.push_str("imap");
                }

                if account.maildir.is_some() {
                    if !backends.is_empty() {
                        backends.push_str(", ")
                    }
                    backends.push_str("maildir");
                }

                #[cfg(feature = "notmuch")]
                if account.imap.is_some() {
                    if !backends.is_empty() {
                        backends.push_str(", ")
                    }
                    backends.push_str("notmuch");
                }

                #[cfg(feature = "smtp")]
                if account.smtp.is_some() {
                    if !backends.is_empty() {
                        backends.push_str(", ")
                    }
                    backends.push_str("smtp");
                }

                if account.sendmail.is_some() {
                    if !backends.is_empty() {
                        backends.push_str(", ")
                    }
                    backends.push_str("sendmail");
                }

                Account::new(name, &backends, account.default.unwrap_or_default())
            })
            .collect();

        // sort accounts by name
        accounts.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

        Self(accounts)
    }
}
