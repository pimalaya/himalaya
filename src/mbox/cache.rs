//! # mbox cache
//!
//! What Himalaya keeps between runs about an mbox file, so a listing does
//! not read the whole file again: the io-mbox scan index, and the
//! envelope of every message already read.
//!
//! An mbox has no directory of its contents, so without it every listing
//! scans and parses the whole file. The index tells an unchanged file
//! from one a delivery appended to, and the envelopes, keyed by content
//! id, stay valid whatever happens to the flags. The cache is an
//! optimisation only: a missing, stale or unreadable one is rebuilt, and
//! failing to write one is not an error.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use io_mbox::{index::MboxIndex, path::MboxFsPath};
use log::debug;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::email::envelope::Envelope;

/// The cached state of one mbox file.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MboxCache {
    /// The scan index, `None` until the file was scanned once.
    pub index: Option<MboxIndex>,
    /// Envelopes keyed by content id, their flags left empty since the
    /// index carries the current ones.
    pub envelopes: BTreeMap<String, Envelope>,
}

impl MboxCache {
    /// Loads the cache of the mbox at `path` from `dir`, an empty one when
    /// there is none or it cannot be read.
    pub fn load(dir: Option<&Path>, path: &MboxFsPath) -> Self {
        let Some(file) = dir.map(|dir| cache_file(dir, path)) else {
            return Self::default();
        };

        let cache = fs::read(&file)
            .map_err(|err| err.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|err| err.to_string()));

        match cache {
            Ok(cache) => cache,
            Err(err) => {
                debug!("cannot load mbox cache {}: {err}", file.display());
                Self::default()
            }
        }
    }

    /// Saves the cache of the mbox at `path` under `dir`, dropping the
    /// envelopes of messages the index no longer holds.
    pub fn save(&mut self, dir: Option<&Path>, path: &MboxFsPath) {
        let Some(dir) = dir else {
            return;
        };

        if let Some(index) = &self.index {
            let ids: Vec<&str> = index.entries.iter().map(|e| e.id.as_str()).collect();
            self.envelopes.retain(|id, _| ids.contains(&id.as_str()));
        }

        let file = cache_file(dir, path);
        let tmp = file.with_extension("json.tmp");
        let result = fs::create_dir_all(dir)
            .and_then(|()| fs::write(&tmp, serde_json::to_vec(self).unwrap_or_default()))
            .and_then(|()| fs::rename(&tmp, &file));

        if let Err(err) = result {
            debug!("cannot save mbox cache {}: {err}", file.display());
        }
    }
}

/// The cache file of the mbox at `path`, named after a hash of its
/// canonical path so two spellings of one file share it.
fn cache_file(dir: &Path, path: &MboxFsPath) -> PathBuf {
    let path = fs::canonicalize(path.as_str()).unwrap_or_else(|_| PathBuf::from(path.as_str()));
    let hash = Sha256::digest(path.to_string_lossy().as_bytes());
    let name: String = hash[..16].iter().map(|b| format!("{b:02x}")).collect();
    dir.join(format!("{name}.json"))
}

#[cfg(test)]
mod tests {
    use std::{env, process};

    use io_mbox::{entry::MboxEntry, path::MboxFsPath};

    use super::*;

    /// A fresh directory under the system temporary one.
    fn tempdir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("himalaya-mbox-cache-{}-{name}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn envelope(id: &str) -> Envelope {
        Envelope {
            id: id.into(),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: Default::default(),
            subject: format!("subject {id}"),
            from: Vec::new(),
            to: Vec::new(),
            date: None,
            size: 1,
            has_attachment: Some(false),
        }
    }

    #[test]
    fn round_trips_and_prunes_gone_messages() {
        let dir = tempdir("round-trip");
        let path = MboxFsPath::new("/nowhere/mbox");

        let mut cache = MboxCache {
            index: Some(MboxIndex {
                entries: vec![MboxEntry {
                    id: "kept".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            envelopes: BTreeMap::from([
                ("kept".into(), envelope("kept")),
                ("gone".into(), envelope("gone")),
            ]),
        };
        cache.save(Some(dir.as_path()), &path);

        let loaded = MboxCache::load(Some(dir.as_path()), &path);
        assert_eq!(loaded.index, cache.index);
        assert_eq!(loaded.envelopes.keys().collect::<Vec<_>>(), ["kept"]);
    }

    #[test]
    fn missing_or_corrupt_caches_load_empty() {
        let dir = tempdir("corrupt");
        let path = MboxFsPath::new("/nowhere/mbox");
        assert!(MboxCache::load(Some(dir.as_path()), &path).index.is_none());
        assert!(MboxCache::load(None, &path).index.is_none());

        fs::write(cache_file(dir.as_path(), &path), b"not json").unwrap();
        assert!(MboxCache::load(Some(dir.as_path()), &path).index.is_none());
    }
}
