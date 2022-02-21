use std::{any, fmt};

use crate::output::PrintTable;

pub trait Envelopes: fmt::Debug + erased_serde::Serialize + PrintTable + any::Any {
    fn as_any(&self) -> &dyn any::Any;
}

impl<T: fmt::Debug + erased_serde::Serialize + PrintTable + any::Any> Envelopes for T {
    fn as_any(&self) -> &dyn any::Any {
        self
    }
}
