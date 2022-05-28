//! Mailbox handling module.
//!
//! This module gathers all mailbox actions triggered by the CLI.

use anyhow::Result;
use himalaya_lib::account::AccountConfig;
use log::{info, trace};

use crate::{
    backends::Backend,
    output::{PrintTableOpts, PrinterService},
};

/// Lists all mailboxes.
pub fn list<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    max_width: Option<usize>,
    config: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    info!("entering list mailbox handler");
    let mboxes = backend.get_mboxes()?;
    trace!("mailboxes: {:?}", mboxes);
    printer.print_table(
        mboxes,
        PrintTableOpts {
            format: &config.format,
            max_width,
        },
    )
}

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, io};
    use termcolor::ColorSpec;

    use crate::{
        backends::{ImapMbox, ImapMboxAttr, ImapMboxAttrs, ImapMboxes},
        mbox::Mboxes,
        msg::{Envelopes, Msg},
        output::{Print, PrintTable, WriteColor},
    };

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

        impl PrinterService for PrinterServiceTest {
            fn print_table<T: Debug + PrintTable + erased_serde::Serialize + ?Sized>(
                &mut self,
                data: Box<T>,
                opts: PrintTableOpts,
            ) -> Result<()> {
                data.print_table(&mut self.writer, opts)?;
                Ok(())
            }
            fn print_str<T: Debug + Print>(&mut self, _data: T) -> Result<()> {
                unimplemented!()
            }
            fn print_struct<T: Debug + Print + serde::Serialize>(
                &mut self,
                _data: T,
            ) -> Result<()> {
                unimplemented!()
            }
            fn is_json(&self) -> bool {
                unimplemented!()
            }
        }

        struct TestBackend;

        impl<'a> Backend<'a> for TestBackend {
            fn add_mbox(&mut self, _: &str) -> Result<()> {
                unimplemented!();
            }
            fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
                Ok(Box::new(ImapMboxes {
                    mboxes: vec![
                        ImapMbox {
                            delim: "/".into(),
                            name: "INBOX".into(),
                            attrs: ImapMboxAttrs(vec![ImapMboxAttr::NoSelect]),
                        },
                        ImapMbox {
                            delim: "/".into(),
                            name: "Sent".into(),
                            attrs: ImapMboxAttrs(vec![
                                ImapMboxAttr::NoInferiors,
                                ImapMboxAttr::Custom("HasNoChildren".into()),
                            ]),
                        },
                    ],
                }))
            }
            fn del_mbox(&mut self, _: &str) -> Result<()> {
                unimplemented!();
            }
            fn get_envelopes(&mut self, _: &str, _: usize, _: usize) -> Result<Box<dyn Envelopes>> {
                unimplemented!()
            }
            fn search_envelopes(
                &mut self,
                _: &str,
                _: &str,
                _: &str,
                _: usize,
                _: usize,
            ) -> Result<Box<dyn Envelopes>> {
                unimplemented!()
            }
            fn add_msg(&mut self, _: &str, _: &[u8], _: &str) -> Result<Box<dyn ToString>> {
                unimplemented!()
            }
            fn get_msg(&mut self, _: &str, _: &str) -> Result<Msg> {
                unimplemented!()
            }
            fn copy_msg(&mut self, _: &str, _: &str, _: &str) -> Result<()> {
                unimplemented!()
            }
            fn move_msg(&mut self, _: &str, _: &str, _: &str) -> Result<()> {
                unimplemented!()
            }
            fn del_msg(&mut self, _: &str, _: &str) -> Result<()> {
                unimplemented!()
            }
            fn add_flags(&mut self, _: &str, _: &str, _: &str) -> Result<()> {
                unimplemented!()
            }
            fn set_flags(&mut self, _: &str, _: &str, _: &str) -> Result<()> {
                unimplemented!()
            }
            fn del_flags(&mut self, _: &str, _: &str, _: &str) -> Result<()> {
                unimplemented!()
            }
        }

        let config = AccountConfig::default();
        let mut printer = PrinterServiceTest::default();
        let mut backend = TestBackend {};
        let backend = Box::new(&mut backend);

        assert!(list(None, &config, &mut printer, backend).is_ok());
        assert_eq!(
            concat![
                "\n",
                "DELIM │NAME  │ATTRIBUTES                 \n",
                "/     │INBOX │NoSelect                   \n",
                "/     │Sent  │NoInferiors, HasNoChildren \n",
                "\n"
            ],
            printer.writer.content
        );
    }
}
