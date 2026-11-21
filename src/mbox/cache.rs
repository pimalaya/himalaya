//! # mbox cache
//!
//! What Himalaya keeps between runs about an mbox file, so a listing does
//! not read the whole file again: the io-mbox scan index, and the
//! envelope of every message already read.
//!
//! The index tells an unchanged file from one a delivery appended to, and
//! the envelopes, keyed by content id, stay valid whatever the flags. It
//! is an optimisation only: a missing, stale or unreadable cache is
//! rebuilt, and failing to write one is not an error.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use io_mbox::{index::MboxIndex, path::MboxFsPath};
use log::debug;
use pimalaya_cli::file::private_options;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::unified::email::envelope::Envelope;

/// The cache format version, raised when a message would now read or scan
/// differently, so an older cache is rebuilt.
const VERSION: u32 = 4;

/// The cached state of one mbox file.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MboxCache {
    /// The `VERSION` the cache was saved under, 0 before there was one.
    #[serde(default)]
    pub version: u32,
    /// The scan index, `None` until the file was scanned once.
    pub index: Option<MboxIndex>,
    /// Envelopes by content id, flags empty as the index carries them;
    /// `None` for a message holding no header field.
    pub envelopes: BTreeMap<String, Option<Envelope>>,
}

impl MboxCache {
    /// Loads the cache of the mbox at `path` from `dir`, an empty one when
    /// there is none or it cannot be read.
    pub fn load(dir: Option<&Path>, path: &MboxFsPath) -> Self {
        let Some(file) = dir.map(|dir| cache_file(dir, path)) else {
            return Self::default();
        };

        let cache: Result<Self, String> = fs::read(&file)
            .map_err(|err| err.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|err| err.to_string()));

        match cache {
            Ok(cache) if cache.version == VERSION => cache,
            Ok(cache) => {
                debug!(
                    "dropping mbox cache {}: version {}",
                    file.display(),
                    cache.version
                );
                Self::default()
            }
            Err(err) => {
                debug!("cannot load mbox cache {}: {err}", file.display());
                Self::default()
            }
        }
    }

    /// Saves the cache of `path` under `dir`, dropping the envelopes of
    /// messages the index no longer holds.
    pub fn save(&mut self, dir: Option<&Path>, path: &MboxFsPath) {
        let Some(dir) = dir else {
            return;
        };

        if let Some(index) = &self.index {
            let ids: HashSet<&str> = index.entries.iter().map(|e| e.id.as_str()).collect();
            self.envelopes.retain(|id, _| ids.contains(id.as_str()));
        }

        self.version = VERSION;

        let file = cache_file(dir, path);
        let tmp = file.with_extension("json.tmp");
        let result = create_private_dir(dir)
            .and_then(|()| write_private(&tmp, &serde_json::to_vec(self).unwrap_or_default()))
            .and_then(|()| fs::rename(&tmp, &file));

        if let Err(err) = result {
            debug!("cannot save mbox cache {}: {err}", file.display());
        }
    }
}

/// Creates `dir` and its parents owner-only, tightening an open `dir`: the
/// cache holds every message's subject, sender, recipients and Message-ID.
fn create_private_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
    }

    #[cfg(not(unix))]
    fs::create_dir_all(dir)
}

/// Writes `bytes` to a new file at `path`, readable by the owner alone,
/// replacing whatever file was there.
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let _ = fs::remove_file(path);
    private_options()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)
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
            subject: Some(format!("subject {id}")),
            from: Vec::new(),
            to: Vec::new(),
            date: None,
            size: Some(1),
            has_attachment: Some(false),
        }
    }

    #[test]
    fn a_cache_of_an_older_version_is_rebuilt() {
        let dir = tempdir("older");
        let path = MboxFsPath::new("/nowhere/mbox");

        let mut cache = MboxCache {
            envelopes: BTreeMap::from([("kept".to_string(), Some(envelope("kept")))]),
            ..Default::default()
        };
        cache.save(Some(&dir), &path);

        let file = cache_file(&dir, &path);
        let mut older: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        older.as_object_mut().unwrap().remove("version");
        fs::write(&file, serde_json::to_vec(&older).unwrap()).unwrap();

        assert!(MboxCache::load(Some(&dir), &path).envelopes.is_empty());
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
                ("kept".into(), Some(envelope("kept"))),
                ("gone".into(), Some(envelope("gone"))),
            ]),
            ..Default::default()
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

    /// The cache directory and files are the owner's alone whatever the
    /// umask, a directory an older run left open tightened.
    #[cfg(unix)]
    #[test]
    fn a_cache_is_private_to_its_owner() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir("private").join("mbox");
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        let path = MboxFsPath::new("/nowhere/mbox");

        let mut cache = MboxCache {
            envelopes: BTreeMap::from([("kept".to_string(), Some(envelope("kept")))]),
            ..Default::default()
        };
        cache.save(Some(&dir), &path);
        cache.save(Some(&dir), &path);

        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&cache_file(&dir, &path)), 0o600);
    }
}
