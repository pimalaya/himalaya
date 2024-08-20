pub mod arg;
pub mod command;
pub mod config;

use comfy_table::{Cell, ContentArrangement, Row, Table};
use crossterm::style::Color;
use serde::{Serialize, Serializer};
use std::{fmt, ops::Deref};

use self::config::ListFoldersTableConfig;

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folder {
    pub name: String,
    pub desc: String,
}

impl Folder {
    pub fn to_row(&self, config: &ListFoldersTableConfig) -> Row {
        let mut row = Row::new();
        row.max_height(1);

        row.add_cell(Cell::new(&self.name).fg(config.name_color()));
        row.add_cell(Cell::new(&self.desc).fg(config.desc_color()));

        row
    }
}

impl From<email::folder::Folder> for Folder {
    fn from(folder: email::folder::Folder) -> Self {
        Folder {
            name: folder.name,
            desc: folder.desc,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folders(Vec<Folder>);

impl Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<email::folder::Folders> for Folders {
    fn from(folders: email::folder::Folders) -> Self {
        Folders(folders.into_iter().map(Folder::from).collect())
    }
}

pub struct FoldersTable {
    folders: Folders,
    width: Option<u16>,
    config: ListFoldersTableConfig,
}

impl FoldersTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
        self.config.preset = preset;
        self
    }

    pub fn with_some_name_color(mut self, color: Option<Color>) -> Self {
        self.config.name_color = color;
        self
    }

    pub fn with_some_desc_color(mut self, color: Option<Color>) -> Self {
        self.config.desc_color = color;
        self
    }
}

impl From<Folders> for FoldersTable {
    fn from(folders: Folders) -> Self {
        Self {
            folders,
            width: None,
            config: Default::default(),
        }
    }
}

impl fmt::Display for FoldersTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(self.config.preset())
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new("NAME"), Cell::new("DESC")]))
            .add_rows(
                self.folders
                    .iter()
                    .map(|folder| folder.to_row(&self.config)),
            );

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for FoldersTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.folders.serialize(serializer)
    }
}
