use anyhow::{anyhow, Context, Result};
use std::convert::{TryFrom, TryInto};

use crate::{
    backends::{maildir::Flags, Backend},
    config::MaildirBackendConfig,
    domain::{Envelope, Envelopes, Msg},
    Mboxes,
};

pub struct MaildirBackend {
    maildir: maildir::Maildir,
}

impl<'a> Backend<'a> for MaildirBackend {
    fn get_mboxes(&mut self) -> Result<Mboxes> {
        unimplemented!()
    }

    fn get_envelopes(
        &mut self,
        _mbox: &str,
        _sort: &str,
        filter: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Envelopes> {
        let mut envelopes = vec![];

        let mail_entries = match filter {
            "new" => self.maildir.list_new(),
            _ => self.maildir.list_cur(),
        };

        for mail_entry in mail_entries {
            let mut parsed_mail = mail_entry?;
            let parsed_mail = parsed_mail
                .parsed()
                .context(format!("cannot parse message"))?;
            let envelope =
                Envelope::try_from(parsed_mail).context(format!("cannot parse message"))?;
            envelopes.push(envelope);
        }

        Ok(envelopes.into())
    }

    fn add_msg(&mut self, _mbox: &str, msg: &[u8], flags: &str) -> Result<String> {
        let flags: Flags = flags.try_into()?;
        self.maildir
            .store_cur_with_flags(msg, &flags.to_string())
            .context("cannot add message to the \"cur\" folder")
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

impl<'a> MaildirBackend {
    pub fn new(maildir_config: &'a MaildirBackendConfig) -> Self {
        Self {
            maildir: maildir_config.maildir_dir.clone().into(),
        }
    }
}
