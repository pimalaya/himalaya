use himalaya_lib::msg::{Envelope, Flag};

use crate::ui::{Cell, Row, Table};

impl Table for Envelope {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("ID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let id = self.id.to_string();
        let flags = self.flags.to_symbols_string();
        let unseen = !self.flags.contains(&Flag::Seen);
        let subject = &self.subject;
        let sender = &self.sender;
        let date = self.date.as_deref().unwrap_or_default();

        Row::new()
            .cell(Cell::new(id).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender).bold_if(unseen).blue())
            .cell(Cell::new(date).bold_if(unseen).yellow())
    }
}
