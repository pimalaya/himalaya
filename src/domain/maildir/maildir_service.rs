use anyhow::Result;
use imap::extensions::sort::{SortCharset, SortCriterion};

use crate::{
    config::AccountConfig,
    domain::{BackendService, Envelopes, Flags, Mbox, Mboxes, Msg},
};

pub struct MaildirService;

impl<'a> BackendService<'a> for MaildirService {
    fn connect(&mut self) -> Result<()> {
        Ok(())
    }
    fn get_mboxes(&mut self) -> Result<Mboxes> {
        unimplemented!()
    }
    fn get_envelopes(&mut self, _: &usize, _: &usize) -> Result<Envelopes> {
        unimplemented!()
    }
    fn find_envelopes(&mut self, _: &str, _: &usize, _: &usize) -> Result<Envelopes> {
        unimplemented!()
    }
    fn find_and_sort_envelopes(
        &mut self,
        _: &[SortCriterion],
        _: SortCharset,
        _: &str,
        _: &usize,
        _: &usize,
    ) -> Result<Envelopes> {
        unimplemented!()
    }
    fn get_msg(&mut self, _: &AccountConfig, _: &str) -> Result<Msg> {
        unimplemented!()
    }
    fn find_raw_msg(&mut self, _: &str) -> Result<Vec<u8>> {
        unimplemented!()
    }
    fn add_msg(&mut self, _: &Mbox, _: &AccountConfig, _: Msg) -> Result<()> {
        unimplemented!()
    }
    fn append_raw_msg_with_flags(&mut self, _: &Mbox, _: &[u8], _: Flags) -> Result<()> {
        unimplemented!()
    }
    fn expunge(&mut self) -> Result<()> {
        unimplemented!()
    }
    fn disconnect(&mut self) -> Result<()> {
        unimplemented!()
    }
    fn add_flags(&mut self, _: &str, _: &Flags) -> Result<()> {
        unimplemented!()
    }
    fn set_flags(&mut self, _: &str, _: &Flags) -> Result<()> {
        unimplemented!()
    }
    fn del_flags(&mut self, _: &str, _: &Flags) -> Result<()> {
        unimplemented!()
    }
}
