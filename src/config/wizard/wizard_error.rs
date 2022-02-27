#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum WizardError {
    #[error("User denied wizard :(")]
    Unallowed,
}
