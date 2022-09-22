//! Message flag handling module.
//!
//! This module gathers all flag actions triggered by the CLI.

use anyhow::Result;
use himalaya_lib::backend::Backend;

use crate::printer::Printer;

/// Adds flags to all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn add<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq_range: &str,
    flags: &str,
    mbox: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.flags_add(mbox, seq_range, flags)?;
    printer.print_struct(format!(
        "Flag(s) {:?} successfully added to message(s) {:?}",
        flags, seq_range
    ))
}

/// Removes flags from all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn remove<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq_range: &str,
    flags: &str,
    mbox: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.flags_delete(mbox, seq_range, flags)?;
    printer.print_struct(format!(
        "Flag(s) {:?} successfully removed from message(s) {:?}",
        flags, seq_range
    ))
}

/// Replaces flags of all messages matching the given sequence range.
/// Flags are case-insensitive, and they do not need to be prefixed with `\`.
pub fn set<'a, P: Printer, B: Backend<'a> + ?Sized>(
    seq_range: &str,
    flags: &str,
    mbox: &str,
    printer: &mut P,
    backend: &mut B,
) -> Result<()> {
    backend.flags_set(mbox, seq_range, flags)?;
    printer.print_struct(format!(
        "Flag(s) {:?} successfully set for message(s) {:?}",
        flags, seq_range
    ))
}
