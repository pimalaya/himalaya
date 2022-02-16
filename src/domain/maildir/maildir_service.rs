use anyhow::{anyhow, Context, Result};
use imap::types::Flag;
use std::{borrow::BorrowMut, convert::TryFrom, path::PathBuf};

use crate::{
    config::{AccountConfig, MaildirBackendConfig},
    domain::{BackendService, Envelope, Envelopes, Flags, Mbox, Mboxes, Msg, SortCriterion},
};

pub struct MaildirService<'a> {
    account_config: &'a AccountConfig,
    maildir_config: &'a MaildirBackendConfig,
    maildir: maildir::Maildir,
}

impl<'a> BackendService<'a> for MaildirService<'a> {
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
        let mut envelopes = vec![];
        for mail_entry in self.maildir.list_new() {
            let mut entry = mail_entry?;
            let parsed_mail = entry.parsed().context(format!("cannot parse message"))?;
            let envelope =
                Envelope::try_from(parsed_mail).context(format!("cannot parse message"))?;
            envelopes.push(envelope);
        }
        Ok(envelopes.into())
    }

    fn get_msg(&mut self, seq: &str) -> Result<Msg> {
        let mut mail_entry = self
            .maildir
            .find(seq)
            .ok_or_else(|| anyhow!("cannot find message {:?}", seq))?;
        let parsed_mail = mail_entry
            .parsed()
            .context(format!("cannot parse message {:?}", seq))?;
        let msg = Msg::from_parsed_mail(parsed_mail)
            .context(format!("cannot parse message {:?}", seq))?;
        Ok(msg)
    }

    fn add_msg(&mut self, _: &Mbox, msg: &[u8], flags: Flags) -> Result<String> {
        self.maildir
            .store_cur_with_flags(msg, &to_maildir_flags(flags))
            .context("cannot add message to the \"cur\" folder")
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

impl<'a> MaildirService<'a> {
    pub fn new(
        account_config: &'a AccountConfig,
        maildir_config: &'a MaildirBackendConfig,
    ) -> Self {
        Self {
            account_config,
            maildir_config,
            maildir: maildir_config.maildir_dir.clone().into(),
        }
    }
}

fn to_maildir_flags(flags: Flags) -> String {
    let mut flags: Vec<_> = flags
        .iter()
        .map(|flag| match flag {
            Flag::Answered => "R",
            Flag::Deleted => "T",
            Flag::Draft => "D",
            Flag::Flagged => "F",
            Flag::Seen => "S",
            _ => "",
        })
        .collect();
    flags.sort();
    flags.join("")
}
