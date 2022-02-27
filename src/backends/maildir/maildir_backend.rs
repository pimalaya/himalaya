use anyhow::{anyhow, Context, Result};
use std::{convert::TryInto, fs, path::PathBuf};

use crate::{
    backends::{Backend, IdMapper, MaildirEnvelopes, MaildirFlags, MaildirMboxes},
    config::{AccountConfig, MaildirBackendConfig, DEFAULT_INBOX_FOLDER},
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

    fn validate_mdir_path(&self, mdir_path: PathBuf) -> Result<PathBuf> {
        if mdir_path.is_dir() {
            Ok(mdir_path)
        } else {
            Err(anyhow!(
                "cannot read maildir from directory {:?}",
                mdir_path
            ))
        }
    }

    fn get_mdir_from_name(&self, mdir: &str) -> Result<maildir::Maildir> {
        let inbox_folder = self
            .account_config
            .mailboxes
            .get("inbox")
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_INBOX_FOLDER);

        if mdir == inbox_folder {
            self.validate_mdir_path(self.mdir.path().to_owned())
                .map(maildir::Maildir::from)
        } else {
            self.validate_mdir_path(mdir.into())
                .or_else(|_| {
                    let path = self.mdir.path().join(format!(".{}", mdir));
                    self.validate_mdir_path(path)
                })
                .map(maildir::Maildir::from)
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
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let mdir = self.get_mdir_from_name(mdir)?;

        // Reads envelopes from the "cur" folder of the selected
        // maildir.
        let mut envelopes: MaildirEnvelopes = mdir.list_cur().try_into().context(format!(
            "cannot parse maildir envelopes from {:?}",
            self.mdir.path()
        ))?;

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        if page_begin > envelopes.len() {
            return Err(anyhow!(format!(
                "cannot list maildir envelopes at page {:?} (out of bounds)",
                page_begin + 1,
            )));
        }
        let page_end = envelopes.len().min(page_begin + page_size);

        // Sorts envelopes by most recent date.
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());

        // Applies pagination boundaries.
        envelopes.0 = envelopes[page_begin..page_end].to_owned();

        // Writes envelope ids and their hashes to a cache file. The
        // cache file name is based on the name of the given maildir:
        // this way there is one cache per maildir.
        let short_hash_len = {
            let mut mapper = IdMapper::new(mdir.path())?;
            let entries = envelopes
                .iter()
                .map(|env| (env.hash.to_owned(), env.id.to_owned()))
                .collect();
            mapper.append(entries)?
        };

        // Shorten envelopes hash.
        envelopes
            .iter_mut()
            .for_each(|env| env.hash = env.hash[0..short_hash_len].to_owned());

        Ok(Box::new(envelopes))
    }

    fn search_envelopes(
        &mut self,
        _mdir: &str,
        _query: &str,
        _sort: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        Err(anyhow!(
            "cannot find maildir envelopes: feature not implemented"
        ))
    }

    fn add_msg(&mut self, mdir: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let flags: MaildirFlags = flags.try_into()?;
        let id = mdir
            .store_cur_with_flags(msg, &flags.to_string())
            .context(format!("cannot add maildir message to {:?}", mdir.path()))?;
        let hash = format!("{:x}", md5::compute(&id));

        // Appends hash line to the maildir cache file.
        let mut mapper = IdMapper::new(mdir.path())?;
        mapper.append(vec![(hash.clone(), id)])?;

        Ok(Box::new(hash))
    }

    fn get_msg(&mut self, mdir: &str, short_hash: &str) -> Result<Msg> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let id = IdMapper::new(mdir.path())?
            .find(short_hash)
            .context(format!(
                "cannot get maildir message from short hash {:?}",
                short_hash
            ))?;
        let mut mail_entry = mdir
            .find(&id)
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

    fn copy_msg(&mut self, mdir_src: &str, mdir_dst: &str, short_hash: &str) -> Result<()> {
        let mdir_src = self.get_mdir_from_name(mdir_src)?;
        let mdir_dst = self.get_mdir_from_name(mdir_dst)?;
        let id = IdMapper::new(mdir_src.path())?.find(short_hash)?;

        mdir_src.copy_to(&id, &mdir_dst).context(format!(
            "cannot copy message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))?;

        // Appends hash line to the destination maildir cache file.
        IdMapper::new(mdir_dst.path())?.append(vec![(format!("{:x}", md5::compute(&id)), id)])?;

        Ok(())
    }

    fn move_msg(&mut self, mdir_src: &str, mdir_dst: &str, short_hash: &str) -> Result<()> {
        let mdir_src = self.get_mdir_from_name(mdir_src)?;
        let mdir_dst = self.get_mdir_from_name(mdir_dst)?;
        let id = IdMapper::new(mdir_src.path())?.find(short_hash)?;

        mdir_src.move_to(&id, &mdir_dst).context(format!(
            "cannot move message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))?;

        // Appends hash line to the destination maildir cache file.
        IdMapper::new(mdir_dst.path())?.append(vec![(format!("{:x}", md5::compute(&id)), id)])?;

        Ok(())
    }

    fn del_msg(&mut self, mdir: &str, short_hash: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        mdir.delete(&id).context(format!(
            "cannot delete message {:?} from maildir {:?}",
            id,
            mdir.path()
        ))
    }

    fn add_flags(&mut self, mdir: &str, short_hash: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.add_flags(&id, &flags.to_string()).context(format!(
            "cannot add flags {:?} to maildir message {:?}",
            flags_str, id
        ))
    }

    fn set_flags(&mut self, mdir: &str, short_hash: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.set_flags(&id, &flags.to_string()).context(format!(
            "cannot set flags {:?} to maildir message {:?}",
            flags_str, id
        ))
    }

    fn del_flags(&mut self, mdir: &str, short_hash: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.remove_flags(&id, &flags.to_string()).context(format!(
            "cannot remove flags {:?} from maildir message {:?}",
            flags_str, id
        ))
    }
}
