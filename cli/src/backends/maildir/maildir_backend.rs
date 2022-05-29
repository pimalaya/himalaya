//! Maildir backend module.
//!
//! This module contains the definition of the maildir backend and its
//! traits implementation.

use anyhow::{anyhow, Context, Result};
use himalaya_lib::{
    account::{AccountConfig, MaildirBackendConfig},
    mbox::{Mbox, Mboxes},
};
use log::{debug, info, trace};
use std::{convert::TryInto, env, ffi::OsStr, fs, path::PathBuf};

use crate::{
    backends::{Backend, IdMapper, MaildirEnvelopes, MaildirFlags},
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
    pub fn get_mdir_from_dir(&self, dir: &str) -> Result<maildir::Maildir> {
        let dir = self.account_config.get_mbox_alias(dir)?;

        // If the dir points to the inbox folder, creates a maildir
        // instance from the root folder.
        if &dir == "inbox" {
            return self
                .validate_mdir_path(self.mdir.path().to_owned())
                .map(maildir::Maildir::from);
        }

        // If the dir is a valid maildir path, creates a maildir
        // instance from it. First checks for absolute path,
        self.validate_mdir_path((&dir).into())
            // then for relative path to `maildir-dir`,
            .or_else(|_| self.validate_mdir_path(self.mdir.path().join(&dir)))
            // and finally for relative path to the current directory.
            .or_else(|_| self.validate_mdir_path(env::current_dir()?.join(&dir)))
            .or_else(|_| {
                // Otherwise creates a maildir instance from a maildir
                // subdirectory by adding a "." in front of the name
                // as described in the [spec].
                //
                // [spec]: http://www.courier-mta.org/imap/README.maildirquota.html
                self.validate_mdir_path(self.mdir.path().join(format!(".{}", dir)))
            })
            .map(maildir::Maildir::from)
    }
}

