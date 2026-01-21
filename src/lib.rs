pub mod account;
pub mod cli;
pub mod completion;
pub mod config;
pub mod email;
pub mod folder;
pub mod manual;

use std::path::PathBuf;

use shellexpand_utils::{canonicalize, expand};

#[doc(inline)]
pub use crate::email::{envelope, flag, message};

/// Parse the given [`str`] as [`PathBuf`].
///
/// The path is first shell expanded, then canonicalized (if
/// applicable).
fn dir_parser(path: &str) -> Result<PathBuf, String> {
    expand::try_path(path)
        .map(canonicalize::path)
        .map_err(|err| err.to_string())
}
