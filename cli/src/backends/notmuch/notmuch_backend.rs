use std::{convert::TryInto, fs};

use anyhow::{anyhow, Context, Result};
use himalaya_lib::account::{AccountConfig, NotmuchBackendConfig};
use log::{debug, info, trace};

use crate::{
    backends::{Backend, IdMapper, MaildirBackend, NotmuchEnvelopes, NotmuchMbox, NotmuchMboxes},
    mbox::Mboxes,
    msg::{Envelopes, Msg},
};

/// Represents the Notmuch backend.
pub struct NotmuchBackend<'a> {
    account_config: &'a AccountConfig,
    notmuch_config: &'a NotmuchBackendConfig,
    pub mdir: &'a mut MaildirBackend<'a>,
    db: notmuch::Database,
}

impl<'a> NotmuchBackend<'a> {
    pub fn new(
        account_config: &'a AccountConfig,
        notmuch_config: &'a NotmuchBackendConfig,
        mdir: &'a mut MaildirBackend<'a>,
    ) -> Result<NotmuchBackend<'a>> {
        info!(">> create new notmuch backend");

        let backend = Self {
            account_config,
            notmuch_config,
            mdir,
            db: notmuch::Database::open(
                notmuch_config.notmuch_database_dir.clone(),
                notmuch::DatabaseMode::ReadWrite,
            )
            .with_context(|| {
                format!(
                    "cannot open notmuch database at {:?}",
                    notmuch_config.notmuch_database_dir
                )
            })?,
        };

