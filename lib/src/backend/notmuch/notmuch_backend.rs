use log::{debug, info, trace};
use std::fs;

use crate::{
    account::{AccountConfig, NotmuchBackendConfig},
    backend::{
        backend::Result, notmuch_envelopes, Backend, IdMapper, MaildirBackend, NotmuchError,
    },
    mbox::{Mbox, Mboxes},
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
            .map_err(NotmuchError::OpenDbError)?,
        };

        info!("<< create new notmuch backend");
        Ok(backend)
    }

    fn _search_envelopes(
        &mut self,
        query: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        // Gets envelopes matching the given Notmuch query.
        let query_builder = self
            .db
            .create_query(query)
            .map_err(NotmuchError::BuildQueryError)?;
        let mut envelopes = notmuch_envelopes::from_notmuch_msgs(
            query_builder
                .search_messages()
                .map_err(NotmuchError::SearchEnvelopesError)?,
        )?;
        debug!("envelopes len: {:?}", envelopes.len());
        trace!("envelopes: {:?}", envelopes);

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        debug!("page begin: {:?}", page_begin);
        if page_begin > envelopes.len() {
            return Err(NotmuchError::GetEnvelopesOutOfBoundsError(page_begin + 1))?;
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
                .map(|env| (env.id.to_owned(), env.internal_id.to_owned()))
                .collect();
            mapper.append(entries)?
        };
        debug!("short hash length: {:?}", short_hash_len);

        // Shorten envelopes hash.
        envelopes
            .iter_mut()
            .for_each(|env| env.id = env.id[0..short_hash_len].to_owned());

        Ok(envelopes)
    }
}

