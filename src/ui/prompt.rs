use dialoguer::Password;
use std::io;

use super::THEME;

pub(crate) fn passwd(prompt: &str) -> io::Result<String> {
    Password::with_theme(&*THEME)
        .with_prompt(prompt)
        .with_confirmation(
            "Confirm password",
            "Passwords do not match, please try again.",
        )
        .interact()
}

pub(crate) fn secret(prompt: &str) -> io::Result<String> {
    Password::with_theme(&*THEME)
        .with_prompt(prompt)
        .report(false)
        .interact()
}
