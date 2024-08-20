pub mod arg;
pub mod command;
pub mod config;
#[cfg(feature = "wizard")]
pub(crate) mod wizard;

use comfy_table::{Cell, ContentArrangement, Row, Table};
use crossterm::style::Color;
use serde::{Serialize, Serializer};
use std::{collections::hash_map::Iter, fmt, ops::Deref};

use self::config::{ListAccountsTableConfig, TomlAccountConfig};

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

    pub fn to_row(&self, config: &ListAccountsTableConfig) -> Row {
        let mut row = Row::new();

        row.add_cell(Cell::new(&self.name).fg(config.name_color()));
        row.add_cell(Cell::new(&self.backend).fg(config.backends_color()));
        row.add_cell(Cell::new(if self.default { "yes" } else { "" }).fg(config.default_color()));

        row
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Represents the list of printable accounts.
#[derive(Debug, Default, Serialize)]
pub struct Accounts(Vec<Account>);

impl Deref for Accounts {
    type Target = Vec<Account>;

    fn deref(&self) -> &Self::Target {
        &self.0
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
                if account.notmuch.is_some() {
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

pub struct AccountsTable {
    accounts: Accounts,
    width: Option<u16>,
    config: ListAccountsTableConfig,
}

impl AccountsTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
        self.config.preset = preset;
        self
    }

    pub fn with_some_name_color(mut self, color: Option<Color>) -> Self {
        self.config.name_color = color;
        self
    }

    pub fn with_some_backends_color(mut self, color: Option<Color>) -> Self {
        self.config.backends_color = color;
        self
    }

    pub fn with_some_default_color(mut self, color: Option<Color>) -> Self {
        self.config.default_color = color;
        self
    }
}

impl From<Accounts> for AccountsTable {
    fn from(accounts: Accounts) -> Self {
        Self {
            accounts,
            width: None,
            config: Default::default(),
        }
    }
}

impl fmt::Display for AccountsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(self.config.preset())
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new("NAME"),
                Cell::new("BACKENDS"),
                Cell::new("DEFAULT"),
            ]))
            .add_rows(
                self.accounts
                    .iter()
                    .map(|account| account.to_row(&self.config)),
            );

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for AccountsTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.accounts.serialize(serializer)
    }
}
