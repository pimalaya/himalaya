pub mod arg;
pub mod command;
pub mod config;
pub mod flag;

use color_eyre::Result;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Row, Table};
use crossterm::{cursor, style::Stylize, terminal};
use email::{account::config::AccountConfig, envelope::ThreadedEnvelope};
use petgraph::graphmap::DiGraphMap;
use serde::{Serialize, Serializer};
use std::{collections::HashMap, fmt, ops::Deref, sync::Arc};

use crate::{
    cache::IdMapper,
    flag::{Flag, Flags},
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
    pub has_attachment: bool,
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
                .fg(comfy_table::Color::Red),
        )
        .add_cell(
            Cell::new(flags)
                .add_attributes(all_attributes.clone())
                .fg(comfy_table::Color::White),
        )
        .add_cell(
            Cell::new(envelope.subject)
                .add_attributes(all_attributes.clone())
                .fg(comfy_table::Color::Green),
        )
        .add_cell(
            Cell::new(if let Some(name) = envelope.from.name {
                name
            } else {
                envelope.from.addr
            })
            .add_attributes(all_attributes.clone())
            .fg(comfy_table::Color::Blue),
        )
        .add_cell(
            Cell::new(envelope.date)
                .add_attributes(all_attributes)
                .fg(comfy_table::Color::Yellow),
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

            flags.push(if envelope.has_attachment { '📎' } else { ' ' });

            flags
        };

        let mut row = Row::new();

        row.add_cell(
            Cell::new(&envelope.id)
                .add_attributes(all_attributes.clone())
                .fg(comfy_table::Color::Red),
        )
        .add_cell(
            Cell::new(flags)
                .add_attributes(all_attributes.clone())
                .fg(comfy_table::Color::White),
        )
        .add_cell(
            Cell::new(&envelope.subject)
                .add_attributes(all_attributes.clone())
                .fg(comfy_table::Color::Green),
        )
        .add_cell(
            Cell::new(if let Some(name) = &envelope.from.name {
                name
            } else {
                &envelope.from.addr
            })
            .add_attributes(all_attributes.clone())
            .fg(comfy_table::Color::Blue),
        )
        .add_cell(
            Cell::new(&envelope.date)
                .add_attributes(all_attributes)
                .fg(comfy_table::Color::Yellow),
        );

        row
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl Envelopes {
    pub fn try_from_backend(
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
                    has_attachment: envelope.has_attachment,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Envelopes(envelopes))
    }

    pub fn to_table(&self) -> Table {
        let mut table = Table::new();

        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new("ID").add_attribute(Attribute::Reverse),
                Cell::new("FLAGS").add_attribute(Attribute::Reverse),
                Cell::new("SUBJECT").add_attribute(Attribute::Reverse),
                Cell::new("FROM").add_attribute(Attribute::Reverse),
                Cell::new("DATE").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(self.iter().map(Row::from));

        table
    }
}

impl Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct EnvelopesTable {
    envelopes: Envelopes,
    width: Option<u16>,
}

impl EnvelopesTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }
}

impl From<Envelopes> for EnvelopesTable {
    fn from(envelopes: Envelopes) -> Self {
        Self {
            envelopes,
            width: None,
        }
    }
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = self.envelopes.to_table();

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for EnvelopesTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.envelopes.serialize(serializer)
    }
}

pub struct ThreadedEnvelopes(email::envelope::ThreadedEnvelopes);

