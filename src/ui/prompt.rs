use std::io;

pub(crate) fn passwd(prompt: &str) -> io::Result<String> {
    inquire::Password::new(prompt)
        .with_custom_confirmation_message("Confirm password")
        .with_custom_confirmation_error_message("Passwords do not match, please try again.")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Interrupted,
                format!("failed to get password: {e}"),
            )
        })
}

pub(crate) fn secret(prompt: &str) -> io::Result<String> {
    inquire::Password::new(prompt)
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Interrupted,
                format!("failed to get secret: {e}"),
            )
        })
}
