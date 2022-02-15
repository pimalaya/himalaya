use anyhow::Result;

use crate::{
    config::AccountConfig,
    domain::{BackendService, Envelopes, Flags, Mbox, Mboxes, Msg, SortCriterion},
};

pub struct MaildirService;

impl<'a> BackendService<'a> for MaildirService {
    fn connect(&mut self) -> Result<()> {
        Ok(())
    }
    fn get_mboxes(&mut self) -> Result<Mboxes> {
        unimplemented!()
    }
    fn get_envelopes(
        &mut self,
        _: &[SortCriterion],
        _: &str,
        _: &usize,
        _: &usize,
    ) -> Result<Envelopes> {
        unimplemented!()
    }
    fn get_msg(&mut self, _: &AccountConfig, _: &str) -> Result<Msg> {
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
