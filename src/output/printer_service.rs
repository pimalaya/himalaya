use anyhow::{Context, Error, Result};
use atty::Stream;
use std::{
    convert::TryFrom,
    fmt::{self, Debug},
};
use termcolor::{ColorChoice, StandardStream};

use crate::output::{OutputFmt, OutputJson, Print, PrintTable, PrintTableOpts, WriteColor};

pub trait PrinterService {
    fn print<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()>;
    fn print_table<T: fmt::Debug + erased_serde::Serialize + PrintTable + ?Sized>(
        &mut self,
        data: Box<T>,
        opts: PrintTableOpts,
    ) -> Result<()>;
    fn is_json(&self) -> bool;
}

pub struct StdoutPrinter {
    pub writter: Box<dyn WriteColor>,
    pub fmt: OutputFmt,
}

impl PrinterService for StdoutPrinter {
    fn print<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print(self.writter.as_mut()),
            OutputFmt::Json => serde_json::to_writer(self.writter.as_mut(), &OutputJson::new(data))
                .context("cannot write JSON to writter"),
        }
    }

    fn print_table<T: fmt::Debug + erased_serde::Serialize + PrintTable + ?Sized>(
        &mut self,
        data: Box<T>,
        opts: PrintTableOpts,
    ) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => data.print_table(self.writter.as_mut(), opts),
            OutputFmt::Json => {
                let json = &mut serde_json::Serializer::new(self.writter.as_mut());
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
        let writter = StandardStream::stdout(if atty::isnt(Stream::Stdin) {
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
        let writter = Box::new(writter);
        Self { writter, fmt }
    }
}

impl TryFrom<Option<&str>> for StdoutPrinter {
    type Error = Error;

    fn try_from(fmt: Option<&str>) -> Result<Self> {
        Ok(Self {
            fmt: OutputFmt::try_from(fmt)?,
            ..Self::from(OutputFmt::Plain)
        })
    }
}
