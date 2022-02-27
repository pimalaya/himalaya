use anyhow::{Result, anyhow};

use dialoguer::Confirm;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wizard;

impl Wizard {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start() -> Result<String> {
        let wizard = Wizard::new();

        if !wizard.is_allowed_to_create_config()? {
            println!("Easter egg");
        }

        Ok("yeet".to_string())
    }

    fn is_allowed_to_create_config(&self) -> Result<bool> {

        println!("Oh! Looks like that you don't have a config file!");

        Confirm::new()
            .with_prompt("Should we (you and me, the config wizard) create the config file together? :)")
            .default(true)
            .interact_opt()?
            .ok_or(anyhow!("Cancelled wizard."))
    }
}
