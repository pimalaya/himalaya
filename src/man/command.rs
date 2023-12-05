use anyhow::Result;
use clap::Parser;
use shellexpand_utils::{canonicalize, expand};
use std::path::PathBuf;

/// Generate all man pages to the given directory
#[derive(Debug, Parser)]
pub struct Generate {
    /// Directory where man files should be generated in
    #[arg(value_parser = dir_parser)]
    pub dir: PathBuf,
}

/// Parse the given [`str`] as [`PathBuf`].
///
/// The path is first shell expanded, then canonicalized (if
/// applicable).
fn dir_parser(path: &str) -> Result<PathBuf, String> {
    expand::try_path(path)
        .map(canonicalize::path)
        .map_err(|err| err.to_string())
}
