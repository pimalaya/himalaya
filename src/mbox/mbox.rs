use std::fmt;

use crate::output::PrintTable;

pub trait PrintableMboxes: fmt::Debug + erased_serde::Serialize + PrintTable {}

pub type Mboxes = Box<dyn PrintableMboxes>;
