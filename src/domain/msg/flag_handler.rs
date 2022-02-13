//! Message flag handling module.
//!
//! This module gathers all flag actions triggered by the CLI.

use anyhow::Result;

use crate::{
    domain::{Backend, Flags},
    output::PrinterService,
};

/// Adds flags to all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn add<'a, Printer: PrinterService, ImapService: Backend<'a>>(
    seq_range: &'a str,
    flags: Vec<&'a str>,
    printer: &'a mut Printer,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.add_flags(seq_range, &flags)?;
    printer.print(format!(
        r#"Flag(s) "{}" successfully added to message(s) "{}""#,
        flags, seq_range
    ))
}

/// Removes flags from all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn remove<'a, Printer: PrinterService, ImapService: Backend<'a>>(
    seq_range: &'a str,
    flags: Vec<&'a str>,
    printer: &'a mut Printer,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.remove_flags(seq_range, &flags)?;
    printer.print(format!(
        r#"Flag(s) "{}" successfully removed from message(s) "{}""#,
        flags, seq_range
    ))
}

/// Replaces flags of all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn set<'a, Printer: PrinterService, ImapService: Backend<'a>>(
    seq_range: &'a str,
    flags: Vec<&'a str>,
    printer: &'a mut Printer,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.set_flags(seq_range, &flags)?;
    printer.print(format!(
        r#"Flag(s) "{}" successfully set for message(s) "{}""#,
        flags, seq_range
    ))
}
