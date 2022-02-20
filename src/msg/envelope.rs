use std::fmt;

use crate::output::PrintTable;

pub trait PrintableEnvelopes: fmt::Debug + erased_serde::Serialize + PrintTable {
    //
}

pub type Envelopes = Box<dyn PrintableEnvelopes>;