impl ThreadedEnvelopes {
    pub fn try_from_backend(
        id_mapper: &IdMapper,
        envelopes: email::envelope::ThreadedEnvelopes,
    ) -> Result<ThreadedEnvelopes> {
        let prev_edges = envelopes
            .graph()
            .all_edges()
            .map(|(a, b, w)| {
                let a = id_mapper.get_or_create_alias(&a.id)?;
                let b = id_mapper.get_or_create_alias(&b.id)?;
                Ok((a, b, *w))
            })
            .collect::<Result<Vec<_>>>()?;

        let envelopes = envelopes
            .map()
            .iter()
            .map(|(_, envelope)| {
                let id = id_mapper.get_or_create_alias(&envelope.id)?;
                let envelope = email::envelope::Envelope {
                    id: id.clone(),
                    message_id: envelope.message_id.clone(),
                    in_reply_to: envelope.in_reply_to.clone(),
                    flags: envelope.flags.clone(),
                    subject: envelope.subject.clone(),
                    from: envelope.from.clone(),
                    to: envelope.to.clone(),
                    date: envelope.date.clone(),
                    has_attachment: envelope.has_attachment,
                };

                Ok((id, envelope))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let envelopes = email::envelope::ThreadedEnvelopes::build(envelopes, move |envelopes| {
            let mut graph = DiGraphMap::<ThreadedEnvelope, u8>::new();

            for (a, b, w) in prev_edges.clone() {
                let eb = envelopes.get(&b).unwrap();
                match envelopes.get(&a) {
                    Some(ea) => {
                        graph.add_edge(ea.as_threaded(), eb.as_threaded(), w);
                    }
                    None => {
                        let ea = ThreadedEnvelope {
                            id: "0",
                            message_id: "0",
                            subject: "",
                            from: "",
                            date: Default::default(),
                        };
                        graph.add_edge(ea, eb.as_threaded(), w);
                    }
                }
            }

            graph
        });

        Ok(ThreadedEnvelopes(envelopes))
    }
}

impl Deref for ThreadedEnvelopes {
    type Target = email::envelope::ThreadedEnvelopes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct EnvelopesTree {
    config: Arc<AccountConfig>,
    envelopes: ThreadedEnvelopes,
}

impl EnvelopesTree {
    pub fn new(config: Arc<AccountConfig>, envelopes: ThreadedEnvelopes) -> Self {
        Self { config, envelopes }
    }

    pub fn fmt(
        f: &mut fmt::Formatter,
        config: &AccountConfig,
        graph: &DiGraphMap<ThreadedEnvelope<'_>, u8>,
        parent: ThreadedEnvelope<'_>,
        pad: String,
        weight: u8,
    ) -> fmt::Result {
        let edges = graph
            .all_edges()
            .filter_map(|(a, b, w)| {
                if a == parent && *w == weight {
                    Some(b)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if parent.id == "0" {
            f.write_str("root")?;
        } else {
            write!(f, "{}{}", parent.id.red(), ") ".dark_grey())?;

            if !parent.subject.is_empty() {
                write!(f, "{} ", parent.subject.green())?;
            }

            if !parent.from.is_empty() {
                let left = "<".dark_grey();
                let right = ">".dark_grey();
                write!(f, "{left}{}{right}", parent.from.blue())?;
            }

            let date = parent.format_date(config);
            let cursor_date_begin_col = terminal::size().unwrap().0 - date.len() as u16;

            let dots =
                "·".repeat((cursor_date_begin_col - cursor::position().unwrap().0 - 2) as usize);
            write!(f, " {} {}", dots.dark_grey(), date.dark_yellow())?;
        }

        writeln!(f)?;

        let edges_count = edges.len();
        for (i, b) in edges.into_iter().enumerate() {
            let is_last = edges_count == i + 1;
            let (x, y) = if is_last {
                (' ', '└')
            } else {
                ('│', '├')
            };

            write!(f, "{pad}{y}─ ")?;

            let pad = format!("{pad}{x}  ");
            Self::fmt(f, config, graph, b, pad, weight + 1)?;
        }

        Ok(())
    }
}

impl fmt::Display for EnvelopesTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        EnvelopesTree::fmt(
            f,
            &self.config,
            self.envelopes.0.graph(),
            ThreadedEnvelope {
                id: "0",
                message_id: "0",
                from: "",
                subject: "",
                date: Default::default(),
            },
            String::new(),
            0,
        )
    }
}

impl Serialize for EnvelopesTree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.envelopes.0.serialize(serializer)
    }
}

impl Deref for EnvelopesTree {
    type Target = ThreadedEnvelopes;

    fn deref(&self) -> &Self::Target {
        &self.envelopes
    }
}
