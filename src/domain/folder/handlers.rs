//! Folder handling module.
//!
//! This module gathers all folder actions triggered by the CLI.

use anyhow::Result;
use himalaya_lib::{AccountConfig, Backend};

use crate::printer::{PrintTableOpts, Printer};

pub fn expunge<P: Printer, B: Backend + ?Sized>(
    folder: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.expunge_folder(folder)?;
    printer.print(format!("Folder {folder} successfully expunged!"))
}

pub fn list<P: Printer, B: Backend + ?Sized>(
    max_width: Option<usize>,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    let folders = backend.list_folders()?;
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
    use himalaya_lib::{
        backend, AccountConfig, Backend, Emails, Envelope, Envelopes, Flags, Folder, Folders,
    };
    use std::{any::Any, fmt::Debug, io};
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
            fn print_log<T: Debug + Print>(&mut self, _data: T) -> anyhow::Result<()> {
                unimplemented!()
            }
            fn print<T: Debug + Print + serde::Serialize>(
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

        impl Backend for TestBackend {
            fn name(&self) -> String {
                unimplemented!();
            }
            fn add_folder(&self, _: &str) -> backend::Result<()> {
                unimplemented!();
            }
            fn list_folders(&self) -> backend::Result<Folders> {
                Ok(Folders::from_iter([
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
            fn expunge_folder(&self, _: &str) -> backend::Result<()> {
                unimplemented!();
            }
            fn purge_folder(&self, _: &str) -> backend::Result<()> {
                unimplemented!();
            }
            fn delete_folder(&self, _: &str) -> backend::Result<()> {
                unimplemented!();
            }
            fn get_envelope(&self, _: &str, _: &str) -> backend::Result<Envelope> {
                unimplemented!();
            }
            fn get_envelope_internal(&self, _: &str, _: &str) -> backend::Result<Envelope> {
                unimplemented!();
            }
            fn list_envelopes(&self, _: &str, _: usize, _: usize) -> backend::Result<Envelopes> {
                unimplemented!()
            }
            fn search_envelopes(
                &self,
                _: &str,
                _: &str,
                _: &str,
                _: usize,
                _: usize,
            ) -> backend::Result<Envelopes> {
                unimplemented!()
            }
            fn add_email(&self, _: &str, _: &[u8], _: &Flags) -> backend::Result<String> {
                unimplemented!()
            }
            fn add_email_internal(&self, _: &str, _: &[u8], _: &Flags) -> backend::Result<String> {
                unimplemented!()
            }
            fn get_emails(&self, _: &str, _: Vec<&str>) -> backend::Result<Emails> {
                unimplemented!()
            }
            fn preview_emails(&self, _: &str, _: Vec<&str>) -> backend::Result<Emails> {
                unimplemented!()
            }
            fn get_emails_internal(&self, _: &str, _: Vec<&str>) -> backend::Result<Emails> {
                unimplemented!()
            }
            fn copy_emails(&self, _: &str, _: &str, _: Vec<&str>) -> backend::Result<()> {
                unimplemented!()
            }
            fn copy_emails_internal(&self, _: &str, _: &str, _: Vec<&str>) -> backend::Result<()> {
                unimplemented!()
            }
            fn move_emails(&self, _: &str, _: &str, _: Vec<&str>) -> backend::Result<()> {
                unimplemented!()
            }
            fn move_emails_internal(&self, _: &str, _: &str, _: Vec<&str>) -> backend::Result<()> {
                unimplemented!()
            }
            fn delete_emails(&self, _: &str, _: Vec<&str>) -> backend::Result<()> {
                unimplemented!()
            }
            fn delete_emails_internal(&self, _: &str, _: Vec<&str>) -> backend::Result<()> {
                unimplemented!()
            }
            fn add_flags(&self, _: &str, _: Vec<&str>, _: &Flags) -> backend::Result<()> {
                unimplemented!()
            }
            fn add_flags_internal(&self, _: &str, _: Vec<&str>, _: &Flags) -> backend::Result<()> {
                unimplemented!()
            }
            fn set_flags(&self, _: &str, _: Vec<&str>, _: &Flags) -> backend::Result<()> {
                unimplemented!()
            }
            fn set_flags_internal(&self, _: &str, _: Vec<&str>, _: &Flags) -> backend::Result<()> {
                unimplemented!()
            }
            fn remove_flags(&self, _: &str, _: Vec<&str>, _: &Flags) -> backend::Result<()> {
                unimplemented!()
            }
            fn remove_flags_internal(
                &self,
                _: &str,
                _: Vec<&str>,
                _: &Flags,
            ) -> backend::Result<()> {
                unimplemented!()
            }
            fn as_any(&'static self) -> &(dyn Any) {
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
