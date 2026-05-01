//! Synchronous filesystem driver for io-maildir coroutines.
//!
//! io-maildir is fully I/O-free; coroutines emit `Wants*` requests and
//! the caller is responsible for performing the matching std::fs
//! operation and feeding the resulting per-coroutine `*Arg` variant
//! back via `resume(Some(arg))`.
//!
//! Each helper performs the operation and returns the raw output
//! (only meaningful for read-style ops). Callers wrap the result in
//! the appropriate `*Arg` variant for their coroutine.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

/// Copies each `(source, target)` pair via [`std::fs::copy`].
pub fn copy(pairs: Vec<(String, String)>) -> io::Result<()> {
    for (source, target) in pairs {
        fs::copy(PathBuf::from(source), PathBuf::from(target))?;
    }

    Ok(())
}

/// Creates each directory via [`std::fs::create_dir`].
pub fn dir_create(paths: BTreeSet<String>) -> io::Result<()> {
    for path in paths {
        fs::create_dir(PathBuf::from(path))?;
    }

    Ok(())
}

/// Reads the entries inside each directory via [`std::fs::read_dir`].
pub fn dir_read(paths: BTreeSet<String>) -> io::Result<BTreeMap<String, BTreeSet<String>>> {
    let mut entries = BTreeMap::new();

    for path in paths {
        let mut children = BTreeSet::new();

        for entry in fs::read_dir(PathBuf::from(&path))? {
            let entry = entry?;
            children.insert(entry.path().to_string_lossy().into_owned());
        }

        entries.insert(path, children);
    }

    Ok(entries)
}

/// Removes each directory and all its contents via [`std::fs::remove_dir_all`].
pub fn dir_remove(paths: BTreeSet<String>) -> io::Result<()> {
    for path in paths {
        fs::remove_dir_all(PathBuf::from(path))?;
    }

    Ok(())
}

/// Creates each file with the associated contents.
pub fn file_create(files: BTreeMap<String, Vec<u8>>) -> io::Result<()> {
    for (path, contents) in files {
        let mut file = File::create(PathBuf::from(path))?;
        file.write_all(&contents)?;
    }

    Ok(())
}

/// Reads each file via [`std::fs::read`].
pub fn file_read(paths: BTreeSet<String>) -> io::Result<BTreeMap<String, Vec<u8>>> {
    let mut contents = BTreeMap::new();

    for path in paths {
        let data = fs::read(PathBuf::from(&path))?;
        contents.insert(path, data);
    }

    Ok(contents)
}

/// Renames or moves each `(from, to)` pair via [`std::fs::rename`].
pub fn rename(pairs: Vec<(String, String)>) -> io::Result<()> {
    for (from, to) in pairs {
        fs::rename(PathBuf::from(from), PathBuf::from(to))?;
    }

    Ok(())
}
