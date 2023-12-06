use clap::Parser;

/// The folder name argument parser
#[derive(Debug, Parser)]
pub struct FolderNameArg {
    /// The name of the folder
    #[arg(name = "folder-name", value_name = "FOLDER")]
    pub name: String,
}
