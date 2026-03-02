use clap::Parser;
use email::folder::INBOX;

/// The optional folder name flag parser.
#[derive(Debug, Parser)]
pub struct FolderNameOptionalFlag {
    /// The name of the folder.
    #[arg(long = "folder", short = 'f')]
    #[arg(name = "folder_name", value_name = "NAME", default_value = INBOX)]
    pub name: String,
}

impl Default for FolderNameOptionalFlag {
    fn default() -> Self {
        Self {
            name: INBOX.to_owned(),
        }
    }
}

/// The optional folder name argument parser.
#[derive(Debug, Parser)]
pub struct FolderNameOptionalArg {
    /// The name of the folder.
    #[arg(name = "folder_name", value_name = "FOLDER", default_value = INBOX)]
    pub name: String,
}

impl Default for FolderNameOptionalArg {
    fn default() -> Self {
        Self {
            name: INBOX.to_owned(),
        }
    }
}

/// The required folder name argument parser.
#[derive(Debug, Parser)]
pub struct FolderNameArg {
    /// The name of the folder.
    #[arg(name = "folder_name", value_name = "FOLDER")]
    pub name: String,
}

/// The optional source folder name flag parser.
#[derive(Debug, Parser)]
pub struct SourceFolderNameOptionalFlag {
    /// The name of the source folder.
    #[arg(long = "folder", short = 'f')]
    #[arg(name = "source_folder_name", value_name = "SOURCE", default_value = INBOX)]
    pub name: String,
}

/// The target folder name argument parser.
#[derive(Debug, Parser)]
pub struct TargetFolderNameArg {
    /// The name of the target folder.
    #[arg(name = "target_folder_name", value_name = "TARGET")]
    pub name: String,
}
