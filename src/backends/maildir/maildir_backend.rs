use anyhow::{anyhow, Context, Result};
use std::{convert::TryInto, fs};

use crate::{
    backends::{Backend, MaildirEnvelopes, MaildirFlags, MaildirMboxes},
    config::{AccountConfig, MaildirBackendConfig},
    mbox::Mboxes,
    msg::{Envelopes, Msg},
};

pub struct MaildirBackend<'a> {
    mdir: maildir::Maildir,
    account_config: &'a AccountConfig,
}

impl<'a> MaildirBackend<'a> {
    pub fn new(
        account_config: &'a AccountConfig,
        maildir_config: &'a MaildirBackendConfig,
    ) -> Self {
        Self {
            account_config,
            mdir: maildir_config.maildir_dir.clone().into(),
        }
    }

    fn get_mdir_from_name(&self, mdir: &str) -> maildir::Maildir {
        if mdir == self.account_config.inbox_folder {
            self.mdir.path().to_owned().into()
        } else {
            self.mdir.path().join(format!(".{}", mdir)).into()
        }
    }
}

impl<'a> Backend<'a> for MaildirBackend<'a> {
    fn add_mbox(&mut self, mdir: &str) -> Result<()> {
        fs::create_dir(self.mdir.path().join(format!(".{}", mdir)))
            .context(format!("cannot create maildir subfolder {:?}", mdir))
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        let mboxes: MaildirMboxes = self.mdir.list_subdirs().try_into()?;
        Ok(Box::new(mboxes))
    }

    fn del_mbox(&mut self, mdir: &str) -> Result<()> {
        fs::remove_dir_all(self.mdir.path().join(format!(".{}", mdir)))
            .context(format!("cannot delete maildir subfolder {:?}", mdir))
    }

    fn get_envelopes(
        &mut self,
        mdir: &str,
        _sort: &str,
        filter: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let mdir = self.get_mdir_from_name(mdir);
        let mail_entries = match filter {
            "new" => mdir.list_new(),
            _ => mdir.list_cur(),
        };
        let mut envelopes: MaildirEnvelopes = mail_entries
            .try_into()
            .context("cannot parse maildir envelopes from {:?}")?;
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());

        let page_begin = page * page_size;
        let page_end = page_begin + page_size;
        if page_end > envelopes.len() {
            return Err(anyhow!(format!(
                "cannot list maildir envelopes at page {:?} with a page size at {:?} (out of bounds)",
                page_begin + 1,
		page_size,
            )));
        }
        envelopes.0 = envelopes[page_begin..page_end].to_owned();
        Ok(Box::new(envelopes))
    }

    fn add_msg(&mut self, mdir: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>> {
        let mdir = self.get_mdir_from_name(mdir);
        let flags: MaildirFlags = flags.try_into()?;
        let id = mdir
            .store_cur_with_flags(msg, &flags.to_string())
            .context(format!(
                "cannot add message to the \"cur\" folder of maildir {:?}",
                mdir.path()
            ))?;
        Ok(Box::new(id))
    }

    fn get_msg(&mut self, mdir: &str, id: &str) -> Result<Msg> {
        let mdir = self.get_mdir_from_name(mdir);
        let mut mail_entry = mdir
            .find(id)
            .ok_or_else(|| anyhow!("cannot find maildir message {:?} in {:?}", id, mdir.path()))?;
        let parsed_mail = mail_entry.parsed().context(format!(
            "cannot parse maildir message {:?} in {:?}",
            id,
            mdir.path()
        ))?;
        Msg::from_parsed_mail(parsed_mail, self.account_config).context(format!(
            "cannot parse maildir message {:?} from {:?}",
            id,
            mdir.path()
        ))
    }

    fn copy_msg(&mut self, mdir_src: &str, mdir_dst: &str, id: &str) -> Result<()> {
        let mdir_src = self.get_mdir_from_name(mdir_src);
        let mdir_dst = self.get_mdir_from_name(mdir_dst);
        mdir_src.copy_to(id, &mdir_dst).context(format!(
            "cannot copy message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))
    }

    fn move_msg(&mut self, mdir_src: &str, mdir_dst: &str, id: &str) -> Result<()> {
        let mdir_src = self.get_mdir_from_name(mdir_src);
        let mdir_dst = self.get_mdir_from_name(mdir_dst);
        mdir_src.move_to(id, &mdir_dst).context(format!(
            "cannot move message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))
    }

    fn del_msg(&mut self, mdir: &str, id: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir);
        mdir.delete(id).context(format!(
            "cannot delete message {:?} from maildir {:?}",
            id,
            mdir.path()
        ))
    }

    fn add_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir);
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.add_flags(id, &flags.to_string()).context(format!(
            "cannot add flags {:?} to maildir message {:?}",
            flags_str, id
        ))
    }

    fn set_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir);
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.set_flags(id, &flags.to_string()).context(format!(
            "cannot set flags {:?} to maildir message {:?}",
            flags_str, id
        ))
    }

    fn del_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir);
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.remove_flags(id, &flags.to_string()).context(format!(
            "cannot remove flags {:?} from maildir message {:?}",
            flags_str, id
        ))
    }
}
