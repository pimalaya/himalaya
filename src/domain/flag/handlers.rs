use anyhow::Result;
use pimalaya_email::{Backend, Flags};

use crate::{printer::Printer, IdMapper};

pub fn add<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    backend.add_flags(folder, ids, flags)?;
    printer.print("Flag(s) successfully added!")
}

pub fn set<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    backend.set_flags(folder, ids, flags)?;
    printer.print("Flag(s) successfully set!")
}

pub fn remove<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &mut dyn Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    let ids = id_mapper.get_ids(ids)?;
    let ids = ids.iter().map(String::as_str).collect::<Vec<_>>();
    backend.remove_flags(folder, ids, flags)?;
    printer.print("Flag(s) successfully removed!")
}
