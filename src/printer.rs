use color_eyre::{eyre::Context, Result};
use std::{
    fmt,
    io::{self, Write},
};

use crate::output::OutputFmt;

pub trait PrintTable {
    fn print(&self, writer: &mut dyn io::Write, table_max_width: Option<u16>) -> Result<()>;
}

pub trait Printer {
    fn out<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()>;

    fn log<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()> {
        self.out(data)
    }

    fn is_json(&self) -> bool {
        false
    }
}

pub struct StdoutPrinter {
    stdout: io::Stdout,
    stderr: io::Stderr,
    output: OutputFmt,
}

impl StdoutPrinter {
    pub fn new(output: OutputFmt) -> Self {
        Self {
            stdout: io::stdout(),
            stderr: io::stderr(),
            output,
        }
    }
}

impl Default for StdoutPrinter {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Printer for StdoutPrinter {
    fn out<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()> {
        match self.output {
            OutputFmt::Plain => {
                write!(self.stdout, "{data}")?;
            }
            OutputFmt::Json => {
                serde_json::to_writer(&mut self.stdout, &data)
                    .context("cannot write json to writer")?;
            }
        };

        Ok(())
    }

    fn log<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()> {
        if let OutputFmt::Plain = self.output {
            write!(&mut self.stderr, "{data}")?;
        }

        Ok(())
    }

    fn is_json(&self) -> bool {
        self.output == OutputFmt::Json
    }
}
