use anyhow::{anyhow, Context, Result};
use log::{debug, info, trace};
use std::{convert::TryInto, fs, path::PathBuf};

use crate::{
    backends::{Backend, IdMapper, MaildirEnvelopes, MaildirFlags, MaildirMboxes},
    config::{AccountConfig, MaildirBackendConfig, DEFAULT_INBOX_FOLDER},
    mbox::Mboxes,
    msg::{Envelopes, Msg},
};

/// Represents the maildir backend.
pub struct MaildirBackend<'a> {
    account_config: &'a AccountConfig,
    mdir: maildir::Maildir,
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
            Err(anyhow!("cannot read maildir directory {:?}", mdir_path))
        }
    }

    /// Creates a maildir instance from a string slice.
    fn get_mdir_from_dir(&self, dir: &str) -> Result<maildir::Maildir> {
        let inbox_folder = self
            .account_config
            .mailboxes
            .get("inbox")
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_INBOX_FOLDER);

        // If the dir points to the inbox folder, creates a maildir
        // instance from the root folder.
        if dir == inbox_folder {
            self.validate_mdir_path(self.mdir.path().to_owned())
                .map(maildir::Maildir::from)
        } else {
            // If the dir is a valid maildir path, creates a maildir instance from it.
            self.validate_mdir_path(dir.into())
                .or_else(|_| {
                    // Otherwise creates a maildir instance from a
                    // maildir subdirectory by adding a "." in front
                    // of the name as described in the spec:
                    // https://cr.yp.to/proto/maildir.html
                    let path = self.mdir.path().join(format!(".{}", dir));
                    self.validate_mdir_path(path)
                })
                .map(maildir::Maildir::from)
        }
    }
}

