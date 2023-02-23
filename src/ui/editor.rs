use anyhow::{Context, Result};
use himalaya_lib::{
    email::{local_draft_path, remove_local_draft},
    AccountConfig, Backend, CompilerBuilder, Flag, Flags, Sender, Tpl,
};
use log::debug;
use std::{env, fs, process::Command};

use crate::{
    printer::Printer,
    ui::choice::{self, PostEditChoice, PreEditChoice},
};

pub fn open_with_tpl(tpl: Tpl) -> Result<Tpl> {
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

    Ok(Tpl::from(content))
}

pub fn open_with_local_draft() -> Result<Tpl> {
    let path = local_draft_path();
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    open_with_tpl(Tpl::from(content))
}

pub fn edit_tpl_with_editor<P: Printer, B: Backend + ?Sized, S: Sender + ?Sized>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut B,
    sender: &mut S,
    mut tpl: Tpl,
) -> Result<()> {
    let draft = local_draft_path();
    if draft.exists() {
        loop {
            match choice::pre_edit() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => {
                        tpl = open_with_local_draft()?;
                        break;
                    }
                    PreEditChoice::Discard => {
                        tpl = open_with_tpl(tpl)?;
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
        tpl = open_with_tpl(tpl)?;
    }

    loop {
        match choice::post_edit() {
            Ok(PostEditChoice::Send) => {
                printer.print_log("Sending email…")?;
                let email = tpl.compile(
                    CompilerBuilder::default()
                        .some_pgp_sign_cmd(config.email_writing_sign_cmd.as_ref())
                        .some_pgp_encrypt_cmd(config.email_writing_encrypt_cmd.as_ref()),
                )?;
                sender.send(&email)?;
                let sent_folder = config.sent_folder_alias()?;
                printer.print_log(format!("Adding email to the {} folder…", sent_folder))?;
                backend.add_email(&sent_folder, &email, &Flags::from_iter([Flag::Seen]))?;
                remove_local_draft()?;
                printer.print("Done!")?;
                break;
            }
            Ok(PostEditChoice::Edit) => {
                tpl = open_with_tpl(tpl)?;
                continue;
            }
            Ok(PostEditChoice::LocalDraft) => {
                printer.print("Email successfully saved locally")?;
                break;
            }
            Ok(PostEditChoice::RemoteDraft) => {
                let draft_folder = config.folder_alias("drafts")?;
                let email = tpl.compile(
                    CompilerBuilder::default()
                        .some_pgp_sign_cmd(config.email_writing_sign_cmd.as_ref())
                        .some_pgp_encrypt_cmd(config.email_writing_encrypt_cmd.as_ref()),
                )?;
                backend.add_email(
                    &draft_folder,
                    &email,
                    &Flags::from_iter([Flag::Seen, Flag::Draft]),
                )?;
                remove_local_draft()?;
                printer.print(format!("Email successfully saved to {}", draft_folder))?;
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
