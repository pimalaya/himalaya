use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use io_msgraph::v1::rest::users::messages::{MsgraphImportance, MsgraphMessage};
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Update a Microsoft Graph message (`PATCH /me/messages/{id}`): mark
/// read/unread, set importance or replace categories.
#[derive(Debug, Parser)]
pub struct MsgraphMessageUpdateCommand {
    /// The id of the message to update.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mark the message as read.
    #[arg(long, conflicts_with = "unread")]
    pub read: bool,

    /// Mark the message as unread.
    #[arg(long)]
    pub unread: bool,

    /// Set the message importance.
    #[arg(long, value_enum, value_name = "LEVEL")]
    pub importance: Option<ImportanceArg>,

    /// Category to set on the message. Can be repeated; replaces the
    /// existing categories.
    #[arg(long = "category", value_name = "NAME")]
    pub categories: Vec<String>,
}

impl MsgraphMessageUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        if !self.read && !self.unread && self.importance.is_none() && self.categories.is_empty() {
            bail!(
                "Nothing to update: pass at least one of \
		 --read, --unread, --importance or --category"
            );
        }

        let mut patch = MsgraphMessage::default();

        if self.read {
            patch.is_read = Some(true);
        }

        if self.unread {
            patch.is_read = Some(false);
        }

        if let Some(importance) = self.importance {
            patch.importance = Some(importance.into());
        }

        patch.categories = self.categories;

        let message = client.message_update(&self.id, &patch)?.response;

        printer.out(Message::new(format!(
            "Microsoft Graph message `{}` successfully updated",
            message.id
        )))
    }
}

/// Message importance requested by `messages update`.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ImportanceArg {
    Low,
    Normal,
    High,
}

impl From<ImportanceArg> for MsgraphImportance {
    fn from(arg: ImportanceArg) -> Self {
        match arg {
            ImportanceArg::Low => MsgraphImportance::Low,
            ImportanceArg::Normal => MsgraphImportance::Normal,
            ImportanceArg::High => MsgraphImportance::High,
        }
    }
}