        info!("<< create new notmuch backend");
        Ok(backend)
    }

    fn _search_envelopes(
        &mut self,
        query: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        // Gets envelopes matching the given Notmuch query.
        let query_builder = self
            .db
            .create_query(query)
            .with_context(|| format!("cannot create notmuch query from {:?}", query))?;
        let mut envelopes: NotmuchEnvelopes = query_builder
            .search_messages()
            .with_context(|| format!("cannot find notmuch envelopes from query {:?}", query))?
            .try_into()
            .with_context(|| format!("cannot parse notmuch envelopes from query {:?}", query))?;
        debug!("envelopes len: {:?}", envelopes.len());
        trace!("envelopes: {:?}", envelopes);

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        debug!("page begin: {:?}", page_begin);
        if page_begin > envelopes.len() {
            return Err(anyhow!(
                "cannot get notmuch envelopes at page {:?} (out of bounds)",
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
            let mut mapper = IdMapper::new(&self.notmuch_config.notmuch_database_dir)?;
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

        Ok(Box::new(envelopes))
    }
}

impl<'a> Backend<'a> for NotmuchBackend<'a> {
    fn add_mbox(&mut self, _mbox: &str) -> Result<()> {
        info!(">> add notmuch mailbox");
        info!("<< add notmuch mailbox");
        Err(anyhow!(
            "cannot add notmuch mailbox: feature not implemented"
        ))
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        info!(">> get notmuch virtual mailboxes");

        let mut mboxes: Vec<_> = self
            .account_config
            .mailboxes
            .iter()
            .map(|(k, v)| NotmuchMbox::new(k, v))
            .collect();
        trace!("virtual mailboxes: {:?}", mboxes);
        mboxes.sort_by(|a, b| b.name.partial_cmp(&a.name).unwrap());

        info!("<< get notmuch virtual mailboxes");
        Ok(Box::new(NotmuchMboxes { mboxes }))
    }

    fn del_mbox(&mut self, _mbox: &str) -> Result<()> {
        info!(">> delete notmuch mailbox");
        info!("<< delete notmuch mailbox");
        Err(anyhow!(
            "cannot delete notmuch mailbox: feature not implemented"
        ))
    }

    fn get_envelopes(
        &mut self,
        virt_mbox: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        info!(">> get notmuch envelopes");
        debug!("virtual mailbox: {:?}", virt_mbox);
        debug!("page size: {:?}", page_size);
        debug!("page: {:?}", page);

        let query = self
            .account_config
            .mailboxes
            .get(virt_mbox)
            .map(|s| s.as_str())
            .unwrap_or("all");
        debug!("query: {:?}", query);
        let envelopes = self._search_envelopes(query, page_size, page)?;

        info!("<< get notmuch envelopes");
        Ok(envelopes)
    }

    fn search_envelopes(
        &mut self,
        virt_mbox: &str,
        query: &str,
        _sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        info!(">> search notmuch envelopes");
        debug!("virtual mailbox: {:?}", virt_mbox);
        debug!("query: {:?}", query);
        debug!("page size: {:?}", page_size);
        debug!("page: {:?}", page);

        let query = if query.is_empty() {
            self.account_config
                .mailboxes
                .get(virt_mbox)
                .map(|s| s.as_str())
                .unwrap_or("all")
        } else {
            query
        };
        debug!("final query: {:?}", query);
        let envelopes = self._search_envelopes(query, page_size, page)?;

        info!("<< search notmuch envelopes");
        Ok(envelopes)
    }

    fn add_msg(&mut self, _: &str, msg: &[u8], tags: &str) -> Result<Box<dyn ToString>> {
        info!(">> add notmuch envelopes");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;

        // Adds the message to the maildir folder and gets its hash.
        let hash = self
            .mdir
            .add_msg("", msg, "seen")
            .with_context(|| {
                format!(
                    "cannot add notmuch message to maildir {:?}",
                    self.notmuch_config.notmuch_database_dir
                )
            })?
            .to_string();
        debug!("hash: {:?}", hash);

        // Retrieves the file path of the added message by its maildir
        // identifier.
        let mut mapper = IdMapper::new(dir)
            .with_context(|| format!("cannot create id mapper instance for {:?}", dir))?;
        let id = mapper
            .find(&hash)
            .with_context(|| format!("cannot find notmuch message from short hash {:?}", hash))?;
        debug!("id: {:?}", id);
        let file_path = dir.join("cur").join(format!("{}:2,S", id));
        debug!("file path: {:?}", file_path);

        // Adds the message to the notmuch database by indexing it.
        let id = self
            .db
            .index_file(&file_path, None)
            .with_context(|| format!("cannot index notmuch message from file {:?}", file_path))?
            .id()
            .to_string();
        let hash = format!("{:x}", md5::compute(&id));

        // Appends hash entry to the id mapper cache file.
        mapper
            .append(vec![(hash.clone(), id.clone())])
            .with_context(|| {
                format!(
                    "cannot append hash {:?} with id {:?} to id mapper",
                    hash, id
                )
            })?;

        // Attaches tags to the notmuch message.
        self.add_flags("", &hash, tags)
            .with_context(|| format!("cannot add flags to notmuch message {:?}", id))?;

        info!("<< add notmuch envelopes");
        Ok(Box::new(hash))
    }

    fn get_msg(&mut self, _: &str, short_hash: &str) -> Result<Msg> {
        info!(">> add notmuch envelopes");
        debug!("short hash: {:?}", short_hash);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)
            .with_context(|| format!("cannot create id mapper instance for {:?}", dir))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find notmuch message from short hash {:?}",
                    short_hash
                )
            })?;
        debug!("id: {:?}", id);
        let msg_file_path = self
            .db
            .find_message(&id)
            .with_context(|| format!("cannot find notmuch message {:?}", id))?
            .ok_or_else(|| anyhow!("cannot find notmuch message {:?}", id))?
            .filename()
            .to_owned();
        debug!("message file path: {:?}", msg_file_path);
        let raw_msg = fs::read(&msg_file_path).with_context(|| {
            format!("cannot read notmuch message from file {:?}", msg_file_path)
        })?;
        let msg = mailparse::parse_mail(&raw_msg)
            .with_context(|| format!("cannot parse raw notmuch message {:?}", id))?;
        let msg = Msg::from_parsed_mail(msg, &self.account_config)
            .with_context(|| format!("cannot parse notmuch message {:?}", id))?;
        trace!("message: {:?}", msg);

        info!("<< get notmuch message");
        Ok(msg)
    }

    fn copy_msg(&mut self, _dir_src: &str, _dir_dst: &str, _short_hash: &str) -> Result<()> {
        info!(">> copy notmuch message");
        info!("<< copy notmuch message");
        Err(anyhow!(
            "cannot copy notmuch message: feature not implemented"
        ))
    }

    fn move_msg(&mut self, _dir_src: &str, _dir_dst: &str, _short_hash: &str) -> Result<()> {
        info!(">> move notmuch message");
        info!("<< move notmuch message");
        Err(anyhow!(
            "cannot move notmuch message: feature not implemented"
        ))
    }

    fn del_msg(&mut self, _virt_mbox: &str, short_hash: &str) -> Result<()> {
        info!(">> delete notmuch message");
        debug!("short hash: {:?}", short_hash);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)
            .with_context(|| format!("cannot create id mapper instance for {:?}", dir))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find notmuch message from short hash {:?}",
                    short_hash
                )
            })?;
        debug!("id: {:?}", id);
        let msg_file_path = self
            .db
            .find_message(&id)
            .with_context(|| format!("cannot find notmuch message {:?}", id))?
            .ok_or_else(|| anyhow!("cannot find notmuch message {:?}", id))?
            .filename()
            .to_owned();
        debug!("message file path: {:?}", msg_file_path);
        self.db
            .remove_message(msg_file_path)
            .with_context(|| format!("cannot delete notmuch message {:?}", id))?;

        info!("<< delete notmuch message");
        Ok(())
    }

    fn add_flags(&mut self, _virt_mbox: &str, short_hash: &str, tags: &str) -> Result<()> {
        info!(">> add notmuch message flags");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)
            .with_context(|| format!("cannot create id mapper instance for {:?}", dir))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find notmuch message from short hash {:?}",
                    short_hash
                )
            })?;
        debug!("id: {:?}", id);
        let query = format!("id:{}", id);
        debug!("query: {:?}", query);
        let tags: Vec<_> = tags.split_whitespace().collect();
        let query_builder = self
            .db
            .create_query(&query)
            .with_context(|| format!("cannot create notmuch query from {:?}", query))?;
        let msgs = query_builder
            .search_messages()
            .with_context(|| format!("cannot find notmuch envelopes from query {:?}", query))?;
        for msg in msgs {
            for tag in tags.iter() {
                msg.add_tag(*tag).with_context(|| {
                    format!("cannot add tag {:?} to notmuch message {:?}", tag, msg.id())
                })?
            }
        }

        info!("<< add notmuch message flags");
        Ok(())
    }

    fn set_flags(&mut self, _virt_mbox: &str, short_hash: &str, tags: &str) -> Result<()> {
        info!(">> set notmuch message flags");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)
            .with_context(|| format!("cannot create id mapper instance for {:?}", dir))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find notmuch message from short hash {:?}",
                    short_hash
                )
            })?;
        debug!("id: {:?}", id);
        let query = format!("id:{}", id);
        debug!("query: {:?}", query);
        let tags: Vec<_> = tags.split_whitespace().collect();
        let query_builder = self
            .db
            .create_query(&query)
            .with_context(|| format!("cannot create notmuch query from {:?}", query))?;
        let msgs = query_builder
            .search_messages()
            .with_context(|| format!("cannot find notmuch envelopes from query {:?}", query))?;
        for msg in msgs {
            msg.remove_all_tags().with_context(|| {
                format!("cannot remove all tags from notmuch message {:?}", msg.id())
            })?;
            for tag in tags.iter() {
                msg.add_tag(*tag).with_context(|| {
                    format!("cannot add tag {:?} to notmuch message {:?}", tag, msg.id())
                })?
            }
        }

        info!("<< set notmuch message flags");
        Ok(())
    }

    fn del_flags(&mut self, _virt_mbox: &str, short_hash: &str, tags: &str) -> Result<()> {
        info!(">> delete notmuch message flags");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)
            .with_context(|| format!("cannot create id mapper instance for {:?}", dir))?
            .find(short_hash)
            .with_context(|| {
                format!(
                    "cannot find notmuch message from short hash {:?}",
                    short_hash
                )
            })?;
        debug!("id: {:?}", id);
        let query = format!("id:{}", id);
        debug!("query: {:?}", query);
        let tags: Vec<_> = tags.split_whitespace().collect();
        let query_builder = self
            .db
            .create_query(&query)
            .with_context(|| format!("cannot create notmuch query from {:?}", query))?;
        let msgs = query_builder
            .search_messages()
            .with_context(|| format!("cannot find notmuch envelopes from query {:?}", query))?;
        for msg in msgs {
            for tag in tags.iter() {
                msg.remove_tag(*tag).with_context(|| {
                    format!(
                        "cannot delete tag {:?} from notmuch message {:?}",
                        tag,
                        msg.id()
                    )
                })?
            }
        }

        info!("<< delete notmuch message flags");
        Ok(())
    }
}
