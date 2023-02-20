use chrono::{DateTime, Local};
use serde::{Serialize, Serializer};

use crate::{
    ui::{Cell, Row, Table},
    Flag, Flags,
};

fn date<S: Serializer>(date: &DateTime<Local>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&date.to_rfc3339())
}

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
    #[serde(serialize_with = "date")]
    pub date: DateTime<Local>,
}

impl From<&himalaya_lib::Envelope> for Envelope {
    fn from(envelope: &himalaya_lib::Envelope) -> Self {
        Envelope {
            id: envelope.id.clone(),
            flags: envelope.flags.clone().into(),
            subject: envelope.subject.clone(),
            from: Mailbox {
                name: envelope.from.name.clone(),
                addr: envelope.from.addr.clone(),
            },
            date: envelope.date.clone(),
        }
    }
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
        let date = self.date.to_rfc3339();

        Row::new()
            .cell(Cell::new(id).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender).bold_if(unseen).blue())
            .cell(Cell::new(date).bold_if(unseen).yellow())
    }
}
