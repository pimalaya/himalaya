use anyhow::Result;
use himalaya_lib::Backend;

use crate::printer::Printer;

pub fn add<P: Printer, B: Backend + ?Sized>(
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    id: &str,
    flags: &str,
) -> Result<()> {
    backend.add_flags(folder, id, flags)?;
    printer.print_log("Flag(s) successfully added!")
}

pub fn set<P: Printer, B: Backend + ?Sized>(
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    id: &str,
    flags: &str,
) -> Result<()> {
    backend.set_flags(folder, id, flags)?;
    printer.print_log("Flag(s) successfully set!")
}

pub fn remove<P: Printer, B: Backend + ?Sized>(
    printer: &mut P,
    backend: &mut B,
    folder: &str,
    id: &str,
    flags: &str,
) -> Result<()> {
    backend.remove_flags(folder, id, flags)?;
    printer.print_log("Flag(s) successfully removed!")
}
