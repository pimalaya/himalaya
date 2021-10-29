use anyhow::{Context, Result};
use log::error;

use crate::output::WriteColor;

pub trait Print {
    fn print(&self, writter: &mut dyn WriteColor) -> Result<()>;
}

impl Print for &str {
    fn print(&self, writter: &mut dyn WriteColor) -> Result<()> {
        writeln!(writter, "{}", self).with_context(|| {
            error!(r#"cannot write string to writter: "{}""#, self);
            "cannot write string to writter"
        })
    }
}

impl Print for String {
    fn print(&self, writter: &mut dyn WriteColor) -> Result<()> {
        self.as_str().print(writter)
    }
}
