use chrono::Local;
use env_logger;
use error_chain::error_chain;
use log::{self, Level, LevelFilter, Metadata, Record};
use std::{fmt, io::Write};

use super::fmt::{set_output_fmt, OutputFmt};

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
    match fmt {
        &OutputFmt::Plain => {
            set_output_fmt(&OutputFmt::Plain);
        }
        &OutputFmt::Json => {
            set_output_fmt(&OutputFmt::Json);
        }
    };

    env_logger::Builder::new()
        .format(|buf, record| {
            if let log::Level::Info = record.metadata().level() {
                write!(buf, "{}", record.args())
            } else {
                writeln!(
                    buf,
                    "[{} {:5} {}] {}",
                    Local::now().format("%Y-%m-%dT%H:%M:%S"),
                    record.metadata().level(),
                    record.module_path().unwrap_or_default(),
                    record.args()
                )
            }
        })
        .filter_level(level.0)
        .init();

    Ok(())
}
