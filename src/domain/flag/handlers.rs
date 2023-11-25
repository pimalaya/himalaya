use anyhow::Result;
use email::email::{envelope::Id, Flags};

use crate::{backend::Backend, printer::Printer, IdMapper};

pub async fn add<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    let ids = Id::multiple(id_mapper.get_ids(ids)?);
    backend.add_flags(folder, &ids, flags).await?;
    printer.print("Flag(s) successfully added!")
}

pub async fn set<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    let ids = Id::multiple(id_mapper.get_ids(ids)?);
    backend.set_flags(folder, &ids, flags).await?;
    printer.print("Flag(s) successfully set!")
}

pub async fn remove<P: Printer>(
    printer: &mut P,
    id_mapper: &IdMapper,
    backend: &Backend,
    folder: &str,
    ids: Vec<&str>,
    flags: &Flags,
) -> Result<()> {
    let ids = Id::multiple(id_mapper.get_ids(ids)?);
    backend.remove_flags(folder, &ids, flags).await?;
    printer.print("Flag(s) successfully removed!")
}
