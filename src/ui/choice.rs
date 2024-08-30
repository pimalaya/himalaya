use std::fmt;

use color_eyre::Result;
use pimalaya_tui::prompt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreEditChoice {
    Edit,
    Discard,
    Quit,
}

impl fmt::Display for PreEditChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

static PRE_EDIT_CHOICES: [PreEditChoice; 3] = [
    PreEditChoice::Edit,
    PreEditChoice::Discard,
    PreEditChoice::Quit,
];

pub fn pre_edit() -> Result<PreEditChoice> {
    let user_choice = prompt::item(
        "A draft was found, what would you like to do with it?",
        &PRE_EDIT_CHOICES,
        None,
    )?;

    Ok(user_choice.clone())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PostEditChoice {
    Send,
    Edit,
    LocalDraft,
    RemoteDraft,
    Discard,
}

impl fmt::Display for PostEditChoice {
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

static POST_EDIT_CHOICES: [PostEditChoice; 5] = [
    PostEditChoice::Send,
    PostEditChoice::Edit,
    PostEditChoice::LocalDraft,
    PostEditChoice::RemoteDraft,
    PostEditChoice::Discard,
];

pub fn post_edit() -> Result<PostEditChoice> {
    let user_choice = prompt::item(
        "What would you like to do with this message?",
        &POST_EDIT_CHOICES,
        None,
    )?;

    Ok(user_choice.clone())
}
