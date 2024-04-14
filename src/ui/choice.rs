use color_eyre::Result;
use dialoguer::Select;

use super::THEME;

#[derive(Clone, Debug)]
pub enum PreEditChoice {
    Edit,
    Discard,
    Quit,
}

impl ToString for PreEditChoice {
    fn to_string(&self) -> String {
        match self {
            Self::Edit => "Edit it".into(),
            Self::Discard => "Discard it".into(),
            Self::Quit => "Quit".into(),
        }
    }
}

pub fn pre_edit() -> Result<PreEditChoice> {
    let choices = [
        PreEditChoice::Edit,
        PreEditChoice::Discard,
        PreEditChoice::Quit,
    ];

    let choice_idx = Select::with_theme(&*THEME)
        .with_prompt("A draft was found, what would you like to do with it?")
        .items(&choices)
        .default(0)
        .interact()?;

    Ok(choices[choice_idx].clone())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PostEditChoice {
    Send,
    Edit,
    LocalDraft,
    RemoteDraft,
    Discard,
}

impl ToString for PostEditChoice {
    fn to_string(&self) -> String {
        match self {
            Self::Send => "Send it".into(),
            Self::Edit => "Edit it again".into(),
            Self::LocalDraft => "Save it as local draft".into(),
            Self::RemoteDraft => "Save it as remote draft".into(),
            Self::Discard => "Discard it".into(),
        }
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

    let choice_idx = Select::with_theme(&*THEME)
        .with_prompt("What would you like to do with this message?")
        .items(&choices)
        .default(0)
        .interact()?;

    Ok(choices[choice_idx].clone())
}
