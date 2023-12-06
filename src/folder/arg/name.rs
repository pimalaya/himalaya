use clap::Parser;
use email::account::config::DEFAULT_INBOX_FOLDER;

/// The folder name argument parser
#[derive(Debug, Parser)]
pub struct FolderNameArg {
    /// The name of the folder
    #[arg(name = "folder-name", value_name = "FOLDER")]
    pub name: String,
}

/// The optional folder name argument parser
#[derive(Debug, Parser)]
pub struct FolderNameOptionalArg {
    /// The name of the folder
    #[arg(name = "folder-name", value_name = "FOLDER", default_value = DEFAULT_INBOX_FOLDER)]
    pub name: String,
}
