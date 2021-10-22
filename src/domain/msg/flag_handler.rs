//! Message flag handling module.
//!
//! This module gathers all flag actions triggered by the CLI.

use anyhow::Result;

use crate::{
    domain::{Flags, ImapServiceInterface},
    output::OutputServiceInterface,
};

/// Adds flags to all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn add<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    seq_range: &'a str,
    flags: Vec<&'a str>,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.add_flags(seq_range, &flags)?;
    output.print(format!(
        r#"Flag(s) "{}" successfully added to message(s) "{}""#,
        flags, seq_range
    ))
}

/// Removes flags from all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn remove<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    seq_range: &'a str,
    flags: Vec<&'a str>,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.remove_flags(seq_range, &flags)?;
    output.print(format!(
        r#"Flag(s) "{}" successfully removed from message(s) "{}""#,
        flags, seq_range
    ))
}

/// Replaces flags of all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn set<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    seq_range: &'a str,
    flags: Vec<&'a str>,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.set_flags(seq_range, &flags)?;
    output.print(format!(
        r#"Flag(s) "{}" successfully set for message(s) "{}""#,
        flags, seq_range
    ))
}
