//! Message flag handling module.
//!
//! This module gathers all flag actions triggered by the CLI.

use anyhow::Result;

use crate::{backends::Backend, output::PrinterService};

/// Adds flags to all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn add<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq_range: &'a str,
    flags: &'a str,
    mbox: &'a str,
    printer: &'a mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    backend.add_flags(mbox, seq_range, flags)?;
    printer.print(format!(
        "Flag(s) {:?} successfully added to message(s) {:?}",
        flags, seq_range
    ))
}

/// Removes flags from all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn remove<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq_range: &'a str,
    flags: &'a str,
    mbox: &'a str,
    printer: &'a mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    backend.del_flags(mbox, seq_range, flags)?;
    printer.print(format!(
        "Flag(s) {:?} successfully removed from message(s) {:?}",
        flags, seq_range
    ))
}

/// Replaces flags of all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn set<'a, P: PrinterService, B: Backend<'a> + ?Sized>(
    seq_range: &'a str,
    flags: &'a str,
    mbox: &'a str,
    printer: &'a mut P,
    backend: Box<&'a mut B>,
) -> Result<()> {
    backend.set_flags(mbox, seq_range, flags)?;
    printer.print(format!(
        "Flag(s) {:?} successfully set for message(s) {:?}",
        flags, seq_range
    ))
}
