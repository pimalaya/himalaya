use anyhow::Result;
use email::flag::Flags;

use crate::{backend::Backend, printer::Printer};

pub async fn add<P: Printer>(
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    backend.add_flags(folder, &ids, flags).await?;
    printer.print(format!("Flag(s) {flags} successfully added!"))
}

pub async fn set<P: Printer>(
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    backend.set_flags(folder, &ids, flags).await?;
    printer.print(format!("Flag(s) {flags} successfully set!"))
}

pub async fn remove<P: Printer>(
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    backend.remove_flags(folder, &ids, flags).await?;
    printer.print(format!("Flag(s) {flags} successfully removed!"))
}
