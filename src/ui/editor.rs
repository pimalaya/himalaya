use anyhow::{Context, Result};
use email::{
    account::AccountConfig,
    backend::Backend,
    email::{local_draft_path, remove_local_draft, Flag, Flags},
    sender::Sender,
};
use log::debug;
use mml::MmlCompiler;
use process::Cmd;
use std::{env, fs};

use crate::{
    printer::Printer,
    ui::choice::{self, PostEditChoice, PreEditChoice},
};

pub async fn open_with_tpl(tpl: String) -> Result<String> {
    let path = local_draft_path();

    debug!("create draft");
    fs::write(&path, tpl.as_bytes()).context(format!("cannot write local draft at {:?}", path))?;

    debug!("open editor");
    let editor = env::var("EDITOR").context("cannot get editor from env var")?;
    Cmd::from(format!("{editor} {}", &path.to_string_lossy()))
        .run()
        .await
        .context("cannot launch editor")?;

    debug!("read draft");
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;

    Ok(content)
}

pub async fn open_with_local_draft() -> Result<String> {
    let path = local_draft_path();
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    open_with_tpl(content).await
}

pub async fn edit_tpl_with_editor<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &mut dyn Backend,
    sender: &mut dyn Sender,
    mut tpl: String,
) -> Result<()> {
    let draft = local_draft_path();
    if draft.exists() {
        loop {
            match choice::pre_edit() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => {
                        tpl = open_with_local_draft().await?;
                        break;
                    }
                    PreEditChoice::Discard => {
                        tpl = open_with_tpl(tpl).await?;
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
        tpl = open_with_tpl(tpl).await?;
    }

    loop {
        match choice::post_edit() {
            Ok(PostEditChoice::Send) => {
                printer.print_log("Sending email…")?;
                let email = MmlCompiler::new()
                    .with_pgp(config.pgp.clone())
                    .compile(tpl)
                    .await?
                    .write_to_vec()?;
                sender.send(&email).await?;
                if config.email_sending_save_copy {
                    let sent_folder = config.sent_folder_alias()?;
                    printer.print_log(format!("Adding email to the {} folder…", sent_folder))?;
                    backend
                        .add_email(&sent_folder, &email, &Flags::from_iter([Flag::Seen]))
                        .await?;
                }
                remove_local_draft()?;
                printer.print("Done!")?;
                break;
            }
            Ok(PostEditChoice::Edit) => {
                tpl = open_with_tpl(tpl).await?;
                continue;
            }
            Ok(PostEditChoice::LocalDraft) => {
                printer.print("Email successfully saved locally")?;
                break;
            }
            Ok(PostEditChoice::RemoteDraft) => {
                let email = MmlCompiler::new()
                    .with_pgp(config.pgp.clone())
                    .compile(tpl)
                    .await?
                    .write_to_vec()?;
                backend
                    .add_email(
                        "drafts",
                        &email,
                        &Flags::from_iter([Flag::Seen, Flag::Draft]),
                    )
                    .await?;
                remove_local_draft()?;
                printer.print("Email successfully saved to drafts")?;
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
