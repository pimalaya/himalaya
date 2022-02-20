use std::fmt;

use crate::output::PrintTable;

pub trait Envelopes: fmt::Debug + erased_serde::Serialize + PrintTable {
    //
}
