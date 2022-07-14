use std::{
    collections, fs,
    io::{self, prelude::*},
    ops, path, result,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse id mapper cache line {0}")]
    ParseLineError(String),
    #[error("cannot find message id from short hash {0}")]
    FindFromShortHashError(String),
    #[error("the short hash {0} matches more than one hash: {1}")]
    MatchShortHashError(String, String),

    #[error("cannot open id mapper file: {1}")]
    OpenHashMapFileError(#[source] io::Error, path::PathBuf),
    #[error("cannot write id mapper file: {1}")]
    WriteHashMapFileError(#[source] io::Error, path::PathBuf),
    #[error("cannot read line from id mapper file")]
    ReadHashMapFileLineError(#[source] io::Error),
}

type Result<T> = result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct IdMapper {
    path: path::PathBuf,
    map: collections::HashMap<String, String>,
    short_hash_len: usize,
}

impl IdMapper {
    pub fn new(dir: &path::Path) -> Result<Self> {
        let mut mapper = Self::default();
        mapper.path = dir.join(".himalaya-id-map");

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&mapper.path)
            .map_err(|err| Error::OpenHashMapFileError(err, mapper.path.to_owned()))?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.map_err(Error::ReadHashMapFileLineError)?;
            if mapper.short_hash_len == 0 {
                mapper.short_hash_len = 2.max(line.parse().unwrap_or(2));
            } else {
                let (hash, id) = line
                    .split_once(' ')
                    .ok_or_else(|| Error::ParseLineError(line.to_owned()))?;
                mapper.insert(hash.to_owned(), id.to_owned());
            }
        }

        Ok(mapper)
    }

    pub fn find(&self, short_hash: &str) -> Result<String> {
        let matching_hashes: Vec<_> = self
            .keys()
            .filter(|hash| hash.starts_with(short_hash))
            .collect();
        if matching_hashes.len() == 0 {
            Err(Error::FindFromShortHashError(short_hash.to_owned()))
        } else if matching_hashes.len() > 1 {
            Err(Error::MatchShortHashError(
                short_hash.to_owned(),
                matching_hashes
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ))
        } else {
            Ok(self.get(matching_hashes[0]).unwrap().to_owned())
        }
    }

    pub fn append(&mut self, lines: Vec<(String, String)>) -> Result<usize> {
        self.extend(lines);

        let mut entries = String::new();
        let mut short_hash_len = self.short_hash_len;

        for (hash, id) in self.iter() {
            loop {
                let short_hash = &hash[0..short_hash_len];
                let conflict_found = self
                    .map
                    .keys()
                    .find(|cached_hash| cached_hash.starts_with(short_hash) && cached_hash != &hash)
                    .is_some();
                if short_hash_len > 32 || !conflict_found {
                    break;
                }
                short_hash_len += 1;
            }
            entries.push_str(&format!("{} {}\n", hash, id));
        }

        self.short_hash_len = short_hash_len;

        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|err| Error::OpenHashMapFileError(err, self.path.to_owned()))?
            .write(format!("{}\n{}", short_hash_len, entries).as_bytes())
            .map_err(|err| Error::WriteHashMapFileError(err, self.path.to_owned()))?;

        Ok(short_hash_len)
    }
}

impl ops::Deref for IdMapper {
    type Target = collections::HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl ops::DerefMut for IdMapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
