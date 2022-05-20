use anyhow::Result;
use himalaya::{
    backends::{Backend, ImapBackend, MaildirBackend},
    config::{AccountConfig, BackendConfig, DeserializedConfig, DEFAULT_INBOX_FOLDER},
    msg::msg_handlers,
};
use iui::controls::Button;
use iui::prelude::*;

use himalaya_gui::mail_list::ListPrinter;

fn main() -> Result<()> {
    let config = DeserializedConfig::from_opt_path(None)?;
    let (account_config, backend_config) =
        AccountConfig::from_config_and_opt_account_name(&config, None)?;

    let mut imap;
    let mut maildir;

    let backend: Box<&mut dyn Backend> = match backend_config {
        BackendConfig::Imap(ref imap_config) => {
            imap = ImapBackend::new(&account_config, imap_config);
            Box::new(&mut imap)
        }
        BackendConfig::Maildir(ref maildir_config) => {
            maildir = MaildirBackend::new(&account_config, maildir_config);
            Box::new(&mut maildir)
        }
    };

    let ui = UI::init().expect("Couldn't initialize UI library");
    let account_name = &account_config.name;
    let mut win = Window::new(
        &ui,
        &format!("Himalaya ({account_name:})"),
        1024,
        768,
        WindowType::NoMenubar,
    );

    let mut printer = ListPrinter::new(&ui);

    msg_handlers::list(
        None,
        None,
        1,
        DEFAULT_INBOX_FOLDER,
        &account_config,
        &mut printer,
        backend,
    )?;
    printer.draw(&ui);

    let mut quit_button = Button::new(&ui, "Quit");
    quit_button.on_clicked(&ui, {
        let ui = ui.clone();
        move |_| {
            ui.quit();
        }
    });

    // XXX: should refactor here
    printer
        .inner
        .append(&ui, quit_button, LayoutStrategy::Compact);

    win.set_child(&ui, printer.inner);
    win.show(&ui);
    ui.main();
    Ok(())
}
