use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashSet,
    convert::TryInto,
    env::temp_dir,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    iter::FromIterator,
    path::PathBuf,
};

use crate::{
    backends::{Backend, MaildirEnvelopes, MaildirFlags, MaildirMboxes},
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

    fn write_envelopes_cache(cache: &[u8]) -> Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(temp_dir().join("himalaya-msg-id-hash-map"))
            .context("cannot open maildir id hash map cache")?
            .write(cache)
            .map(|_| ())
            .context("cannot write maildir id hash map cache")
    }

    fn get_id_from_short_hash(short_hash: &str) -> Result<String> {
        let path = temp_dir().join("himalaya-msg-id-hash-map");
        let file = OpenOptions::new()
            .read(true)
            .open(path)
            .context("cannot open id hash map file")?;
        let reader = BufReader::new(file);
        let mut id_found = None;
        for line in reader.lines() {
            let line = line.context("cannot read id hash map line")?;
            let line = line
                .split_once(' ')
                .ok_or_else(|| anyhow!("cannot parse id hash map line {:?}", line));
            match line {
                Ok((id, hash)) if hash.starts_with(short_hash) => {
                    if id_found.is_some() {
                        return Err(anyhow!(
                            "cannot find id from hash {:?}: multiple match found",
                            short_hash
                        ));
                    } else {
                        id_found = Some(id.to_owned())
                    }
                }
                _ => continue,
            }
        }
        id_found.ok_or_else(|| anyhow!("cannot find id from hash {:?}", short_hash))
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

        let mut envelopes: MaildirEnvelopes = mdir
            .list_cur()
            .try_into()
            .context("cannot parse maildir envelopes from {:?}")?;

        Self::write_envelopes_cache(
            envelopes
                .iter()
                .map(|env| format!("{} {:x}", env.id, md5::compute(&env.id)))
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        )?;

        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        envelopes
            .iter_mut()
            .for_each(|env| env.id = format!("{:x}", md5::compute(&env.id)));

        let mut short_id_len = 2;
        loop {
            let short_ids: Vec<_> = envelopes
                .iter()
                .map(|env| env.id[0..short_id_len].to_string())
                .collect();
            let short_ids_set: HashSet<String> = HashSet::from_iter(short_ids.iter().cloned());

            if short_id_len > 32 {
                break;
            }

            if short_ids.len() == short_ids_set.len() {
                break;
            }

            short_id_len += 1;
        }

        envelopes
            .iter_mut()
            .for_each(|env| env.id = env.id[0..short_id_len].to_string());

        let page_begin = page * page_size;
        if page_begin > envelopes.len() {
            return Err(anyhow!(format!(
                "cannot list maildir envelopes at page {:?} (out of bounds)",
                page_begin + 1,
            )));
        }
        let page_end = envelopes.len().min(page_begin + page_size);
        envelopes.0 = envelopes[page_begin..page_end].to_owned();

        Ok(Box::new(envelopes))
    }

    fn find_envelopes(
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
            .context(format!(
                "cannot add message to the \"cur\" folder of maildir {:?}",
                mdir.path()
            ))?;
        Ok(Box::new(id))
    }

    fn get_msg(&mut self, mdir: &str, hash: &str) -> Result<Msg> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let id = Self::get_id_from_short_hash(hash)
            .context(format!("cannot get msg from hash {:?}", hash))?;
        let mut mail_entry = mdir.find(&id).ok_or_else(|| {
            anyhow!(
                "cannot find maildir message {:?} in {:?}",
                hash,
                mdir.path()
            )
        })?;
        let parsed_mail = mail_entry.parsed().context(format!(
            "cannot parse maildir message {:?} in {:?}",
            hash,
            mdir.path()
        ))?;
        Msg::from_parsed_mail(parsed_mail, self.account_config).context(format!(
            "cannot parse maildir message {:?} from {:?}",
            hash,
            mdir.path()
        ))
    }

    fn copy_msg(&mut self, mdir_src: &str, mdir_dst: &str, id: &str) -> Result<()> {
        let mdir_src = self.get_mdir_from_name(mdir_src)?;
        let mdir_dst = self.get_mdir_from_name(mdir_dst)?;
        mdir_src.copy_to(id, &mdir_dst).context(format!(
            "cannot copy message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))
    }

    fn move_msg(&mut self, mdir_src: &str, mdir_dst: &str, id: &str) -> Result<()> {
        let mdir_src = self.get_mdir_from_name(mdir_src)?;
        let mdir_dst = self.get_mdir_from_name(mdir_dst)?;
        mdir_src.move_to(id, &mdir_dst).context(format!(
            "cannot move message {:?} from maildir {:?} to maildir {:?}",
            id,
            mdir_src.path(),
            mdir_dst.path()
        ))
    }

    fn del_msg(&mut self, mdir: &str, id: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        mdir.delete(id).context(format!(
            "cannot delete message {:?} from maildir {:?}",
            id,
            mdir.path()
        ))
    }

    fn add_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.add_flags(id, &flags.to_string()).context(format!(
            "cannot add flags {:?} to maildir message {:?}",
            flags_str, id
        ))
    }

    fn set_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.set_flags(id, &flags.to_string()).context(format!(
            "cannot set flags {:?} to maildir message {:?}",
            flags_str, id
        ))
    }

    fn del_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        let mdir = self.get_mdir_from_name(mdir)?;
        let flags: MaildirFlags = flags_str.try_into()?;
        mdir.remove_flags(id, &flags.to_string()).context(format!(
            "cannot remove flags {:?} from maildir message {:?}",
            flags_str, id
        ))
    }
}
