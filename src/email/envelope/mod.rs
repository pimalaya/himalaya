pub mod arg;
pub mod command;
pub mod config;
pub mod flag;

use color_eyre::Result;
use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Row, Table};
use email::account::config::AccountConfig;
use serde::Serialize;
use std::ops;

use crate::{
    cache::IdMapper,
    flag::{Flag, Flags},
    printer::{PrintTable, WriteColor},
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
impl From<Envelope> for Row {
    fn from(envelope: Envelope) -> Self {
        let mut all_attributes = vec![];

        let unseen = !envelope.flags.contains(&Flag::Seen);
        if unseen {
            all_attributes.push(Attribute::Bold)
        }

        let flags = {
            let mut flags = String::new();
            flags.push(if !unseen { ' ' } else { '✷' });
            flags.push(if envelope.flags.contains(&Flag::Answered) {
                '↵'
            } else {
                ' '
            });
            flags.push(if envelope.flags.contains(&Flag::Flagged) {
                '⚑'
            } else {
                ' '
            });
            flags
        };

        let mut row = Row::new();

        row.add_cell(
            Cell::new(envelope.id)
                .add_attributes(all_attributes.clone())
                .fg(Color::Red),
        )
        .add_cell(
            Cell::new(flags)
                .add_attributes(all_attributes.clone())
                .fg(Color::White),
        )
        .add_cell(
            Cell::new(envelope.subject)
                .add_attributes(all_attributes.clone())
                .fg(Color::Green),
        )
        .add_cell(
            Cell::new(if let Some(name) = envelope.from.name {
                name
            } else {
                envelope.from.addr
            })
            .add_attributes(all_attributes.clone())
            .fg(Color::Blue),
        )
        .add_cell(
            Cell::new(envelope.date)
                .add_attributes(all_attributes)
                .fg(Color::Yellow),
        );

        row
    }
}

impl From<&Envelope> for Row {
    fn from(envelope: &Envelope) -> Self {
        let mut all_attributes = vec![];

        let unseen = !envelope.flags.contains(&Flag::Seen);
        if unseen {
            all_attributes.push(Attribute::Bold)
        }

        let flags = {
            let mut flags = String::new();
            flags.push(if !unseen { ' ' } else { '✷' });
            flags.push(if envelope.flags.contains(&Flag::Answered) {
                '↵'
            } else {
                ' '
            });
            flags.push(if envelope.flags.contains(&Flag::Flagged) {
                '⚑'
            } else {
                ' '
            });
            flags
        };

        let mut row = Row::new();

        row.add_cell(
            Cell::new(&envelope.id)
                .add_attributes(all_attributes.clone())
                .fg(Color::Red),
        )
        .add_cell(
            Cell::new(flags)
                .add_attributes(all_attributes.clone())
                .fg(Color::White),
        )
        .add_cell(
            Cell::new(&envelope.subject)
                .add_attributes(all_attributes.clone())
                .fg(Color::Green),
        )
        .add_cell(
            Cell::new(if let Some(name) = &envelope.from.name {
                name
            } else {
                &envelope.from.addr
            })
            .add_attributes(all_attributes.clone())
            .fg(Color::Blue),
        )
        .add_cell(
            Cell::new(&envelope.date)
                .add_attributes(all_attributes)
                .fg(Color::Yellow),
        );

        row
    }
}

/// Represents the list of envelopes.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl From<Envelopes> for Table {
    fn from(envelopes: Envelopes) -> Self {
        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(Row::from([
                Cell::new("ID").add_attribute(Attribute::Reverse),
                Cell::new("FLAGS").add_attribute(Attribute::Reverse),
                Cell::new("SUBJECT").add_attribute(Attribute::Reverse),
                Cell::new("FROM").add_attribute(Attribute::Reverse),
                Cell::new("DATE").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(envelopes.0.into_iter().map(Row::from));

        table
    }
}

impl From<&Envelopes> for Table {
    fn from(envelopes: &Envelopes) -> Self {
        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(Row::from([
                Cell::new("ID").add_attribute(Attribute::Reverse),
                Cell::new("FLAGS").add_attribute(Attribute::Reverse),
                Cell::new("SUBJECT").add_attribute(Attribute::Reverse),
                Cell::new("FROM").add_attribute(Attribute::Reverse),
                Cell::new("DATE").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(envelopes.0.iter().map(Row::from));

        table
    }
}

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
    fn print_table(&self, writer: &mut dyn WriteColor, table_max_width: Option<u16>) -> Result<()> {
        let mut table = Table::from(self);
        if let Some(width) = table_max_width {
            table.set_width(width);
        }
        writeln!(writer)?;
        write!(writer, "{}", table)?;
        writeln!(writer)?;
        Ok(())
    }
}
