pub mod arg;
pub mod args;

use anyhow::{anyhow, Context, Result};
use email::account::config::AccountConfig;
use log::{debug, trace};
use std::path::{Path, PathBuf};

const ID_MAPPER_DB_FILE_NAME: &str = ".id-mapper.sqlite";

#[derive(Debug)]
pub enum IdMapper {
    Dummy,
    Mapper(String, rusqlite::Connection),
}

impl IdMapper {
    pub fn find_closest_db_path(dir: impl AsRef<Path>) -> PathBuf {
        let mut db_path = dir.as_ref().join(ID_MAPPER_DB_FILE_NAME);
        let mut db_parent_dir = dir.as_ref().parent();

        while !db_path.is_file() {
            match db_parent_dir {
                Some(dir) => {
                    db_path = dir.join(ID_MAPPER_DB_FILE_NAME);
                    db_parent_dir = dir.parent();
                }
                None => {
                    db_path = dir.as_ref().join(ID_MAPPER_DB_FILE_NAME);
                    break;
                }
            }
        }

        db_path
    }

    pub fn new(account_config: &AccountConfig, folder: &str, db_path: PathBuf) -> Result<Self> {
        let folder = account_config.get_folder_alias(folder);
        let digest = md5::compute(account_config.name.clone() + &folder);
        let table = format!("id_mapper_{digest:x}");
        debug!("creating id mapper table {table} at {db_path:?}…");

        let db_path = Self::find_closest_db_path(db_path);
        let conn = rusqlite::Connection::open(&db_path)
            .with_context(|| format!("cannot open id mapper database at {db_path:?}"))?;

        let query = format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                internal_id TEXT UNIQUE
            )",
        );
        trace!("create table query: {query:#?}");

        conn.execute(&query, [])
            .context("cannot create id mapper table")?;

        Ok(Self::Mapper(table, conn))
    }

    pub fn create_alias<I>(&self, id: I) -> Result<String>
    where
        I: AsRef<str>,
    {
        let id = id.as_ref();
        match self {
            Self::Dummy => Ok(id.to_owned()),
            Self::Mapper(table, conn) => {
                debug!("creating alias for id {id}…");

                let query = format!("INSERT OR IGNORE INTO {} (internal_id) VALUES (?)", table);
                trace!("insert query: {query:#?}");

                conn.execute(&query, [id])
                    .with_context(|| format!("cannot create id alias for id {id}"))?;

                let alias = conn.last_insert_rowid().to_string();
                debug!("created alias {alias} for id {id}");

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
            Self::Mapper(table, conn) => {
                debug!("getting alias for id {id}…");

                let query = format!("SELECT id FROM {} WHERE internal_id = ?", table);
                trace!("select query: {query:#?}");

                let mut stmt = conn
                    .prepare(&query)
                    .with_context(|| format!("cannot get alias for id {id}"))?;
                let aliases: Vec<i64> = stmt
                    .query_map([id], |row| row.get(0))
                    .with_context(|| format!("cannot get alias for id {id}"))?
                    .collect::<rusqlite::Result<_>>()
                    .with_context(|| format!("cannot get alias for id {id}"))?;
                let alias = match aliases.first() {
                    Some(alias) => {
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
        let alias = alias
            .parse::<i64>()
            .context(format!("cannot parse id mapper alias {alias}"))?;

        match self {
            Self::Dummy => Ok(alias.to_string()),
            Self::Mapper(table, conn) => {
                debug!("getting id from alias {alias}…");

                let query = format!("SELECT internal_id FROM {} WHERE id = ?", table);
                trace!("select query: {query:#?}");

                let mut stmt = conn
                    .prepare(&query)
                    .with_context(|| format!("cannot get id from alias {alias}"))?;
                let ids: Vec<String> = stmt
                    .query_map([alias], |row| row.get(0))
                    .with_context(|| format!("cannot get id from alias {alias}"))?
                    .collect::<rusqlite::Result<_>>()
                    .with_context(|| format!("cannot get id from alias {alias}"))?;
                let id = ids
                    .first()
                    .ok_or_else(|| anyhow!("cannot get id from alias {alias}"))?
                    .to_owned();
                debug!("found id {id} from alias {alias}");

                Ok(id)
            }
        }
    }

    pub fn get_ids<A, I>(&self, aliases: I) -> Result<Vec<String>>
    where
        A: ToString,
        I: IntoIterator<Item = A>,
    {
        aliases
            .into_iter()
            .map(|alias| self.get_id(alias))
            .collect()
    }
}
