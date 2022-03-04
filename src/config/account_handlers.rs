//! Account handlers module.
//!
//! This module gathers all account actions triggered by the CLI.

use anyhow::Result;
use log::{info, trace};

use crate::{
    config::{AccountConfig, Accounts, DeserializedConfig},
    output::{PrintTableOpts, PrinterService},
};

/// Lists all accounts.
pub fn list<'a, P: PrinterService>(
    max_width: Option<usize>,
    config: &DeserializedConfig,
    account_config: &AccountConfig,
    printer: &mut P,
) -> Result<()> {
    info!(">> account list handler");

    let accounts: Accounts = config.accounts.iter().into();
    trace!("accounts: {:?}", accounts);

    printer.print_table(
        Box::new(accounts),
        PrintTableOpts {
            format: &account_config.format,
            max_width,
        },
    )?;

    info!("<< account list handler");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fmt::Debug, io, iter::FromIterator};
    use termcolor::ColorSpec;

    use crate::{
        config::{DeserializedAccountConfig, DeserializedImapAccountConfig},
        output::{Print, PrintTable, WriteColor},
    };

    use super::*;

    #[test]
    fn it_should_match_cmds_accounts() {
        #[derive(Debug, Default, Clone)]
        struct StringWritter {
            content: String,
        }

        impl io::Write for StringWritter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.content
                    .push_str(&String::from_utf8(buf.to_vec()).unwrap());
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                self.content = String::default();
                Ok(())
            }
        }

        impl termcolor::WriteColor for StringWritter {
            fn supports_color(&self) -> bool {
                false
            }

            fn set_color(&mut self, _spec: &ColorSpec) -> io::Result<()> {
                io::Result::Ok(())
            }

            fn reset(&mut self) -> io::Result<()> {
                io::Result::Ok(())
            }
        }

        impl WriteColor for StringWritter {}

        #[derive(Debug, Default)]
        struct PrinterServiceTest {
            pub writter: StringWritter,
        }

        impl PrinterService for PrinterServiceTest {
            fn print_table<T: Debug + PrintTable + erased_serde::Serialize + ?Sized>(
                &mut self,
                data: Box<T>,
                opts: PrintTableOpts,
            ) -> Result<()> {
                data.print_table(&mut self.writter, opts)?;
                Ok(())
            }
            fn print<T: serde::Serialize + Print>(&mut self, _data: T) -> Result<()> {
                unimplemented!()
            }
            fn is_json(&self) -> bool {
                unimplemented!()
            }
        }

        let config = DeserializedConfig {
            accounts: HashMap::from_iter([(
                "account-1".into(),
                DeserializedAccountConfig::Imap(DeserializedImapAccountConfig {
                    default: Some(true),
                    ..DeserializedImapAccountConfig::default()
                }),
            )]),
            ..DeserializedConfig::default()
        };

        let account_config = AccountConfig::default();
        let mut printer = PrinterServiceTest::default();

        assert!(list(None, &config, &account_config, &mut printer).is_ok());
        assert_eq!(
            concat![
                "\n",
                "NAME      │BACKEND │DEFAULT \n",
                "account-1 │imap    │yes     \n",
                "\n"
            ],
            printer.writter.content
        );
    }
}
