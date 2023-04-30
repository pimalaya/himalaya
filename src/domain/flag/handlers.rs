use anyhow::Result;
use pimalaya_email::{Backend, Flags};

use crate::printer::Printer;

pub fn add<P: Printer, B: Backend + ?Sized>(
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    backend.add_flags(folder, ids, flags)?;
    printer.print("Flag(s) successfully added!")
}

pub fn set<P: Printer, B: Backend + ?Sized>(
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    backend.set_flags(folder, ids, flags)?;
    printer.print("Flag(s) successfully set!")
}

pub fn remove<P: Printer, B: Backend + ?Sized>(
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    backend.remove_flags(folder, ids, flags)?;
    printer.print("Flag(s) successfully removed!")
}
