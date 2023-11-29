//! Account module.
//!
//! This module contains the definition of the printable account,
//! which is only used by the "accounts" command to list all available
//! accounts from the config file.

use anyhow::Result;
use serde::Serialize;
use std::{collections::hash_map::Iter, ops::Deref};

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
};

use super::{Account, TomlAccountConfig};

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

                #[cfg(feature = "imap-backend")]
                if account.imap.is_some() {
                    backends.push_str("imap");
                }

                if account.maildir.is_some() {
                    if !backends.is_empty() {
                        backends.push_str(", ")
                    }
                    backends.push_str("maildir");
                }

                #[cfg(feature = "notmuch-backend")]
                if account.imap.is_some() {
                    if !backends.is_empty() {
                        backends.push_str(", ")
                    }
                    backends.push_str("notmuch");
                }

                #[cfg(feature = "smtp-sender")]
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
        accounts.sort_by(|a, b| b.name.partial_cmp(&a.name).unwrap());
        Self(accounts)
    }
}
