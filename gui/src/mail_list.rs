use anyhow::Result;
use himalaya::output::{Print, PrintTable, PrintTableOpts, PrinterService, WriteColor};
use iui::{
    controls::{Label, LayoutStrategy, VerticalBox},
    UI,
};
use std::fmt::Debug;

#[derive(Debug)]
pub struct MailDesc(Vec<u8>);

pub struct ListPrinter {
    mails: Vec<MailDesc>,
    pub inner: VerticalBox,
}

impl std::fmt::Debug for ListPrinter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListPrinter")
            .field("mails", &self.mails)
            .finish()
    }
}

impl ListPrinter {
    pub fn new(ui: &UI) -> Self {
        let mut vbox = VerticalBox::new(&ui);
        vbox.set_padded(&ui, true);
        ListPrinter {
            mails: Vec::new(),
            inner: vbox,
        }
    }

    pub fn draw(&mut self, ui: &UI) {
        for m in self.mails.iter() {
            let label = Label::new(&ui, std::str::from_utf8(&m.0).unwrap_or_default());
            self.inner
                .append(&ui, label.clone(), LayoutStrategy::Stretchy);
        }
    }
}

impl std::io::Write for MailDesc {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.0.flush()
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.0.write_all(buf)
    }
    fn write_fmt(&mut self, fmt: std::fmt::Arguments) -> Result<(), std::io::Error> {
        self.0.write_fmt(fmt)
    }
    fn by_ref(&mut self) -> &mut Self {
        self
    }
}

impl termcolor::WriteColor for MailDesc {
    fn supports_color(&self) -> bool {
        false
    }
    fn set_color(&mut self, _spec: &termcolor::ColorSpec) -> Result<(), std::io::Error> {
        Ok(())
    }
    fn reset(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
    fn is_synchronous(&self) -> bool {
        true
    }
}

impl WriteColor for MailDesc {}

impl PrinterService for ListPrinter {
    fn print_str<T: Debug + Print>(&mut self, data: T) -> Result<()> {
        let mut m = MailDesc(Vec::new());
        data.print(&mut m)?;
        self.mails.push(m);
        Ok(())
    }

    fn print_struct<T: Debug + Print + serde::Serialize>(&mut self, data: T) -> Result<()> {
        let mut m = MailDesc(Vec::new());
        data.print(&mut m)?;
        self.mails.push(m);
        Ok(())
    }

    fn print_table<T: Debug + erased_serde::Serialize + PrintTable + ?Sized>(
        &mut self,
        data: Box<T>,
        opts: PrintTableOpts,
    ) -> Result<()> {
        let mut m = MailDesc(Vec::new());
        data.print_table(&mut m, opts)?;
        self.mails.push(m);
        Ok(())
    }

    fn is_json(&self) -> bool {
        false
    }
}
