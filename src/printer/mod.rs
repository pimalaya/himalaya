pub mod print;
#[allow(clippy::module_inception)]
pub mod printer;

use std::io;

pub use print::*;
pub use printer::*;
use termcolor::StandardStream;

pub trait WriteColor: io::Write + termcolor::WriteColor {}

impl WriteColor for StandardStream {}
