use std::fmt;

use crate::output::PrintTable;

pub trait Mboxes: fmt::Debug + erased_serde::Serialize + PrintTable {
    //
}
