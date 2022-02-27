use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub struct IdMapper {
    path: PathBuf,
    map: HashMap<String, String>,
    short_hash_len: usize,
}

impl IdMapper {
    pub fn new(dir: &Path) -> Result<Self> {
        let mut mapper = Self::default();
        mapper.path = dir.join(".himalaya-id-map");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&mapper.path)
            .context("cannot open id hash map file")?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line =
                line.context("cannot read line from maildir envelopes id mapper cache file")?;
            if mapper.short_hash_len == 0 {
                mapper.short_hash_len = 2.max(line.parse().unwrap_or(2));
            } else {
                let (hash, id) = line.split_once(' ').ok_or_else(|| {
                    anyhow!(
                        "cannot parse line {:?} from maildir envelopes id mapper cache file",
                        line
                    )
                })?;
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
            Err(anyhow!(
                "cannot find maildir message id from short hash {:?}",
                short_hash,
            ))
        } else if matching_hashes.len() > 1 {
            Err(anyhow!(
                "the short hash {:?} matches more than one hash: {}",
                short_hash,
                matching_hashes
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .context(format!(
                "cannot find maildir message id from short hash {:?}",
                short_hash
            )))
        } else {
            Ok(self.get(matching_hashes[0]).unwrap().to_owned())
        }
    }

    pub fn append(&mut self, lines: Vec<(String, String)>) -> Result<usize> {
        let mut entries = String::new();

        self.extend(lines.clone());

        for (hash, id) in self.iter() {
            entries.push_str(&format!("{} {}\n", hash, id));
        }

        for (hash, id) in lines {
            loop {
                let short_hash = &hash[0..self.short_hash_len];
                let conflict_found = self
                    .map
                    .keys()
                    .find(|cached_hash| {
                        cached_hash.starts_with(short_hash) && *cached_hash != &hash
                    })
                    .is_some();
                if self.short_hash_len > 32 || !conflict_found {
                    break;
                }
                self.short_hash_len += 1;
            }
            entries.push_str(&format!("{} {}\n", hash, id));
        }

        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .context("cannot open maildir id hash map cache")?
            .write(format!("{}\n{}", self.short_hash_len, entries).as_bytes())
            .context("cannot write maildir id hash map cache")?;

        Ok(self.short_hash_len)
    }
}

impl Deref for IdMapper {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for IdMapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
