use error_chain::error_chain;
use log::{self, Level, LevelFilter, Metadata, Record};
use std::fmt;

use super::fmt::{set_output_fmt, OutputFmt};

error_chain! {}

// Macros

#[macro_export]
macro_rules! info {
    ($t:expr) => {
        use crate::output::fmt::{get_output_fmt, OutputFmt};
        use log::info as log_info;
        unsafe {
            match get_output_fmt() {
                OutputFmt::Plain => log_info!("{}", $t.to_string()),
                OutputFmt::Json => {
                    // Should be safe enough to `.unwrap()` since it's
                    // formatted by Himalaya itself
                    log_info!("{{\"response\":{}}}", serde_json::to_string($t).unwrap())
                }
            };
        }
    };
}

// Log level struct

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
        write!(f, "{}", self.0.to_string())
    }
}

// Plain logger

pub struct PlainLogger;

impl log::Log for PlainLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Level::Error = record.level() {
                eprintln!("{}", record.args());
            } else {
                println!("{}", record.args());
            }
        }
    }

    fn flush(&self) {}
}

// JSON logger

pub struct JsonLogger;

impl log::Log for JsonLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        [Level::Error, Level::Info].contains(&metadata.level())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Level::Error = record.level() {
                eprintln!("{}", record.args());
            } else {
                print!("{}", record.args());
            }
        }
    }

    fn flush(&self) {}
}

// Init

pub fn init(fmt: &OutputFmt, level: &LogLevel) -> Result<()> {
    log::set_logger(match fmt {
        &OutputFmt::Plain => {
            set_output_fmt(&OutputFmt::Plain);
            &PlainLogger
        }
        &OutputFmt::Json => {
            set_output_fmt(&OutputFmt::Json);
            &JsonLogger
        }
    })
    .map(|()| log::set_max_level(level.0))
    .chain_err(|| "Could not init logger")
}
