#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizardIO {

}

impl WizardIO {
    pub fn new() -> Self {
        Self
    }

    pub fn introduction(&self) {
        println!("Oops! Looks like that you don't have a config for himalaya!");
    }
}
