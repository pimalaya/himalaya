//! Mailbox handling module.
//!
//! This module gathers all mailbox actions triggered by the CLI.

use anyhow::Result;
use log::trace;

use crate::{domain::ImapServiceInterface, output::OutputServiceInterface};

/// List all mailboxes.
pub fn list<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    output: &OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let mboxes = imap.fetch_mboxes()?;
    trace!("mailboxes: {:#?}", mboxes);
    output.print(mboxes)
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use std::fmt::Display;

    use super::*;
    use crate::{
        config::Config,
        domain::{AttrRemote, Attrs, Envelopes, Flags, Mbox, Mboxes, Msg},
        output::OutputJson,
    };

    #[test]
    fn it_should_list_mboxes() {
        struct OutputServiceTest;

        impl OutputServiceInterface for OutputServiceTest {
            fn print<T: Serialize + Display>(&self, data: T) -> Result<()> {
                let data = serde_json::to_string(&OutputJson::new(data))?;
                assert_eq!(
                    data,
                    r#"{"response":[{"delim":"/","name":"INBOX","attrs":["NoSelect"]},{"delim":"/","name":"Sent","attrs":["NoInferiors",{"Custom":"HasNoChildren"}]}]}"#
                );
                Ok(())
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

            fn watch(&mut self, _: u64) -> Result<()> {
                unimplemented!()
            }

            fn get_msgs(&mut self, _: &usize, _: &usize) -> Result<Envelopes> {
                unimplemented!()
            }

            fn find_msgs(&mut self, _: &str, _: &usize, _: &usize) -> Result<Envelopes> {
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

        let output = OutputServiceTest {};
        let mut imap = ImapServiceTest {};

        assert!(list(&output, &mut imap).is_ok());
    }
}