impl<'a> Backend<'a> for NotmuchBackend<'a> {
    fn add_mbox(&mut self, _mbox: &str) -> Result<()> {
        info!(">> add notmuch mailbox");
        info!("<< add notmuch mailbox");
        Err(NotmuchError::AddMboxUnimplementedError)?
    }

    fn get_mboxes(&mut self) -> Result<Mboxes> {
        trace!(">> get notmuch virtual mailboxes");

        let mut mboxes = Mboxes::default();
        for (name, desc) in &self.account_config.mailboxes {
            mboxes.push(Mbox {
                name: name.into(),
                desc: desc.into(),
                ..Mbox::default()
            })
        }
        mboxes.sort_by(|a, b| b.name.partial_cmp(&a.name).unwrap());

        trace!("notmuch virtual mailboxes: {:?}", mboxes);
        trace!("<< get notmuch virtual mailboxes");
        Ok(mboxes)
    }

    fn del_mbox(&mut self, _mbox: &str) -> Result<()> {
        info!(">> delete notmuch mailbox");
        info!("<< delete notmuch mailbox");
        Err(NotmuchError::DelMboxUnimplementedError)?
    }

    fn get_envelopes(
        &mut self,
        virt_mbox: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
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
    ) -> Result<Envelopes> {
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

    fn add_msg(&mut self, _: &str, msg: &[u8], tags: &str) -> Result<String> {
        info!(">> add notmuch envelopes");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;

        // Adds the message to the maildir folder and gets its hash.
        let hash = self.mdir.add_msg("", msg, "seen")?;
        debug!("hash: {:?}", hash);

        // Retrieves the file path of the added message by its maildir
        // identifier.
        let mut mapper = IdMapper::new(dir)?;
        let id = mapper.find(&hash)?;
        debug!("id: {:?}", id);
        let file_path = dir.join("cur").join(format!("{}:2,S", id));
        debug!("file path: {:?}", file_path);

        println!("file_path: {:?}", file_path);
        // Adds the message to the notmuch database by indexing it.
        let id = self
            .db
            .index_file(&file_path, None)
            .map_err(NotmuchError::IndexFileError)?
            .id()
            .to_string();
        let hash = format!("{:x}", md5::compute(&id));

        // Appends hash entry to the id mapper cache file.
        mapper.append(vec![(hash.clone(), id.clone())])?;

        // Attaches tags to the notmuch message.
        self.add_flags("", &hash, tags)?;

        info!("<< add notmuch envelopes");
        Ok(hash)
    }

    fn get_msg(&mut self, _: &str, short_hash: &str) -> Result<Msg> {
        info!(">> add notmuch envelopes");
        debug!("short hash: {:?}", short_hash);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)?.find(short_hash)?;
        debug!("id: {:?}", id);
        let msg_file_path = self
            .db
            .find_message(&id)
            .map_err(NotmuchError::FindMsgError)?
            .ok_or_else(|| NotmuchError::FindMsgEmptyError)?
            .filename()
            .to_owned();
        debug!("message file path: {:?}", msg_file_path);
        let raw_msg = fs::read(&msg_file_path).map_err(NotmuchError::ReadMsgError)?;
        let msg = mailparse::parse_mail(&raw_msg).map_err(NotmuchError::ParseMsgError)?;
        let msg = Msg::from_parsed_mail(msg, &self.account_config)?;
        trace!("message: {:?}", msg);

        info!("<< get notmuch message");
        Ok(msg)
    }

    fn copy_msg(&mut self, _dir_src: &str, _dir_dst: &str, _short_hash: &str) -> Result<()> {
        info!(">> copy notmuch message");
        info!("<< copy notmuch message");
        Err(NotmuchError::CopyMsgUnimplementedError)?
    }

    fn move_msg(&mut self, _dir_src: &str, _dir_dst: &str, _short_hash: &str) -> Result<()> {
        info!(">> move notmuch message");
        info!("<< move notmuch message");
        Err(NotmuchError::MoveMsgUnimplementedError)?
    }

    fn del_msg(&mut self, _virt_mbox: &str, short_hash: &str) -> Result<()> {
        info!(">> delete notmuch message");
        debug!("short hash: {:?}", short_hash);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)?.find(short_hash)?;
        debug!("id: {:?}", id);
        let msg_file_path = self
            .db
            .find_message(&id)
            .map_err(NotmuchError::FindMsgError)?
            .ok_or_else(|| NotmuchError::FindMsgEmptyError)?
            .filename()
            .to_owned();
        debug!("message file path: {:?}", msg_file_path);
        self.db
            .remove_message(msg_file_path)
            .map_err(NotmuchError::DelMsgError)?;

        info!("<< delete notmuch message");
        Ok(())
    }

    fn add_flags(&mut self, _virt_mbox: &str, short_hash: &str, tags: &str) -> Result<()> {
        info!(">> add notmuch message flags");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)?.find(short_hash)?;
        debug!("id: {:?}", id);
        let query = format!("id:{}", id);
        debug!("query: {:?}", query);
        let tags: Vec<_> = tags.split_whitespace().collect();
        let query_builder = self
            .db
            .create_query(&query)
            .map_err(NotmuchError::BuildQueryError)?;
        let msgs = query_builder
            .search_messages()
            .map_err(NotmuchError::SearchEnvelopesError)?;

        for msg in msgs {
            for tag in tags.iter() {
                msg.add_tag(*tag).map_err(NotmuchError::AddTagError)?;
            }
        }

        info!("<< add notmuch message flags");
        Ok(())
    }

    fn set_flags(&mut self, _virt_mbox: &str, short_hash: &str, tags: &str) -> Result<()> {
        info!(">> set notmuch message flags");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)?.find(short_hash)?;
        debug!("id: {:?}", id);
        let query = format!("id:{}", id);
        debug!("query: {:?}", query);
        let tags: Vec<_> = tags.split_whitespace().collect();
        let query_builder = self
            .db
            .create_query(&query)
            .map_err(NotmuchError::BuildQueryError)?;
        let msgs = query_builder
            .search_messages()
            .map_err(NotmuchError::SearchEnvelopesError)?;
        for msg in msgs {
            msg.remove_all_tags().map_err(NotmuchError::DelTagError)?;

            for tag in tags.iter() {
                msg.add_tag(*tag).map_err(NotmuchError::AddTagError)?;
            }
        }

        info!("<< set notmuch message flags");
        Ok(())
    }

    fn del_flags(&mut self, _virt_mbox: &str, short_hash: &str, tags: &str) -> Result<()> {
        info!(">> delete notmuch message flags");
        debug!("tags: {:?}", tags);

        let dir = &self.notmuch_config.notmuch_database_dir;
        let id = IdMapper::new(dir)?.find(short_hash)?;
        debug!("id: {:?}", id);
        let query = format!("id:{}", id);
        debug!("query: {:?}", query);
        let tags: Vec<_> = tags.split_whitespace().collect();
        let query_builder = self
            .db
            .create_query(&query)
            .map_err(NotmuchError::BuildQueryError)?;
        let msgs = query_builder
            .search_messages()
            .map_err(NotmuchError::SearchEnvelopesError)?;
        for msg in msgs {
            for tag in tags.iter() {
                msg.remove_tag(*tag).map_err(NotmuchError::DelTagError)?;
            }
        }

        info!("<< delete notmuch message flags");
        Ok(())
    }
}
