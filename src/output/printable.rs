use anyhow::{Context, Result};
use std::io::{self, Write};

pub trait Printable {
    fn print(&self) -> Result<()>;

    fn println(&self) -> Result<()> {
        writeln!(&mut io::stdout(), "")?;
        self.print()
    }
}

impl Printable for &str {
    fn print(&self) -> Result<()> {
        write!(&mut io::stdout(), "{}", self).context(format!(r#"cannot print string "{}""#, self))
    }
}

impl Printable for String {
    fn print(&self) -> Result<()> {
        self.as_str().print()
    }
}
