pub mod arg;
pub mod command;
pub mod config;
pub(crate) mod wizard;

use color_eyre::Result;
use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Row, Table};
use serde::Serialize;
use std::{collections::hash_map::Iter, fmt, ops::Deref};

use crate::printer::{PrintTable, WriteColor};

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

impl From<Account> for Row {
    fn from(account: Account) -> Self {
        let mut r = Row::new();
        r.add_cell(Cell::new(account.name).fg(Color::Green));
        r.add_cell(Cell::new(account.backend).fg(Color::Blue));
        r.add_cell(Cell::new(if account.default { "yes" } else { "" }).fg(Color::White));
        r
    }
}
impl From<&Account> for Row {
    fn from(account: &Account) -> Self {
        let mut r = Row::new();
        r.add_cell(Cell::new(&account.name).fg(Color::Green));
        r.add_cell(Cell::new(&account.backend).fg(Color::Blue));
        r.add_cell(Cell::new(if account.default { "yes" } else { "" }).fg(Color::White));
        r
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

impl From<Accounts> for Table {
    fn from(accounts: Accounts) -> Self {
        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(Row::from([
                Cell::new("NAME").add_attribute(Attribute::Reverse),
                Cell::new("BACKENDS").add_attribute(Attribute::Reverse),
                Cell::new("DEFAULT").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(accounts.0.into_iter().map(Row::from));
        table
    }
}

impl From<&Accounts> for Table {
    fn from(accounts: &Accounts) -> Self {
        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(Row::from([
                Cell::new("NAME").add_attribute(Attribute::Reverse),
                Cell::new("BACKENDS").add_attribute(Attribute::Reverse),
                Cell::new("DEFAULT").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(accounts.0.iter().map(Row::from));
        table
    }
}

impl PrintTable for Accounts {
    fn print_table(&self, writer: &mut dyn WriteColor, table_max_width: Option<u16>) -> Result<()> {
        let mut table = Table::from(self);
        if let Some(width) = table_max_width {
            table.set_width(width);
        }
        writeln!(writer)?;
        write!(writer, "{}", table)?;
        writeln!(writer)?;
        Ok(())
    }
}

impl From<Iter<'_, String, TomlAccountConfig>> for Accounts {
    fn from(map: Iter<'_, String, TomlAccountConfig>) -> Self {
        let mut accounts: Vec<_> = map
            .map(|(name, account)| {
                #[allow(unused_mut)]
                let mut backends = String::new();

                #[cfg(feature = "imap")]
                if account.imap.is_some() {
                    backends.push_str("imap");
                }

                #[cfg(feature = "maildir")]
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

                #[cfg(feature = "sendmail")]
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
