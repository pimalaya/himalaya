use anyhow::Result;

pub trait Print {
    fn print(&self) -> Result<()>;

    fn println(&self) -> Result<()> {
        println!();
        self.print()
    }
}

impl Print for &str {
    fn print(&self) -> Result<()> {
        print!("{}", self);
        Ok(())
    }
}

impl Print for String {
    fn print(&self) -> Result<()> {
        self.as_str().print()
    }
}
