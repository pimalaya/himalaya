use anyhow::{Context, Result};
use atty::Stream;
use std::fmt::{self, Debug};
use termcolor::{ColorChoice, StandardStream};

use crate::{
    output::OutputFmt,
    printer::{Print, PrintTable, PrintTableOpts, WriteColor},
};

pub trait Printer {
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

impl StdoutPrinter {
    pub fn from_fmt(fmt: OutputFmt) -> Self {
        let writer = StandardStream::stdout(if atty::isnt(Stream::Stdin) {
            // Colors should be deactivated if the terminal is not a tty.
            ColorChoice::Never
        } else {
            // Otherwise let's `termcolor` decide by inspecting the environment. From the [doc]:
            // - If `NO_COLOR` is set to any value, then colors will be suppressed.
            // - If `TERM` is set to dumb, then colors will be suppressed.
            // - In non-Windows environments, if `TERM` is not set, then colors will be suppressed.
            //
            // [doc]: https://github.com/BurntSushi/termcolor#automatic-color-selection
            ColorChoice::Auto
        });
        let writer = Box::new(writer);
        Self { writer, fmt }
    }

    pub fn from_opt_str(s: Option<&str>) -> Result<Self> {
        Ok(Self {
            fmt: OutputFmt::try_from(s)?,
            ..Self::from_fmt(OutputFmt::Plain)
        })
    }
}

impl Printer for StdoutPrinter {
    fn print_str<T: Debug + Print>(&mut self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print(self.writer.as_mut()),
            OutputFmt::Json => Ok(()),
        }
    }

    fn print_struct<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()> {
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
