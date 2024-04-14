pub mod arg;

use color_eyre::{eyre::eyre, eyre::Context, Result};
use dirs::data_dir;
use email::account::config::AccountConfig;
use sled::{Config, Db};
use std::collections::HashSet;
use tracing::debug;

#[derive(Debug)]
pub enum IdMapper {
    Dummy,
    Mapper(Db),
}

impl IdMapper {
    pub fn new(account_config: &AccountConfig, folder: &str) -> Result<Self> {
        let digest = md5::compute(account_config.name.clone() + folder);
        let db_path = data_dir()
            .ok_or(eyre!("cannot get XDG data directory"))?
            .join("himalaya")
            .join(".id-mappers")
            .join(format!("{digest:x}"));

        let conn = Config::new()
            .path(&db_path)
            .idgen_persist_interval(1)
            .open()
            .with_context(|| format!("cannot open id mapper database at {db_path:?}"))?;

        Ok(Self::Mapper(conn))
    }

    pub fn create_alias<I>(&self, id: I) -> Result<String>
    where
        I: AsRef<str>,
    {
        let id = id.as_ref();
        match self {
            Self::Dummy => Ok(id.to_owned()),
            Self::Mapper(conn) => {
                debug!("creating alias for id {id}…");

                let alias = conn
                    .generate_id()
                    .with_context(|| format!("cannot create alias for id {id}"))?
                    .to_string();
                debug!("created alias {alias} for id {id}");

                conn.insert(&id, alias.as_bytes())
                    .with_context(|| format!("cannot insert alias {alias} for id {id}"))?;

                Ok(alias)
            }
        }
    }

    pub fn get_or_create_alias<I>(&self, id: I) -> Result<String>
    where
        I: AsRef<str>,
    {
        let id = id.as_ref();
        match self {
            Self::Dummy => Ok(id.to_owned()),
            Self::Mapper(conn) => {
                debug!("getting alias for id {id}…");

                let alias = conn
                    .get(id)
                    .with_context(|| format!("cannot get alias for id {id}"))?;

                let alias = match alias {
                    Some(alias) => {
                        let alias = String::from_utf8_lossy(alias.as_ref());
                        debug!("found alias {alias} for id {id}");
                        alias.to_string()
                    }
                    None => {
                        debug!("alias not found, creating it…");
                        self.create_alias(id)?
                    }
                };

                Ok(alias)
            }
        }
    }

    pub fn get_id<A>(&self, alias: A) -> Result<String>
    where
        A: ToString,
    {
        let alias = alias.to_string();

        match self {
            Self::Dummy => Ok(alias.to_string()),
            Self::Mapper(conn) => {
                debug!("getting id from alias {alias}…");

                let id = conn
                    .iter()
                    .flat_map(|entry| entry)
                    .find_map(|(entry_id, entry_alias)| {
                        if entry_alias.as_ref() == alias.as_bytes() {
                            let entry_id = String::from_utf8_lossy(entry_id.as_ref());
                            Some(entry_id.to_string())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| eyre!("cannot get id from alias {alias}"))?;
                debug!("found id {id} from alias {alias}");

                Ok(id)
            }
        }
    }

    pub fn get_ids(&self, aliases: impl IntoIterator<Item = impl ToString>) -> Result<Vec<String>> {
        let aliases: Vec<String> = aliases.into_iter().map(|alias| alias.to_string()).collect();

        match self {
            Self::Dummy => Ok(aliases),
            Self::Mapper(conn) => {
                let aliases: HashSet<&str> = aliases.iter().map(|alias| alias.as_str()).collect();
                let ids: Vec<String> = conn
                    .iter()
                    .flat_map(|entry| entry)
                    .filter_map(|(entry_id, entry_alias)| {
                        let alias = String::from_utf8_lossy(entry_alias.as_ref());
                        if aliases.contains(alias.as_ref()) {
                            let entry_id = String::from_utf8_lossy(entry_id.as_ref());
                            Some(entry_id.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                Ok(ids)
            }
        }
    }
}
