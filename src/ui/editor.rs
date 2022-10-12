use anyhow::{Context, Result};
use himalaya_lib::{
    email::{local_draft_path, remove_local_draft, Email, TplOverride},
    AccountConfig, Backend, Sender,
};
use log::{debug, info};
use std::{env, fs, process::Command};

use crate::{
    printer::Printer,
    ui::choice::{self, PostEditChoice, PreEditChoice},
};

pub fn open_with_tpl(tpl: String) -> Result<String> {
    let path = local_draft_path();

    debug!("create draft");
    fs::write(&path, tpl.as_bytes()).context(format!("cannot write local draft at {:?}", path))?;

    debug!("open editor");
    Command::new(env::var("EDITOR").context(r#"cannot find "$EDITOR" env var"#)?)
        .arg(&path)
        .status()
        .context("cannot launch editor")?;

    debug!("read draft");
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;

    Ok(content)
}

pub fn open_with_draft() -> Result<String> {
    let path = local_draft_path();
    let tpl =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    open_with_tpl(tpl)
}

fn _edit_email_with_editor(
    email: &Email,
    tpl: TplOverride,
    config: &AccountConfig,
) -> Result<Email> {
    let tpl = email.to_tpl(tpl, config)?;
    let tpl = open_with_tpl(tpl)?;
    Email::from_tpl(&tpl).context("cannot parse email from template")
}

pub fn edit_email_with_editor<'a, P: Printer, B: Backend<'a> + ?Sized, S: Sender + ?Sized>(
    mut email: Email,
    tpl: TplOverride,
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
) -> Result<()> {
    info!("start editing with editor");

    let draft = local_draft_path();
    if draft.exists() {
        loop {
            match choice::pre_edit() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => {
                        let tpl = open_with_draft()?;
                        email.merge_with(Email::from_tpl(&tpl)?);
                        break;
                    }
                    PreEditChoice::Discard => {
                        email.merge_with(_edit_email_with_editor(&email, tpl.clone(), config)?);
                        break;
                    }
                    PreEditChoice::Quit => return Ok(()),
                },
                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            }
        }
    } else {
        email.merge_with(_edit_email_with_editor(&email, tpl.clone(), config)?);
    }

    loop {
        match choice::post_edit() {
            Ok(PostEditChoice::Send) => {
                printer.print_str("Sending email…")?;
                let sent_email: Vec<u8> = sender.send(&email)?;
                let sent_folder = config.folder_alias("sent")?;
                printer.print_str(format!("Adding email to the {:?} folder…", sent_folder))?;
                backend.email_add(&sent_folder, &sent_email, "seen")?;
                remove_local_draft()?;
                printer.print_struct("Done!")?;
                break;
            }
            Ok(PostEditChoice::Edit) => {
                email.merge_with(_edit_email_with_editor(&email, tpl.clone(), config)?);
                continue;
            }
            Ok(PostEditChoice::LocalDraft) => {
                printer.print_struct("Email successfully saved locally")?;
                break;
            }
            Ok(PostEditChoice::RemoteDraft) => {
                let tpl = email.to_tpl(TplOverride::default(), config)?;
                let draft_folder = config.folder_alias("drafts")?;
                backend.email_add(&draft_folder, tpl.as_bytes(), "seen draft")?;
                remove_local_draft()?;
                printer.print_struct(format!("Email successfully saved to {}", draft_folder))?;
                break;
            }
            Ok(PostEditChoice::Discard) => {
                remove_local_draft()?;
                break;
            }
            Err(err) => {
                println!("{}", err);
                continue;
            }
        }
    }

    Ok(())
}
