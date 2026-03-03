use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::list::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{config::ImapConfig, imap::stream};

/// List all mailboxes.
///
/// This command allows you to list all exsting mailboxes from your
/// IMAP account.
#[derive(Debug, Parser)]
pub struct ListMailboxesCommand {
    // /// The maximum width the table should not exceed.
    // ///
    // /// This argument will force the table not to exceed the given
    // /// width, in pixels. Columns may shrink with ellipsis in order to
    // /// fit the width.
    // #[arg(long = "max-width", short = 'w')]
    // #[arg(name = "table_max_width", value_name = "PIXELS")]
    // pub table_max_width: Option<u16>,
}

impl ListMailboxesCommand {
    pub fn execute(self, _printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mut arg = None;
        let mut coroutine = ImapList::new(context, "".try_into().unwrap(), "*".try_into().unwrap());

        let mailboxes = loop {
            match coroutine.resume(arg.take()) {
                ImapListResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                ImapListResult::Ok { mailboxes, .. } => break mailboxes,
                ImapListResult::Err { err, .. } => bail!(err),
            }
        };

        println!("mailboxes: {mailboxes:#?}");

        // TODO: list mailboxs

        // let mailboxs = Mailboxs::from(backend.list_mailboxs().await?);
        // let table = MailboxsTable::from(mailboxs)
        //     .with_some_width(self.table_max_width)
        //     .with_some_preset(toml_account_config.mailbox_list_table_preset())
        //     .with_some_name_color(toml_account_config.mailbox_list_table_name_color())
        //     .with_some_desc_color(toml_account_config.mailbox_list_table_desc_color());

        // printer.out(table)?;
        Ok(())
    }
}

// #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "kebab-case")]
// pub struct ListMailboxesTableConfig {
//     pub preset: Option<String>,
//     pub name_color: Option<Color>,
//     pub desc_color: Option<Color>,
// }

// impl ListMailboxesTableConfig {
//     pub fn preset(&self) -> &str {
//         self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
//     }

//     pub fn name_color(&self) -> comfy_table::Color {
//         map_color(self.name_color.unwrap_or(Color::Blue))
//     }

//     pub fn desc_color(&self) -> comfy_table::Color {
//         map_color(self.desc_color.unwrap_or(Color::Green))
//     }
// }

// pub struct MailboxesTable {
//     mailboxes: Vec<Mailbox<'static>>,
//     width: Option<u16>,
//     config: ListMailboxesTableConfig,
// }

// impl MailboxesTable {
//     pub fn with_some_width(mut self, width: Option<u16>) -> Self {
//         self.width = width;
//         self
//     }

//     pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
//         self.config.preset = preset;
//         self
//     }

//     pub fn with_some_name_color(mut self, color: Option<Color>) -> Self {
//         self.config.name_color = color;
//         self
//     }

//     pub fn with_some_desc_color(mut self, color: Option<Color>) -> Self {
//         self.config.desc_color = color;
//         self
//     }
// }

// impl From<Mailboxes> for MailboxesTable {
//     fn from(mailboxes: Mailboxes) -> Self {
//         Self {
//             mailboxes,
//             width: None,
//             config: Default::default(),
//         }
//     }
// }

// impl fmt::Display for MailboxesTable {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let mut table = Table::new();

//         table
//             .load_preset(self.config.preset())
//             .set_content_arrangement(ContentArrangement::DynamicFullWidth)
//             .set_header(Row::from([Cell::new("NAME"), Cell::new("DESC")]))
//             .add_rows(
//                 self.mailboxes
//                     .iter()
//                     .map(|mailbox| mailbox.to_row(&self.config)),
//             );

//         if let Some(width) = self.width {
//             table.set_width(width);
//         }

//         writeln!(f)?;
//         write!(f, "{table}")?;
//         writeln!(f)?;
//         Ok(())
//     }
// }

// impl Serialize for MailboxesTable {
//     fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
//         self.mailboxes.serialize(serializer)
//     }
// }
