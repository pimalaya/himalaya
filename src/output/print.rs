use anyhow::{Context, Result};
use std::io;
use termcolor::{StandardStream, WriteColor};

pub trait WriteWithColor: io::Write + WriteColor {}

impl WriteWithColor for StandardStream {}

pub trait Print {
    fn print<W: WriteWithColor>(&self, writter: &mut W) -> Result<()>;

    fn println<W: WriteWithColor>(&self, writter: &mut W) -> Result<()> {
        println!();
        self.print(writter)
    }
}

impl Print for &str {
    fn print<W: WriteWithColor>(&self, writter: &mut W) -> Result<()> {
        write!(writter, "{}", self).context(format!(r#"cannot print string "{}""#, self))
    }
}

impl Print for String {
    fn print<W: WriteWithColor>(&self, writter: &mut W) -> Result<()> {
        self.as_str().print(writter)
    }
}