impl<'a> Backend<'a> for MaildirBackend<'a> {
    fn add_mbox(&mut self, subdir: &str) -> Result<()> {
        info!(">> add maildir subdir");
        debug!("subdir: {:?}", subdir);

        let path = self.mdir.path().join(format!(".{}", subdir));
        trace!("subdir path: {:?}", path);

        fs::create_dir(&path).context(format!(
            "cannot create maildir subdir {:?} at {:?}",
            subdir, path
        ))?;

        info!("<< add maildir subdir");
        Ok(())
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        info!(">> get maildir subdirs");

        let subdirs: MaildirMboxes = self.mdir.list_subdirs().try_into().context(format!(
            "cannot parse maildir subdirs from {:?}",
            self.mdir.path()
        ))?;
        trace!("subdirs: {:?}", subdirs);

        info!("<< get maildir subdirs");
        Ok(Box::new(subdirs))
    }

    fn del_mbox(&mut self, subdir: &str) -> Result<()> {
        info!(">> delete maildir subdir");
        debug!("subdir: {:?}", subdir);

        let path = self.mdir.path().join(format!(".{}", subdir));
        trace!("subdir path: {:?}", path);

        fs::remove_dir_all(&path).context(format!(
            "cannot delete maildir subdir {:?} from {:?}",
            subdir, path
        ))?;

        info!("<< delete maildir subdir");
        Ok(())
    }

    fn get_envelopes(
        &mut self,
        subdir: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        info!(">> get maildir envelopes");
        debug!("maildir subdir: {:?}", subdir);
        debug!("page size: {:?}", page_size);
        debug!("page: {:?}", page);

        let mdir = self.get_mdir_from_dir(subdir).context(format!(
            "cannot get maildir instance from subdir {:?}",
            subdir
        ))?;

        // Reads envelopes from the "cur" folder of the selected
        // maildir.
        let mut envelopes: MaildirEnvelopes = mdir.list_cur().try_into().context(format!(
            "cannot parse maildir envelopes from {:?}",
            self.mdir.path()
        ))?;
        debug!("envelopes len: {:?}", envelopes.len());
        trace!("envelopes: {:?}", envelopes);

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        debug!("page begin: {:?}", page_begin);
        if page_begin > envelopes.len() {
            return Err(anyhow!(format!(
                "cannot get maildir envelopes at page {:?} (out of bounds)",
                page_begin + 1,
            )));
        }
        let page_end = envelopes.len().min(page_begin + page_size);
        debug!("page end: {:?}", page_end);

        // Sorts envelopes by most recent date.
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());

        // Applies pagination boundaries.
        envelopes.0 = envelopes[page_begin..page_end].to_owned();

        // Appends envelopes hash to the id mapper cache file and
        // calculates the new short hash length. The short hash length
        // represents the minimum hash length possible to avoid
        // conflicts.
        let short_hash_len = {
            let mut mapper = IdMapper::new(mdir.path())?;
            let entries = envelopes
                .iter()
                .map(|env| (env.hash.to_owned(), env.id.to_owned()))
                .collect();
            mapper.append(entries)?
        };
        debug!("short hash length: {:?}", short_hash_len);

        // Shorten envelopes hash.
        envelopes
            .iter_mut()
            .for_each(|env| env.hash = env.hash[0..short_hash_len].to_owned());

        info!("<< get maildir envelopes");
        Ok(Box::new(envelopes))
    }

    fn search_envelopes(
        &mut self,
        _subdir: &str,
        _query: &str,
        _sort: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        info!(">> search maildir envelopes");
        info!("<< search maildir envelopes");
        Err(anyhow!(
            "cannot find maildir envelopes: feature not implemented"
        ))
    }

    fn add_msg(&mut self, subdir: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>> {
        info!(">> add maildir message");
        debug!("subdir: {:?}", subdir);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(subdir).context(format!(
            "cannot get maildir instance from subdir {:?}",
            subdir
        ))?;
        let flags: MaildirFlags = flags
            .try_into()
            .context(format!("cannot parse flags {:?}", flags))?;
        let id = mdir
            .store_cur_with_flags(msg, &flags.to_string())
            .context(format!("cannot add maildir message to {:?}", mdir.path()))?;
        debug!("id: {:?}", id);
        let hash = format!("{:x}", md5::compute(&id));
        debug!("hash: {:?}", hash);

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir.path()).context(format!(
            "cannot create id mapper instance for {:?}",
            mdir.path()
        ))?;
        mapper
            .append(vec![(hash.clone(), id.clone())])
            .context(format!(
                "cannot append hash {:?} with id {:?} to id mapper",
                hash, id
            ))?;

        info!("<< add maildir message");
        Ok(Box::new(hash))
    }

    fn get_msg(&mut self, subdir: &str, short_hash: &str) -> Result<Msg> {
        info!(">> get maildir message");
        debug!("subdir: {:?}", subdir);
        debug!("short hash: {:?}", short_hash);

        let mdir = self.get_mdir_from_dir(subdir).context(format!(
            "cannot get maildir instance from subdir {:?}",
            subdir
        ))?;
        let id = IdMapper::new(mdir.path())?
            .find(short_hash)
            .context(format!(
                "cannot find maildir message by short hash {:?} at {:?}",
                short_hash,
                mdir.path()
            ))?;
        debug!("id: {:?}", id);
        let mut mail_entry = mdir.find(&id).ok_or_else(|| {
            anyhow!(
                "cannot find maildir message by id {:?} at {:?}",
                id,
                mdir.path()
            )
        })?;
        let parsed_mail = mail_entry.parsed().context(format!(
            "cannot parse maildir message {:?} at {:?}",
            id,
            mdir.path()
        ))?;
        let msg = Msg::from_parsed_mail(parsed_mail, self.account_config).context(format!(
            "cannot parse maildir message {:?} at {:?}",
            id,
            mdir.path()
        ))?;
        trace!("message: {:?}", msg);

        info!("<< get maildir message");
        Ok(msg)
    }

    fn copy_msg(&mut self, subdir_src: &str, subdir_dst: &str, short_hash: &str) -> Result<()> {
        info!(">> copy maildir message");
        debug!("source subdir: {:?}", subdir_src);
        debug!("destination subdir: {:?}", subdir_dst);

        let mdir_src = self.get_mdir_from_dir(subdir_src).context(format!(
            "cannot get source maildir instance from subdir {:?}",
            subdir_src
        ))?;
        let mdir_dst = self.get_mdir_from_dir(subdir_dst).context(format!(
            "cannot get destination maildir instance from subdir {:?}",
            subdir_dst
        ))?;
        let id = IdMapper::new(mdir_src.path())
            .context(format!(
                "cannot create id mapper instance for {:?}",
                mdir_src.path()
            ))?
            .find(short_hash)
            .context(format!(
                "cannot find maildir message by short hash {:?} at {:?}",
                short_hash,
                mdir_src.path()
            ))?;
        debug!("id: {:?}", id);

        mdir_src.copy_to(&id, &mdir_dst).context(format!(
            "cannot copy message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))?;

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir_dst.path()).context(format!(
            "cannot create id mapper instance for {:?}",
            mdir_dst.path()
        ))?;
        let hash = format!("{:x}", md5::compute(&id));
        mapper
            .append(vec![(hash.clone(), id.clone())])
            .context(format!(
                "cannot append hash {:?} with id {:?} to id mapper",
                hash, id
            ))?;

        info!("<< copy maildir message");
        Ok(())
    }

    fn move_msg(&mut self, subdir_src: &str, subdir_dst: &str, short_hash: &str) -> Result<()> {
        info!(">> move maildir message");
        debug!("source subdir: {:?}", subdir_src);
        debug!("destination subdir: {:?}", subdir_dst);

        let mdir_src = self.get_mdir_from_dir(subdir_src).context(format!(
            "cannot get source maildir instance from subdir {:?}",
            subdir_src
        ))?;
        let mdir_dst = self.get_mdir_from_dir(subdir_dst).context(format!(
            "cannot get destination maildir instance from subdir {:?}",
            subdir_dst
        ))?;
        let id = IdMapper::new(mdir_src.path())
            .context(format!(
                "cannot create id mapper instance for {:?}",
                mdir_src.path()
            ))?
            .find(short_hash)
            .context(format!(
                "cannot find maildir message by short hash {:?} at {:?}",
                short_hash,
                mdir_src.path()
            ))?;
        debug!("id: {:?}", id);

        mdir_src.move_to(&id, &mdir_dst).context(format!(
            "cannot move message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))?;

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir_dst.path()).context(format!(
            "cannot create id mapper instance for {:?}",
            mdir_dst.path()
        ))?;
        let hash = format!("{:x}", md5::compute(&id));
        mapper
            .append(vec![(hash.clone(), id.clone())])
            .context(format!(
                "cannot append hash {:?} with id {:?} to id mapper",
                hash, id
            ))?;

        info!("<< move maildir message");
        Ok(())
    }

    fn del_msg(&mut self, subdir: &str, short_hash: &str) -> Result<()> {
        info!(">> delete maildir message");
        debug!("subdir: {:?}", subdir);
        debug!("short hash: {:?}", short_hash);

        let mdir = self.get_mdir_from_dir(subdir).context(format!(
            "cannot get maildir instance from subdir {:?}",
            subdir
        ))?;
        let id = IdMapper::new(mdir.path())
            .context(format!(
                "cannot create id mapper instance for {:?}",
                mdir.path()
            ))?
            .find(short_hash)
            .context(format!(
                "cannot find maildir message by short hash {:?} at {:?}",
                short_hash,
                mdir.path()
            ))?;
        debug!("id: {:?}", id);
        mdir.delete(&id).context(format!(
            "cannot delete message {:?} from maildir {:?}",
            id,
            mdir.path()
        ))?;

        info!("<< delete maildir message");
        Ok(())
    }

    fn add_flags(&mut self, subdir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> add maildir message flags");
        debug!("subdir: {:?}", subdir);
        debug!("short hash: {:?}", short_hash);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(subdir).context(format!(
            "cannot get maildir instance from subdir {:?}",
            subdir
        ))?;
        let flags: MaildirFlags = flags
            .try_into()
            .context(format!("cannot parse maildir flags {:?}", flags))?;
        debug!("flags: {:?}", flags);
        let id = IdMapper::new(mdir.path())
            .context(format!(
                "cannot create id mapper instance for {:?}",
                mdir.path()
            ))?
            .find(short_hash)
            .context(format!(
                "cannot find maildir message by short hash {:?} at {:?}",
                short_hash,
                mdir.path()
            ))?;
        debug!("id: {:?}", id);
        mdir.add_flags(&id, &flags.to_string()).context(format!(
            "cannot add flags {:?} to maildir message {:?}",
            flags, id
        ))?;

        info!("<< add maildir message flags");
        Ok(())
    }

    fn set_flags(&mut self, subdir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> set maildir message flags");
        debug!("subdir: {:?}", subdir);
        debug!("short hash: {:?}", short_hash);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(subdir).context(format!(
            "cannot get maildir instance from subdir {:?}",
            subdir
        ))?;
        let flags: MaildirFlags = flags
            .try_into()
            .context(format!("cannot parse maildir flags {:?}", flags))?;
        debug!("flags: {:?}", flags);
        let id = IdMapper::new(mdir.path())
            .context(format!(
                "cannot create id mapper instance for {:?}",
                mdir.path()
            ))?
            .find(short_hash)
            .context(format!(
                "cannot find maildir message by short hash {:?} at {:?}",
                short_hash,
                mdir.path()
            ))?;
        debug!("id: {:?}", id);
        mdir.set_flags(&id, &flags.to_string()).context(format!(
            "cannot set flags {:?} to maildir message {:?}",
            flags, id
        ))?;

        info!("<< set maildir message flags");
        Ok(())
    }

    fn del_flags(&mut self, subdir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> delete maildir message flags");
        debug!("subdir: {:?}", subdir);
        debug!("short hash: {:?}", short_hash);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(subdir).context(format!(
            "cannot get maildir instance from subdir {:?}",
            subdir
        ))?;
        let flags: MaildirFlags = flags
            .try_into()
            .context(format!("cannot parse maildir flags {:?}", flags))?;
        debug!("flags: {:?}", flags);
        let id = IdMapper::new(mdir.path())
            .context(format!(
                "cannot create id mapper instance for {:?}",
                mdir.path()
            ))?
            .find(short_hash)
            .context(format!(
                "cannot find maildir message by short hash {:?} at {:?}",
                short_hash,
                mdir.path()
            ))?;
        debug!("id: {:?}", id);
        mdir.remove_flags(&id, &flags.to_string()).context(format!(
            "cannot delete flags {:?} to maildir message {:?}",
            flags, id
        ))?;

        info!("<< delete maildir message flags");
        Ok(())
    }
}
