//! Mailbox handling module.
//!
//! This module gathers all mailbox actions triggered by the CLI.

use anyhow::Result;
use log::{info, trace};

use crate::{
    domain::ImapServiceInterface,
    output::{PrintTableOpts, PrinterService},
};

/// Lists all mailboxes.
pub fn list<'a, Printer: PrinterService, ImapService: ImapServiceInterface<'a>>(
    max_width: Option<usize>,
    printer: &mut Printer,
    imap: &'a mut ImapService,
) -> Result<()> {
    info!("entering list mailbox handler");
    let mboxes = imap.fetch_mboxes()?;
    trace!("mailboxes: {:?}", mboxes);
    printer.print_table(mboxes, PrintTableOpts { max_width })
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use std::{fmt::Debug, io};
    use termcolor::ColorSpec;

    use crate::{
        config::{Account, Config},
        domain::{AttrRemote, Attrs, Envelopes, Flags, Mbox, Mboxes, Msg},
        output::{Print, PrintTable, WriteColor},
    };

    use super::*;

    #[test]
    fn it_should_list_mboxes() {
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
            fn print_table<T: Debug + PrintTable + Serialize>(
                &mut self,
                data: T,
                opts: PrintTableOpts,
            ) -> Result<()> {
                data.print_table(&mut self.writter, opts)?;
                Ok(())
            }
            fn print<T: Serialize + Print>(&mut self, _data: T) -> Result<()> {
                unimplemented!()
            }
            fn is_json(&self) -> bool {
                unimplemented!()
            }
        }

        struct ImapServiceTest;

        impl<'a> ImapServiceInterface<'a> for ImapServiceTest {
            fn fetch_mboxes(&'a mut self) -> Result<Mboxes> {
                Ok(Mboxes(vec![
                    Mbox {
                        delim: "/".into(),
                        name: "INBOX".into(),
                        attrs: Attrs::from(vec![AttrRemote::NoSelect]),
                    },
                    Mbox {
                        delim: "/".into(),
                        name: "Sent".into(),
                        attrs: Attrs::from(vec![
                            AttrRemote::NoInferiors,
                            AttrRemote::Custom("HasNoChildren".into()),
                        ]),
                    },
                ]))
            }

            fn notify(&mut self, _: &Config, _: u64) -> Result<()> {
                unimplemented!()
            }
            fn watch(&mut self, _: &Account, _: u64) -> Result<()> {
                unimplemented!()
            }
            fn fetch_envelopes(&mut self, _: &usize, _: &usize) -> Result<Envelopes> {
                unimplemented!()
            }
            fn fetch_envelopes_with(&mut self, _: &str, _: &usize, _: &usize) -> Result<Envelopes> {
                unimplemented!()
            }
            fn find_msg(&mut self, _: &str) -> Result<Msg> {
                unimplemented!()
            }
            fn find_raw_msg(&mut self, _: &str) -> Result<Vec<u8>> {
                unimplemented!()
            }
            fn append_msg(&mut self, _: &Mbox, _: Msg) -> Result<()> {
                unimplemented!()
            }
            fn append_raw_msg_with_flags(&mut self, _: &Mbox, _: &[u8], _: Flags) -> Result<()> {
                unimplemented!()
            }
            fn expunge(&mut self) -> Result<()> {
                unimplemented!()
            }
            fn logout(&mut self) -> Result<()> {
                unimplemented!()
            }
            fn add_flags(&mut self, _: &str, _: &Flags) -> Result<()> {
                unimplemented!()
            }
            fn set_flags(&mut self, _: &str, _: &Flags) -> Result<()> {
                unimplemented!()
            }
            fn remove_flags(&mut self, _: &str, _: &Flags) -> Result<()> {
                unimplemented!()
            }
        }

        let mut printer = PrinterServiceTest::default();
        let mut imap = ImapServiceTest {};

        assert!(list(None, &mut printer, &mut imap).is_ok());
        assert_eq!(
            concat![
                "\n",
                "DELIM │NAME  │ATTRIBUTES                 \n",
                "/     │INBOX │NoSelect                   \n",
                "/     │Sent  │NoInferiors, HasNoChildren \n",
                "\n"
            ],
            printer.writter.content
        );
    }
}
