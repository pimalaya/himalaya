pub mod arg;
pub mod command;
pub mod config;
pub mod flag;

use color_eyre::Result;
use email::account::config::AccountConfig;
use serde::Serialize;
use std::ops;

use crate::{
    cache::IdMapper,
    flag::{Flag, Flags},
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Mailbox {
    pub name: Option<String>,
    pub addr: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelope {
    pub id: String,
    pub flags: Flags,
    pub subject: String,
    pub from: Mailbox,
    pub to: Mailbox,
    pub date: String,
}

impl Table for Envelope {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("ID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("FROM").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let id = self.id.to_string();
        let unseen = !self.flags.contains(&Flag::Seen);
        let flags = {
            let mut flags = String::new();
            flags.push_str(if !unseen { " " } else { "✷" });
            flags.push_str(if self.flags.contains(&Flag::Answered) {
                "↵"
            } else {
                " "
            });
            flags.push_str(if self.flags.contains(&Flag::Flagged) {
                "⚑"
            } else {
                " "
            });
            flags
        };
        let subject = &self.subject;
        let sender = if let Some(name) = &self.from.name {
            name
        } else {
            &self.from.addr
        };
        let date = &self.date;

        Row::new()
            .cell(Cell::new(id).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender).bold_if(unseen).blue())
            .cell(Cell::new(date).bold_if(unseen).yellow())
    }
}

/// Represents the list of envelopes.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl Envelopes {
    pub fn from_backend(
        config: &AccountConfig,
        id_mapper: &IdMapper,
        envelopes: email::envelope::Envelopes,
    ) -> Result<Envelopes> {
        let envelopes = envelopes
            .iter()
            .map(|envelope| {
                Ok(Envelope {
                    id: id_mapper.get_or_create_alias(&envelope.id)?,
                    flags: envelope.flags.clone().into(),
                    subject: envelope.subject.clone(),
                    from: Mailbox {
                        name: envelope.from.name.clone(),
                        addr: envelope.from.addr.clone(),
                    },
                    to: Mailbox {
                        name: envelope.to.name.clone(),
                        addr: envelope.to.addr.clone(),
                    },
                    date: envelope.format_date(config),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Envelopes(envelopes))
    }
}

impl ops::Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PrintTable for Envelopes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}
