pub mod arg;
pub mod command;

use color_eyre::Result;
use email::template::Template;

use crate::printer::{Print, WriteColor};

impl Print for Template {
    fn print(&self, writer: &mut dyn WriteColor) -> Result<()> {
        self.as_str().print(writer)?;
        Ok(writer.reset()?)
    }
}
