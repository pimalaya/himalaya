//! # m2dir flag
//!
//! The `m2dir flag` command family, over the flags an m2dir entry
//! carries.

use anyhow::{Result, bail};
use io_m2dir::flag::M2dirFlags;

pub mod add;
pub mod cli;
pub mod list;
pub mod remove;
pub mod set;

/// Parses raw flags into an [`M2dirFlags`], refusing what the m2dir spec
/// forbids: an empty flag, or one holding a control character, a line
/// break among them, which would write several.
pub fn parse_flags(flags: &[String]) -> Result<M2dirFlags> {
    for flag in flags {
        if flag.is_empty() {
            bail!("An m2dir flag must not be empty");
        }

        if flag.chars().any(char::is_control) {
            bail!(
                "The m2dir flag `{}` must not hold a control character",
                flag.escape_debug()
            );
        }
    }

    Ok(M2dirFlags::from_iter(flags.iter().map(String::as_str)))
}

#[cfg(test)]
mod tests {
    use super::parse_flags;

    #[test]
    fn a_flag_is_one_non_empty_line_without_control_characters() {
        assert!(parse_flags(&["$seen".into(), "custom tag".into()]).is_ok());

        for flag in ["", "a\nb", "a\tb"] {
            assert!(parse_flags(&[flag.into()]).is_err(), "{flag:?}");
        }
    }
}
