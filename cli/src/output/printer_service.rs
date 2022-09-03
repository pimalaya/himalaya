use anyhow::{Context, Result};
use std::fmt::{self, Debug};
use termcolor::StandardStream;

use crate::output::{OutputFmt, OutputJson, Print, PrintTable, PrintTableOpts, WriteColor};

use super::ColorFmt;

pub trait PrinterService {
    fn print_str<T: Debug + Print>(&mut self, data: T) -> Result<()>;
    fn print_struct<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()>;
    fn print_table<T: Debug + erased_serde::Serialize + PrintTable + ?Sized>(
        &mut self,
        data: Box<T>,
        opts: PrintTableOpts,
    ) -> Result<()>;
    fn is_json(&self) -> bool;
}

pub struct StdoutPrinter {
    pub writer: Box<dyn WriteColor>,
    pub fmt: OutputFmt,
}

impl PrinterService for StdoutPrinter {
    fn print_str<T: Debug + Print>(&mut self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print(self.writer.as_mut()),
            OutputFmt::Json => Ok(()),
        }
    }

    fn print_struct<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print(self.writer.as_mut()),
            OutputFmt::Json => serde_json::to_writer(self.writer.as_mut(), &OutputJson::new(data))
                .context("cannot write JSON to writer"),
        }
    }

    fn print_table<T: fmt::Debug + erased_serde::Serialize + PrintTable + ?Sized>(
        &mut self,
        data: Box<T>,
        opts: PrintTableOpts,
    ) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print_table(self.writer.as_mut(), opts),
            OutputFmt::Json => {
                let json = &mut serde_json::Serializer::new(self.writer.as_mut());
                let ser = &mut <dyn erased_serde::Serializer>::erase(json);
                data.erased_serialize(ser).unwrap();
                Ok(())
            }
        }
    }

    fn is_json(&self) -> bool {
        self.fmt == OutputFmt::Json
    }
}

impl StdoutPrinter {
    pub fn new(fmt: OutputFmt, color: ColorFmt) -> Self {
        let writer = Box::new(StandardStream::stdout(color.into()));
        Self { fmt, writer }
    }
}

impl From<OutputFmt> for StdoutPrinter {
    fn from(fmt: OutputFmt) -> Self {
        Self::new(fmt, ColorFmt::Auto)
    }
}
