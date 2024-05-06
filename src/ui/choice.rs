use std::fmt::Display;

use color_eyre::Result;
use inquire::Select;

#[derive(Clone, Debug)]
pub enum PreEditChoice {
    Edit,
    Discard,
    Quit,
}

impl Display for PreEditChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Edit => "Edit it",
                Self::Discard => "Discard it",
                Self::Quit => "Quit",
            }
        )
    }
}

pub fn pre_edit() -> Result<PreEditChoice> {
    let choices = [
        PreEditChoice::Edit,
        PreEditChoice::Discard,
        PreEditChoice::Quit,
    ];

    let user_choice = Select::new(
        "A draft was found, what would you like to do with it?",
        choices.to_vec(),
    )
    .with_starting_cursor(0)
    .with_vim_mode(true)
    .prompt()?;

    Ok(user_choice)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PostEditChoice {
    Send,
    Edit,
    LocalDraft,
    RemoteDraft,
    Discard,
}

impl Display for PostEditChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Send => "Send it",
                Self::Edit => "Edit it again",
                Self::LocalDraft => "Save it as local draft",
                Self::RemoteDraft => "Save it as remote draft",
                Self::Discard => "Discard it",
            }
        )
    }
}

pub fn post_edit() -> Result<PostEditChoice> {
    let choices = [
        PostEditChoice::Send,
        PostEditChoice::Edit,
        PostEditChoice::LocalDraft,
        PostEditChoice::RemoteDraft,
        PostEditChoice::Discard,
    ];

    let user_choice = inquire::Select::new(
        "What would you like to do with this message?",
        choices.to_vec(),
    )
    .with_starting_cursor(0)
    .with_vim_mode(true)
    .prompt()?;

    Ok(user_choice)
}
