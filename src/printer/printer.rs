use anyhow::{Context, Error, Result};
use clap::ArgMatches;
use std::fmt::{self, Debug};
use termcolor::StandardStream;

use crate::{
    output::{args, ColorFmt, OutputFmt},
    printer::{Print, PrintTable, PrintTableOpts, WriteColor},
};

pub trait Printer {
    // TODO: rename end
    fn print<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()>;
    // TODO: rename log
    fn print_log<T: Debug + Print>(&mut self, data: T) -> Result<()>;
    // TODO: rename table
    fn print_table<T: Debug + erased_serde::Serialize + PrintTable + ?Sized>(
        &mut self,
        // TODO: remove Box
        data: Box<T>,
        opts: PrintTableOpts,
    ) -> Result<()>;
    fn is_json(&self) -> bool;
}

pub struct StdoutPrinter {
    pub writer: Box<dyn WriteColor>,
    pub fmt: OutputFmt,
}

impl Default for StdoutPrinter {
    fn default() -> Self {
        let fmt = OutputFmt::default();
        let writer = Box::new(StandardStream::stdout(ColorFmt::default().into()));
        Self { fmt, writer }
    }
}

impl StdoutPrinter {
    pub fn new(fmt: OutputFmt, color: ColorFmt) -> Self {
        let writer = Box::new(StandardStream::stdout(color.into()));
        Self { fmt, writer }
    }
}

impl Printer for StdoutPrinter {
    fn print_log<T: Debug + Print>(&mut self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print(self.writer.as_mut()),
            OutputFmt::Json => Ok(()),
        }
    }

    fn print<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print(self.writer.as_mut()),
            OutputFmt::Json => serde_json::to_writer(self.writer.as_mut(), &data)
                .context("cannot write json to writer"),
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

impl From<OutputFmt> for StdoutPrinter {
    fn from(fmt: OutputFmt) -> Self {
        Self::new(fmt, ColorFmt::Auto)
    }
}

impl TryFrom<&ArgMatches> for StdoutPrinter {
    type Error = Error;

    fn try_from(m: &ArgMatches) -> Result<Self, Self::Error> {
        let fmt: OutputFmt = m
            .get_one::<String>(args::ARG_OUTPUT)
            .map(String::as_str)
            .unwrap()
            .parse()?;

        let color: ColorFmt = m
            .get_one::<String>(args::ARG_COLOR)
            .map(String::as_str)
            .unwrap()
            .parse()?;

        Ok(Self::new(fmt, color))
    }
}
