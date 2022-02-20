use anyhow::{anyhow, Context, Result};
use std::convert::{TryFrom, TryInto};

use crate::{
    backends::{
        maildir::msg_flag::Flags, Backend, MaildirEnvelopes, MaildirMboxes, RawMaildirEnvelopes,
    },
    config::MaildirBackendConfig,
    domain::Msg,
    mbox::Mboxes,
    msg::Envelopes,
};

pub struct MaildirBackend {
    maildir: maildir::Maildir,
    /// Holds raw mailboxes fetched by the `maildir` crate in order to
    /// extend mailboxes lifetime outside of handlers.
    _raw_envelopes_cache: Option<RawMaildirEnvelopes>,
}

impl<'a> MaildirBackend {
    pub fn new(maildir_config: &'a MaildirBackendConfig) -> Self {
        Self {
            maildir: maildir_config.maildir_dir.clone().into(),
            _raw_envelopes_cache: None,
        }
    }
}

impl<'a> Backend<'a> for MaildirBackend {
    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        let mboxes: MaildirMboxes = self.maildir.list_subdirs().try_into()?;
        Ok(Box::new(mboxes))
    }

    fn get_envelopes(
        &mut self,
        _mbox: &str,
        _sort: &str,
        filter: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let mail_entries = match filter {
            "new" => self.maildir.list_new(),
            _ => self.maildir.list_cur(),
        };
        let envelopes: MaildirEnvelopes = mail_entries
            .try_into()
            .context("cannot parse maildir envelopes")?;
        Ok(Box::new(envelopes))
    }

    fn add_msg(&mut self, _mbox: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>> {
        let flags: Flags = flags.try_into()?;
        let id = self
            .maildir
            .store_cur_with_flags(msg, &flags.to_string())
            .context("cannot add message to the \"cur\" folder")?;
        Ok(Box::new(id))
    }

    fn get_msg(&mut self, _mbox: &str, id: &str) -> Result<Msg> {
        let mut mail_entry = self
            .maildir
            .find(id)
            .ok_or_else(|| anyhow!("cannot find message {:?}", id))?;
        // TODO: parse flags
        let parsed_mail = mail_entry
            .parsed()
            .context(format!("cannot parse message {:?}", id))?;
        Msg::try_from(parsed_mail).context(format!("cannot parse message {:?}", id))
    }

    fn copy_msg(&mut self, _mbox_src: &str, _mbox_dest: &str, _id: &str) -> Result<()> {
        unimplemented!();
    }

    fn move_msg(&mut self, _mbox_src: &str, _mbox_dest: &str, _id: &str) -> Result<()> {
        unimplemented!();
    }

    fn del_msg(&mut self, _mbox: &str, id: &str) -> Result<()> {
        self.maildir
            .delete(id)
            .context(format!("cannot delete message {:?}", id))
    }

    fn add_flags(&mut self, _mbox: &str, id: &str, flags_str: &str) -> Result<()> {
        let flags: Flags = flags_str.try_into()?;
        self.maildir
            .add_flags(id, &flags.to_string())
            .context(format!(
                "cannot add flags {:?} to message {:?}",
                flags_str, id
            ))
    }

    fn set_flags(&mut self, _mbox: &str, id: &str, flags_str: &str) -> Result<()> {
        let flags: Flags = flags_str.try_into()?;
        self.maildir
            .set_flags(id, &flags.to_string())
            .context(format!(
                "cannot set flags {:?} to message {:?}",
                flags_str, id
            ))
    }

    fn del_flags(&mut self, _mbox: &str, id: &str, flags_str: &str) -> Result<()> {
        let flags: Flags = flags_str.try_into()?;
        self.maildir
            .remove_flags(id, &flags.to_string())
            .context(format!(
                "cannot remove flags {:?} to message {:?}",
                flags_str, id
            ))
    }
}
