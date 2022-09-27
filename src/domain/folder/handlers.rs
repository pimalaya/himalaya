//! Folder handling module.
//!
//! This module gathers all folder actions triggered by the CLI.

use anyhow::Result;
use himalaya_lib::{AccountConfig, Backend};
use log::trace;

use crate::printer::{PrintTableOpts, Printer};

/// Lists all folders.
pub fn list<'a, P: Printer, B: Backend<'a> + ?Sized>(
    max_width: Option<usize>,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let folders = backend.folder_list()?;
    trace!("folders: {:?}", folders);
    printer.print_table(
        // TODO: remove Box
        Box::new(folders),
        PrintTableOpts {
            format: &config.email_reading_format,
            max_width,
        },
    )
}

#[cfg(test)]
mod tests {
    use himalaya_lib::{backend, AccountConfig, Backend, Email, Envelopes, Folder, Folders};
    use std::{fmt::Debug, io};
    use termcolor::ColorSpec;

    use crate::printer::{Print, PrintTable, WriteColor};

    use super::*;

    #[test]
    fn it_should_list_mboxes() {
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
            ) -> anyhow::Result<()> {
                data.print_table(&mut self.writer, opts)?;
                Ok(())
            }
            fn print_str<T: Debug + Print>(&mut self, _data: T) -> anyhow::Result<()> {
                unimplemented!()
            }
            fn print_struct<T: Debug + Print + serde::Serialize>(
                &mut self,
                _data: T,
            ) -> anyhow::Result<()> {
                unimplemented!()
            }
            fn is_json(&self) -> bool {
                unimplemented!()
            }
        }

        struct TestBackend;

        impl<'a> Backend<'a> for TestBackend {
            fn folder_add(&mut self, _: &str) -> backend::Result<()> {
                unimplemented!();
            }
            fn folder_list(&mut self) -> backend::Result<Folders> {
                Ok(Folders(vec![
                    Folder {
                        delim: "/".into(),
                        name: "INBOX".into(),
                        desc: "desc".into(),
                    },
                    Folder {
                        delim: "/".into(),
                        name: "Sent".into(),
                        desc: "desc".into(),
                    },
                ]))
            }
            fn folder_delete(&mut self, _: &str) -> backend::Result<()> {
                unimplemented!();
            }
            fn envelope_list(&mut self, _: &str, _: usize, _: usize) -> backend::Result<Envelopes> {
                unimplemented!()
            }
            fn envelope_search(
                &mut self,
                _: &str,
                _: &str,
                _: &str,
                _: usize,
                _: usize,
            ) -> backend::Result<Envelopes> {
                unimplemented!()
            }
            fn email_add(&mut self, _: &str, _: &[u8], _: &str) -> backend::Result<String> {
                unimplemented!()
            }
            fn email_get(&mut self, _: &str, _: &str) -> backend::Result<Email> {
                unimplemented!()
            }
            fn email_copy(&mut self, _: &str, _: &str, _: &str) -> backend::Result<()> {
                unimplemented!()
            }
            fn email_move(&mut self, _: &str, _: &str, _: &str) -> backend::Result<()> {
                unimplemented!()
            }
            fn email_delete(&mut self, _: &str, _: &str) -> backend::Result<()> {
                unimplemented!()
            }
            fn flags_add(&mut self, _: &str, _: &str, _: &str) -> backend::Result<()> {
                unimplemented!()
            }
            fn flags_set(&mut self, _: &str, _: &str, _: &str) -> backend::Result<()> {
                unimplemented!()
            }
            fn flags_delete(&mut self, _: &str, _: &str, _: &str) -> backend::Result<()> {
                unimplemented!()
            }
            fn as_any(&self) -> &(dyn std::any::Any + 'a) {
                self
            }
        }

        let account_config = AccountConfig::default();
        let mut printer = PrinterServiceTest::default();
        let mut backend = TestBackend {};

        assert!(list(None, &account_config, &mut printer, &mut backend).is_ok());
        assert_eq!(
            concat![
                "\n",
                "DELIM │NAME  │DESC \n",
                "/     │INBOX │desc \n",
                "/     │Sent  │desc \n",
                "\n"
            ],
            printer.writer.content
        );
    }
}
