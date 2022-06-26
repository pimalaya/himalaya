//! Maildir backend module.
//!
//! This module contains the definition of the maildir backend and its
//! traits implementation.

use log::{debug, info, trace};
use std::{env, ffi::OsStr, fs, path::PathBuf};

use crate::{
    account::{Account, MaildirBackendConfig},
    backend::{backend::Result, maildir_envelopes, maildir_flags, Backend, IdMapper},
    mbox::{Mbox, Mboxes},
    msg::{Envelopes, Flags, Msg},
};

use super::MaildirError;

/// Represents the maildir backend.
pub struct MaildirBackend<'a> {
    account_config: &'a Account,
    mdir: maildir::Maildir,
}

impl<'a> MaildirBackend<'a> {
    pub fn new(account_config: &'a Account, maildir_config: &'a MaildirBackendConfig) -> Self {
        Self {
            account_config,
            mdir: maildir_config.maildir_dir.clone().into(),
        }
    }

    fn validate_mdir_path(&self, mdir_path: PathBuf) -> Result<PathBuf> {
        let path = if mdir_path.is_dir() {
            Ok(mdir_path)
        } else {
            Err(MaildirError::ReadDirError(mdir_path.to_owned()))
        }?;
        Ok(path)
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
            .or_else(|_| {
                self.validate_mdir_path(
                    env::current_dir()
                        .map_err(MaildirError::GetCurrentDirError)?
                        .join(&dir),
                )
            })
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
            .map_err(|err| MaildirError::CreateSubdirError(err, subdir.to_owned()))?;

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
            let dir = entry.map_err(MaildirError::DecodeSubdirError)?;
            let dirname = dir.path().file_name();
            mboxes.push(Mbox {
                delim: String::from("/"),
                name: dirname
                    .and_then(OsStr::to_str)
                    .and_then(|s| if s.len() < 2 { None } else { Some(&s[1..]) })
                    .ok_or_else(|| MaildirError::ParseSubdirError(dir.path().to_owned()))?
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
            .map_err(|err| MaildirError::DeleteAllDirError(err, path.to_owned()))?;

        info!("<< delete maildir dir");
        Ok(())
    }

    fn get_envelopes(&mut self, dir: &str, page_size: usize, page: usize) -> Result<Envelopes> {
        info!(">> get maildir envelopes");
        debug!("dir: {:?}", dir);
        debug!("page size: {:?}", page_size);
        debug!("page: {:?}", page);

        let mdir = self.get_mdir_from_dir(dir)?;

        // Reads envelopes from the "cur" folder of the selected
        // maildir.
        let mut envelopes = maildir_envelopes::from_maildir_entries(mdir.list_cur())?;
        debug!("envelopes len: {:?}", envelopes.len());
        trace!("envelopes: {:?}", envelopes);

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        debug!("page begin: {:?}", page_begin);
        if page_begin > envelopes.len() {
            return Err(MaildirError::GetEnvelopesOutOfBoundsError(page_begin + 1))?;
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
                .map(|env| (env.id.to_owned(), env.internal_id.to_owned()))
                .collect();
            mapper.append(entries)?
        };
        debug!("short hash length: {:?}", short_hash_len);

        // Shorten envelopes hash.
        envelopes
            .iter_mut()
            .for_each(|env| env.id = env.id[0..short_hash_len].to_owned());

        info!("<< get maildir envelopes");
        Ok(envelopes)
    }

    fn search_envelopes(
        &mut self,
        _dir: &str,
        _query: &str,
        _sort: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Envelopes> {
        info!(">> search maildir envelopes");
        info!("<< search maildir envelopes");
        Err(MaildirError::SearchEnvelopesUnimplementedError)?
    }

    fn add_msg(&mut self, dir: &str, msg: &[u8], flags: &str) -> Result<String> {
        info!(">> add maildir message");
        debug!("dir: {:?}", dir);
        debug!("flags: {:?}", flags);

        let flags = Flags::from(flags);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(dir)?;
        let id = mdir
            .store_cur_with_flags(msg, &maildir_flags::to_normalized_string(&flags))
            .map_err(MaildirError::StoreWithFlagsError)?;
        debug!("id: {:?}", id);
        let hash = format!("{:x}", md5::compute(&id));
        debug!("hash: {:?}", hash);

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir.path())?;
        mapper.append(vec![(hash.clone(), id.clone())])?;

        info!("<< add maildir message");
        Ok(hash)
    }

    fn get_msg(&mut self, dir: &str, short_hash: &str) -> Result<Msg> {
        info!(">> get maildir message");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);

        let mdir = self.get_mdir_from_dir(dir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        debug!("id: {:?}", id);
        let mut mail_entry = mdir
            .find(&id)
            .ok_or_else(|| MaildirError::GetMsgError(id.to_owned()))?;
        let parsed_mail = mail_entry.parsed().map_err(MaildirError::ParseMsgError)?;
        let msg = Msg::from_parsed_mail(parsed_mail, self.account_config)?;
        trace!("message: {:?}", msg);

        info!("<< get maildir message");
        Ok(msg)
    }

    fn copy_msg(&mut self, dir_src: &str, dir_dst: &str, short_hash: &str) -> Result<()> {
        info!(">> copy maildir message");
        debug!("source dir: {:?}", dir_src);
        debug!("destination dir: {:?}", dir_dst);

        let mdir_src = self.get_mdir_from_dir(dir_src)?;
        let mdir_dst = self.get_mdir_from_dir(dir_dst)?;
        let id = IdMapper::new(mdir_src.path())?.find(short_hash)?;
        debug!("id: {:?}", id);

        mdir_src
            .copy_to(&id, &mdir_dst)
            .map_err(MaildirError::CopyMsgError)?;

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir_dst.path())?;
        let hash = format!("{:x}", md5::compute(&id));
        mapper.append(vec![(hash.clone(), id.clone())])?;

        info!("<< copy maildir message");
        Ok(())
    }

    fn move_msg(&mut self, dir_src: &str, dir_dst: &str, short_hash: &str) -> Result<()> {
        info!(">> move maildir message");
        debug!("source dir: {:?}", dir_src);
        debug!("destination dir: {:?}", dir_dst);

        let mdir_src = self.get_mdir_from_dir(dir_src)?;
        let mdir_dst = self.get_mdir_from_dir(dir_dst)?;
        let id = IdMapper::new(mdir_src.path())?.find(short_hash)?;
        debug!("id: {:?}", id);

        mdir_src
            .move_to(&id, &mdir_dst)
            .map_err(MaildirError::MoveMsgError)?;

        // Appends hash entry to the id mapper cache file.
        let mut mapper = IdMapper::new(mdir_dst.path())?;
        let hash = format!("{:x}", md5::compute(&id));
        mapper.append(vec![(hash.clone(), id.clone())])?;

        info!("<< move maildir message");
        Ok(())
    }

    fn del_msg(&mut self, dir: &str, short_hash: &str) -> Result<()> {
        info!(">> delete maildir message");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);

        let mdir = self.get_mdir_from_dir(dir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        debug!("id: {:?}", id);
        mdir.delete(&id).map_err(MaildirError::DelMsgError)?;

        info!("<< delete maildir message");
        Ok(())
    }

    fn add_flags(&mut self, dir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> add maildir message flags");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);
        let flags = Flags::from(flags);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(dir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        debug!("id: {:?}", id);

        mdir.add_flags(&id, &maildir_flags::to_normalized_string(&flags))
            .map_err(MaildirError::AddFlagsError)?;

        info!("<< add maildir message flags");
        Ok(())
    }

    fn set_flags(&mut self, dir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> set maildir message flags");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);
        let flags = Flags::from(flags);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(dir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        debug!("id: {:?}", id);
        mdir.set_flags(&id, &maildir_flags::to_normalized_string(&flags))
            .map_err(MaildirError::SetFlagsError)?;

        info!("<< set maildir message flags");
        Ok(())
    }

    fn del_flags(&mut self, dir: &str, short_hash: &str, flags: &str) -> Result<()> {
        info!(">> delete maildir message flags");
        debug!("dir: {:?}", dir);
        debug!("short hash: {:?}", short_hash);
        let flags = Flags::from(flags);
        debug!("flags: {:?}", flags);

        let mdir = self.get_mdir_from_dir(dir)?;
        let id = IdMapper::new(mdir.path())?.find(short_hash)?;
        debug!("id: {:?}", id);
        mdir.remove_flags(&id, &maildir_flags::to_normalized_string(&flags))
            .map_err(MaildirError::DelFlagsError)?;

        info!("<< delete maildir message flags");
        Ok(())
    }
}
