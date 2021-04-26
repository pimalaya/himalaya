use chrono::Local;
use env_logger;
use error_chain::error_chain;
use log::{self, debug, Level, LevelFilter};
use std::{fmt, io, io::Write, ops::Deref};

use super::fmt::{set_output_fmt, OutputFmt};

error_chain! {}

// Log level wrapper

pub struct LogLevel(pub LevelFilter);

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s {
            "error" => Self(LevelFilter::Error),
            "warn" => Self(LevelFilter::Warn),
            "debug" => Self(LevelFilter::Debug),
            "trace" => Self(LevelFilter::Trace),
            "info" | _ => Self(LevelFilter::Info),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}

impl Deref for LogLevel {
    type Target = LevelFilter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Init

pub fn init(fmt: OutputFmt, filter: LogLevel) -> Result<()> {
    let level_filter = filter.deref();
    let level = level_filter.to_level();

    match fmt {
        OutputFmt::Plain => {
            set_output_fmt(&OutputFmt::Plain);
        }
        OutputFmt::Json => {
            set_output_fmt(&OutputFmt::Json);
        }
    };

    env_logger::Builder::new()
        .target(env_logger::Target::Stdout)
        .format(move |buf, record| match level {
            None => Ok(()),
            Some(Level::Info) => match record.metadata().level() {
                Level::Info => write!(buf, "{}", record.args()),
                Level::Error => writeln!(&mut io::stderr(), "{}", record.args()),
                _ => writeln!(buf, "{}", record.args()),
            },
            _ => {
                writeln!(
                    buf,
                    "[{} {:5} {}] {}",
                    Local::now().format("%Y-%m-%dT%H:%M:%S"),
                    record.metadata().level(),
                    record.module_path().unwrap_or_default(),
                    record.args(),
                )
            }
        })
        .filter_level(*level_filter)
        .init();

    debug!("output format: {}", fmt);
    debug!("log level: {}", filter);

    Ok(())
}
