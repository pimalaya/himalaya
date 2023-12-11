use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::Accounts,
    config::TomlConfig,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::TableMaxWidthFlag,
};

/// List all accounts.
///
/// This command lists all accounts defined in your TOML configuration
/// file.
#[derive(Debug, Parser)]
pub struct AccountListCommand {
    #[command(flatten)]
    pub table: TableMaxWidthFlag,
}

impl AccountListCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing account list command");

        let accounts: Accounts = config.accounts.iter().into();

        printer.print_table(
            Box::new(accounts),
            PrintTableOpts {
                format: &Default::default(),
                max_width: self.table.max_width,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use email::{account::config::AccountConfig, imap::config::ImapConfig};
    use std::{collections::HashMap, fmt::Debug, io};
    use termcolor::ColorSpec;

    use crate::{
        account::TomlAccountConfig,
        backend::BackendKind,
        printer::{Print, PrintTable, WriteColor},
    };

    use super::*;

    #[test]
    fn it_should_match_cmds_accounts() {
        #[derive(Debug, Default, Clone)]
        struct StringWriter {
            content: String,
        }

        impl io::Write for StringWriter {
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

        impl termcolor::WriteColor for StringWriter {
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

        impl WriteColor for StringWriter {}

        #[derive(Debug, Default)]
        struct PrinterServiceTest {
            pub writer: StringWriter,
        }

        impl Printer for PrinterServiceTest {
            fn print_table<T: Debug + PrintTable + erased_serde::Serialize + ?Sized>(
                &mut self,
                data: Box<T>,
                opts: PrintTableOpts,
            ) -> Result<()> {
                data.print_table(&mut self.writer, opts)?;
                Ok(())
            }
            fn print_log<T: Debug + Print>(&mut self, _data: T) -> Result<()> {
                unimplemented!()
            }
            fn print<T: Debug + Print + serde::Serialize>(&mut self, _data: T) -> Result<()> {
                unimplemented!()
            }
            fn is_json(&self) -> bool {
                unimplemented!()
            }
        }

        let mut printer = PrinterServiceTest::default();
        let config = AccountConfig::default();
        let deserialized_config = TomlConfig {
            accounts: HashMap::from_iter([(
                "account-1".into(),
                TomlAccountConfig {
                    default: Some(true),
                    backend: Some(BackendKind::Imap),
                    imap: Some(ImapConfig::default()),
                    ..Default::default()
                },
            )]),
            ..TomlConfig::default()
        };

        assert!(list(None, &config, &deserialized_config, &mut printer).is_ok());
        assert_eq!(
            concat![
                "\n",
                "NAME      │BACKEND │DEFAULT \n",
                "account-1 │imap    │yes     \n",
                "\n"
            ],
            printer.writer.content
        );
    }
}
