use error_chain::error_chain;
use log::{Level, LevelFilter, Metadata, Record};
use std::fmt;

use super::fmt::OutputFmt;

error_chain! {}

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

struct PlainLogger;

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

struct JsonLogger;

impl log::Log for JsonLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        [Level::Error, Level::Info].contains(&metadata.level())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Level::Error = record.level() {
                eprintln!("{}", record.args());
            } else {
                // Should be safe enough to `.unwrap()` since it's
                // formatted by Himalaya itself
                print!("{}", serde_json::to_string(record.args()).unwrap());
            }
        }
    }

    fn flush(&self) {}
}

// Init

pub fn init(fmt: &OutputFmt, level: &LogLevel) -> Result<()> {
    log::set_boxed_logger(match fmt {
        &OutputFmt::Json => Box::new(JsonLogger),
        &OutputFmt::Plain => Box::new(PlainLogger),
    })
    .map(|()| log::set_max_level(level.0))
    .chain_err(|| "Could not init logger")
}