impl<'a> Backend<'a> for MaildirBackend<'a> {
    fn add_mbox(&mut self, subdir: &str) -> Result<()> {
        info!(">> add maildir subdir");
        debug!("subdir: {:?}", subdir);

        let path = self.mdir.path().join(format!(".{}", subdir));
        trace!("subdir path: {:?}", path);

        fs::create_dir(&path)
            .with_context(|| format!("cannot create maildir subdir {:?} at {:?}", subdir, path))?;

        info!("<< add maildir subdir");
        Ok(())
    }

    fn get_mboxes(&mut self) -> Result<Mboxes> {
        trace!(">> get maildir mailboxes");

        let mut mboxes = Mboxes::default();
        for (name, desc) in &self.account_config.mailboxes {
            mboxes.push(Mbox {
                delim: String::from("/"),
                name: name.into(),
                desc: desc.into(),
            })
        }
        for entry in self.mdir.list_subdirs() {
            let dir = entry?;
            let dirname = dir.path().file_name();
            mboxes.push(Mbox {
                delim: String::from("/"),
                name: dirname
                    .and_then(OsStr::to_str)
                    .and_then(|s| if s.len() < 2 { None } else { Some(&s[1..]) })
                    .ok_or_else(|| {
                        anyhow!(
                            "cannot parse maildir subdirectory name from path {:?}",
                            dirname
                        )
                    })?
                    .into(),
                ..Mbox::default()
            });
        }

        trace!("maildir mailboxes: {:?}", mboxes);
        trace!("<< get maildir mailboxes");
        Ok(mboxes)
    }

    fn del_mbox(&mut self, dir: &str) -> Result<()> {
        info!(">> delete maildir dir");
        debug!("dir: {:?}", dir);

        let path = self.mdir.path().join(format!(".{}", dir));
        trace!("dir path: {:?}", path);

        fs::remove_dir_all(&path)
            .with_context(|| format!("cannot delete maildir {:?} from {:?}", dir, path))?;

        info!("<< delete maildir dir");
        Ok(())
    }

    fn get_envelopes(
        &mut self,
        dir: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        info!(">> get maildir envelopes");
        debug!("dir: {:?}", dir);
        debug!("page size: {:?}", page_size);
        debug!("page: {:?}", page);

        let mdir = self
            .get_mdir_from_dir(dir)
            .with_context(|| format!("cannot get maildir instance from {:?}", dir))?;

        // Reads envelopes from the "cur" folder of the selected
        // maildir.
        let mut envelopes: MaildirEnvelopes = mdir.list_cur().try_into().with_context(|| {
            format!("cannot parse maildir envelopes from {:?}", self.mdir.path())
        })?;
        debug!("envelopes len: {:?}", envelopes.len());
        trace!("envelopes: {:?}", envelopes);

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        debug!("page begin: {:?}", page_begin);
        if page_begin > envelopes.len() {
            return Err(anyhow!(
                "cannot get maildir envelopes at page {:?} (out of bounds)",
                page_begin + 1,
            ));
        }
        let page_end = envelopes.len().min(page_begin + page_size);
        debug!("page end: {:?}", page_end);

        // Sorts envelopes by most recent date.
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());

        // Applies pagination boundaries.
        envelopes.envelopes = envelopes[page_begin..page_end].to_owned();

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
        _dir: &str,
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

    fn add_msg(&mut self, dir: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>> {
        info!(">> add maildir message");
        debug!("dir: {:?}", dir);
        debug!("flags: {:?}", flags);

        let mdir = self
            .get_mdir_from_dir(dir)
            .with_context(|| format!("cannot get maildir instance from {:?}", dir))?;
        let flags: MaildirFlags = flags
            .try_into()
            .with_context(|| format!("cannot parse maildir flags {:?}", flags))?;
        let id = mdir
            .store_cur_with_flags(msg, &flags.to_string())
            .with_context(|| format!("cannot add maildir message to {:?}", mdir.path()))?;
        debug!("id: {:?}", id);
        let hash = format!("{:x}", md5::compute(&id));
        debug!("hash: {:?}", hash);

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir.path())
            .with_context(|| format!("cannot create id mapper instance for {:?}", mdir.path()))?;
        mapper
            .append(vec![(hash.clone(), id.clone())])
            .with_context(|| {
                format!(
                    "cannot append hash {:?} with id {:?} to id mapper",
                    hash, id
                )
            })?;

        info!("<< add maildir message");
        Ok(Box::new(hash))
    }

    fn get_msg(&mut self, dir: &str, short_hash: &str) -> Result<Msg> {
        info!(">> get maildir message");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);

        let mdir = self
            .get_mdir_from_dir(dir)
            .with_context(|| format!("cannot get maildir instance from {:?}", dir))?;
        let id = IdMapper::new(mdir.path())?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find maildir message by short hash {:?} at {:?}",
                    short_hash,
                    mdir.path()
                )
            })?;
        debug!("id: {:?}", id);
        let mut mail_entry = mdir.find(&id).ok_or_else(|| {
            anyhow!(
                "cannot find maildir message by id {:?} at {:?}",
                id,
                mdir.path()
            )
        })?;
        let parsed_mail = mail_entry.parsed().with_context(|| {
            format!("cannot parse maildir message {:?} at {:?}", id, mdir.path())
        })?;
        let msg = Msg::from_parsed_mail(parsed_mail, self.account_config).with_context(|| {
            format!("cannot parse maildir message {:?} at {:?}", id, mdir.path())
        })?;
        trace!("message: {:?}", msg);

        info!("<< get maildir message");
        Ok(msg)
    }

    fn copy_msg(&mut self, dir_src: &str, dir_dst: &str, short_hash: &str) -> Result<()> {
        info!(">> copy maildir message");
        debug!("source dir: {:?}", dir_src);
        debug!("destination dir: {:?}", dir_dst);

        let mdir_src = self
            .get_mdir_from_dir(dir_src)
            .with_context(|| format!("cannot get source maildir instance from {:?}", dir_src))?;
        let mdir_dst = self.get_mdir_from_dir(dir_dst).with_context(|| {
            format!("cannot get destination maildir instance from {:?}", dir_dst)
        })?;
        let id = IdMapper::new(mdir_src.path())
            .with_context(|| format!("cannot create id mapper instance for {:?}", mdir_src.path()))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find maildir message by short hash {:?} at {:?}",
                    short_hash,
                    mdir_src.path()
                )
            })?;
        debug!("id: {:?}", id);

        mdir_src.copy_to(&id, &mdir_dst).with_context(|| {
            format!(
                "cannot copy message {:?} from maildir {:?} to maildir {:?}",
                id,
                mdir_src.path(),
                mdir_dst.path()
            )
        })?;

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir_dst.path()).with_context(|| {
            format!("cannot create id mapper instance for {:?}", mdir_dst.path())
        })?;
        let hash = format!("{:x}", md5::compute(&id));
        mapper
            .append(vec![(hash.clone(), id.clone())])
            .with_context(|| {
                format!(
                    "cannot append hash {:?} with id {:?} to id mapper",
                    hash, id
                )
            })?;

        info!("<< copy maildir message");
        Ok(())
    }

    fn move_msg(&mut self, dir_src: &str, dir_dst: &str, short_hash: &str) -> Result<()> {
        info!(">> move maildir message");
        debug!("source dir: {:?}", dir_src);
        debug!("destination dir: {:?}", dir_dst);

        let mdir_src = self
            .get_mdir_from_dir(dir_src)
            .with_context(|| format!("cannot get source maildir instance from {:?}", dir_src))?;
        let mdir_dst = self.get_mdir_from_dir(dir_dst).with_context(|| {
            format!("cannot get destination maildir instance from {:?}", dir_dst)
        })?;
        let id = IdMapper::new(mdir_src.path())
            .with_context(|| format!("cannot create id mapper instance for {:?}", mdir_src.path()))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find maildir message by short hash {:?} at {:?}",
                    short_hash,
                    mdir_src.path()
                )
            })?;
        debug!("id: {:?}", id);

        mdir_src.move_to(&id, &mdir_dst).with_context(|| {
            format!(
                "cannot move message {:?} from maildir {:?} to maildir {:?}",
                id,
                mdir_src.path(),
                mdir_dst.path()
            )
        })?;

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir_dst.path()).with_context(|| {
            format!("cannot create id mapper instance for {:?}", mdir_dst.path())
        })?;
        let hash = format!("{:x}", md5::compute(&id));
        mapper
            .append(vec![(hash.clone(), id.clone())])
            .with_context(|| {
                format!(
                    "cannot append hash {:?} with id {:?} to id mapper",
                    hash, id
                )
            })?;

        info!("<< move maildir message");
        Ok(())
    }

    fn del_msg(&mut self, dir: &str, short_hash: &str) -> Result<()> {
        info!(">> delete maildir message");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);

        let mdir = self
            .get_mdir_from_dir(dir)
            .with_context(|| format!("cannot get maildir instance from {:?}", dir))?;
        let id = IdMapper::new(mdir.path())
            .with_context(|| format!("cannot create id mapper instance for {:?}", mdir.path()))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find maildir message by short hash {:?} at {:?}",
                    short_hash,
                    mdir.path()
                )
            })?;
        debug!("id: {:?}", id);
        mdir.delete(&id).with_context(|| {
            format!(
                "cannot delete message {:?} from maildir {:?}",
                id,
                mdir.path()
            )
        })?;

        info!("<< delete maildir message");
        Ok(())
    }

    fn add_flags(&mut self, dir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> add maildir message flags");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);
        debug!("flags: {:?}", flags);

        let mdir = self
            .get_mdir_from_dir(dir)
            .with_context(|| format!("cannot get maildir instance from {:?}", dir))?;
        let flags: MaildirFlags = flags
            .try_into()
            .with_context(|| format!("cannot parse maildir flags {:?}", flags))?;
        debug!("flags: {:?}", flags);
        let id = IdMapper::new(mdir.path())
            .with_context(|| format!("cannot create id mapper instance for {:?}", mdir.path()))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find maildir message by short hash {:?} at {:?}",
                    short_hash,
                    mdir.path()
                )
            })?;
        debug!("id: {:?}", id);
        mdir.add_flags(&id, &flags.to_string())
            .with_context(|| format!("cannot add flags {:?} to maildir message {:?}", flags, id))?;

        info!("<< add maildir message flags");
        Ok(())
    }

    fn set_flags(&mut self, dir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> set maildir message flags");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);
        debug!("flags: {:?}", flags);

        let mdir = self
            .get_mdir_from_dir(dir)
            .with_context(|| format!("cannot get maildir instance from {:?}", dir))?;
        let flags: MaildirFlags = flags
            .try_into()
            .with_context(|| format!("cannot parse maildir flags {:?}", flags))?;
        debug!("flags: {:?}", flags);
        let id = IdMapper::new(mdir.path())
            .with_context(|| format!("cannot create id mapper instance for {:?}", mdir.path()))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find maildir message by short hash {:?} at {:?}",
                    short_hash,
                    mdir.path()
                )
            })?;
        debug!("id: {:?}", id);
        mdir.set_flags(&id, &flags.to_string())
            .with_context(|| format!("cannot set flags {:?} to maildir message {:?}", flags, id))?;

        info!("<< set maildir message flags");
        Ok(())
    }

    fn del_flags(&mut self, dir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> delete maildir message flags");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);
        debug!("flags: {:?}", flags);

        let mdir = self
            .get_mdir_from_dir(dir)
            .with_context(|| format!("cannot get maildir instance from {:?}", dir))?;
        let flags: MaildirFlags = flags
            .try_into()
            .with_context(|| format!("cannot parse maildir flags {:?}", flags))?;
        debug!("flags: {:?}", flags);
        let id = IdMapper::new(mdir.path())
            .with_context(|| format!("cannot create id mapper instance for {:?}", mdir.path()))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find maildir message by short hash {:?} at {:?}",
                    short_hash,
                    mdir.path()
                )
            })?;
        debug!("id: {:?}", id);
        mdir.remove_flags(&id, &flags.to_string())
            .with_context(|| {
                format!(
                    "cannot delete flags {:?} to maildir message {:?}",
                    flags, id
                )
            })?;

        info!("<< delete maildir message flags");
        Ok(())
    }
}
