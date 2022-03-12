use anyhow::{Context, Result};

use crate::output::WriteColor;

pub trait Print {
    fn print(&self, writer: &mut dyn WriteColor) -> Result<()>;
}

impl Print for &str {
    fn print(&self, writer: &mut dyn WriteColor) -> Result<()> {
        writeln!(writer, "{}", self).context("cannot write string to writer")
    }
}

impl Print for String {
    fn print(&self, writer: &mut dyn WriteColor) -> Result<()> {
        self.as_str().print(writer)
    }
}
